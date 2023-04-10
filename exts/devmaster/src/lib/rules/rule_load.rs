// Copyright (c) 2022 Huawei Technologies Co.,Ltd. All rights reserved.
//
// sysMaster is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan
// PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY
// KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

//! load rules
//!

use super::*;
use crate::error::{Error, Result};
use lazy_static::lazy_static;
use regex::Regex;
use std::fs::File;
use std::io::{BufRead, BufReader};

/// directories for searching rule files
const DEFAULT_RULES_DIRS: [&str; 4] = [
    "/etc/udev/rules.d",
    "/run/udev/rules.d",
    "/usr/local/lib/udev/rules.d",
    "/usr/lib/udev/rules.d",
];

impl Rules {
    /// enumerate all .rules file under the directories and generate the rules object
    pub fn new(dirs: &[&str]) -> Rules {
        let mut rules = Rules {
            files: None,
            current_file: None,
        };

        for dir in dirs {
            let dir_path = std::path::Path::new(dir);
            if !dir_path.exists() || !dir_path.is_dir() {
                log::warn!("Rule directory {} is invalid.", dir);
                continue;
            }

            let mut files: Vec<String> = vec![];

            for file in dir_path.read_dir().unwrap() {
                if file.is_err() {
                    log::warn!(
                        "Failed to read file under {}: {:?}.",
                        dir,
                        file.unwrap_err()
                    );
                    continue;
                }
                let buf = file.unwrap().path();
                let de = buf.as_os_str().to_str().unwrap();
                if !de.ends_with(".rules") {
                    log::warn!("Ignore file not ending with rules: {}", de);
                    continue;
                }
                files.push(de.to_string());
            }

            files.sort();

            for f in files {
                rules.parse_file(f);
            }
        }

        rules
    }

    pub(crate) fn parse_file(&mut self, file_name: String) {
        log::debug!("{}", file_name);
        let file = RuleFile::new(file_name);

        self.add_file(file);
    }

    /// add the rule file into
    pub(crate) fn add_file(&mut self, file: Rc<RefCell<RuleFile>>) {
        if self.current_file.is_none() {
            self.files = Some(file.clone());
        } else {
            self.current_file.as_mut().unwrap().borrow_mut().next = Some(file.clone());
            file.borrow_mut().prev = self.current_file.clone();
        }

        self.current_file = Some(file);
    }
}

impl Default for Rules {
    fn default() -> Self {
        Self::new(&DEFAULT_RULES_DIRS)
    }
}

impl RuleFile {
    pub(crate) fn new(file_name: String) -> Rc<RefCell<RuleFile>> {
        let rule_file = Rc::<RefCell<RuleFile>>::new(RefCell::<RuleFile>::new(RuleFile {
            file_name,
            lines: None,
            current_line: None,
            prev: None,
            next: None,
        }));

        rule_file.borrow_mut().parse_lines(rule_file.clone());

        rule_file
    }

    /// parse and load all available lines in the rule file
    pub(crate) fn parse_lines(&mut self, self_ptr: Rc<RefCell<RuleFile>>) {
        let file = File::open(&self.file_name).unwrap();
        let reader = BufReader::new(file);

        let mut full_line = String::new();
        let mut offset = 0;
        for (line_number, line) in reader.lines().enumerate() {
            if let Err(e) = line {
                log::warn!("Read line failed in {} : {:?}", self.file_name, e);
                continue;
            }
            let line = line.unwrap();
            let line = line.trim_start().trim_end();
            if line.starts_with('#') || line.is_empty() {
                continue;
            }

            if line.ends_with('\\') {
                full_line.push_str(line.strip_suffix('\\').unwrap());
                offset += 1;
            } else {
                full_line.push_str(line);
                let line = RuleLine::new(
                    full_line.to_string(),
                    (line_number - offset) as u32,
                    self_ptr.clone(),
                )
                .unwrap();
                self.add_line(line);
                full_line.clear();
                offset = 0;
            }
        }
    }

    /// add rule line to the rule file object
    pub(crate) fn add_line(&mut self, line: Rc<RefCell<RuleLine>>) {
        if self.lines.is_none() {
            self.lines = Some(line.clone());
        } else {
            self.current_line.as_mut().unwrap().borrow_mut().next = Some(line.clone());
            line.borrow_mut().prev = self.current_line.clone();
        }

        self.current_line = Some(line);
    }
}

impl RuleLine {
    /// create a rule line object
    pub fn new(
        line: String,
        line_number: u32,
        file: Rc<RefCell<RuleFile>>,
    ) -> Result<Rc<RefCell<RuleLine>>> {
        lazy_static! {
            static ref RE_LINE: Regex =
                Regex::new("((?P<key>[^={+\\-!:\0\\s]+)(\\{(?P<attr>[^\\{\\}]+)\\})?\\s*(?P<op>[!:+-=]?=)\\s*\"(?P<value>[^\"]+)\"\\s*,?\\s*)+").unwrap();
            static ref RE_TOKEN: Regex =
                Regex::new("(?P<key>[^={+\\-!:\0\\s]+)(\\{(?P<attr>[^\\{\\}]+)\\})?\\s*(?P<op>[!:+-=]?=)\\s*\"(?P<value>[^\"]+)\"\\s*,?\\s*").unwrap();
        }

        let mut rule_line = RuleLine {
            line: line.clone(),
            line_number,

            tokens: None,
            current_token: None,

            file: Rc::downgrade(&file),

            next: None,
            prev: None,
        };

        if !RE_LINE.is_match(&line) {
            return Err(Error::RulesLoaderError {
                msg: "Invalid rule line",
            });
        }

        for token in RE_TOKEN.captures_iter(&line) {
            // through previous check through regular expression,
            // key, op, value must not be none
            // attr may be none in case of specific rule tokens
            let key = token.name("key").map(|k| k.as_str().to_string()).unwrap();
            let attr = token.name("attr").map(|a| a.as_str().to_string());
            let op = token.name("op").map(|o| o.as_str().to_string()).unwrap();
            let value = token.name("value").map(|v| v.as_str().to_string()).unwrap();
            log::debug!(
                "
{}
key = {}
attr = {}
op = {}
value = {}",
                line,
                key,
                attr.clone().unwrap_or_default(),
                op,
                value,
            );
            let rule_token = RuleToken::new(key, attr, op, value)?;
            rule_line.add_token(rule_token);
        }

        Ok(Rc::<RefCell<RuleLine>>::new(RefCell::<RuleLine>::new(
            rule_line,
        )))
    }

    /// add token into rule line
    pub(crate) fn add_token(&mut self, rule_token: RuleToken) {
        let rule_token = Rc::<RefCell<RuleToken>>::new(RefCell::<RuleToken>::new(rule_token));
        if self.tokens.is_none() {
            self.tokens = Some(rule_token.clone());
        } else {
            self.current_token.as_mut().unwrap().borrow_mut().next = Some(rule_token.clone());
            rule_token.borrow_mut().prev = self.current_token.clone();
        }

        self.current_token = Some(rule_token);
    }
}

impl RuleToken {
    /// create rule token object
    pub fn new(key: String, attr: Option<String>, op: String, value: String) -> Result<RuleToken> {
        let op = op.parse::<OperatorType>()?;
        let op_is_match = [OperatorType::Match, OperatorType::Nomatch].contains(&op);
        match key.as_str() {
            "ACTION" => {
                if attr.is_some() {
                    return Err(Error::RulesLoaderError {
                        msg: "key ACTION can not carry attribute.",
                    });
                }
                if !op_is_match {
                    return Err(Error::RulesLoaderError {
                        msg: "key ACTION can only take match or unmatch operator.",
                    });
                }

                Ok(RuleToken {
                    r#type: TokenType::MatchAction,
                    op,
                    attr: None,
                    value,
                    prev: None,
                    next: None,
                })
            }
            "SYMLINK" => {
                if attr.is_some() {
                    return Err(Error::RulesLoaderError {
                        msg: "key SYMLINK can not carry attribute.",
                    });
                }

                if !op_is_match {
                    // crate::rules::rule_utils::check_value_format_and_warn();
                    Ok(RuleToken {
                        r#type: TokenType::AssignDevlink,
                        op,
                        attr: None,
                        value,
                        prev: None,
                        next: None,
                    })
                } else {
                    Ok(RuleToken {
                        r#type: TokenType::MatchDevlink,
                        op,
                        attr: None,
                        value,
                        prev: None,
                        next: None,
                    })
                }
            }
            _ => {
                todo!("Unimplemented key")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use basic::logger::init_log_to_console;
    use log::LevelFilter;

    use super::*;
    use std::fs;

    #[test]
    fn test_rules_new() {
        init_log_to_console("test_rules_new", LevelFilter::Debug);

        assert!(fs::create_dir("test_rules_new_1").is_ok());
        assert!(fs::write(
            "test_rules_new_1/test1.rules",
            "ACTION == \"change\", SYMLINK += \"test1\"
ACTION == \"change\", SYMLINK += \"test11\", \\
SYMLINK += \"test111\"
ACTION == \"change\", SYMLINK += \"test1111\", \\
SYMLINK += \"test11111\", \\
SYMLINK += \"test111111\"",
        )
        .is_ok());
        assert!(fs::create_dir("test_rules_new_2").is_ok());
        assert!(fs::write(
            "test_rules_new_2/test1.rules",
            "ACTION == \"change\", SYMLINK += \"test2\"
ACTION == \"change\", SYMLINK += \"test22\", \\
SYMLINK += \"test222\"
ACTION == \"change\", SYMLINK += \"test2222\", \\
SYMLINK += \"test22222\", \\
SYMLINK += \"test222222\"",
        )
        .is_ok());
        let rules = Rules::new(&["test_rules_new_1", "test_rules_new_2"]);
        println!("{}", rules);
        assert!(fs::remove_dir_all("test_rules_new_1").is_ok());
        assert!(fs::remove_dir_all("test_rules_new_2").is_ok());
    }

    #[test]
    fn test_rules_file() {
        fs::write(
            "test_rules_file.rules",
            "ACTION == \"change\", SYMLINK+=\"test\"\nACTION != \"change\"\n",
        )
        .unwrap();
        RuleFile::new("test_rules_file.rules".to_string());
        fs::remove_file("test_rules_file.rules").unwrap();
    }

    #[test]
    fn test_rules_token() {
        assert!(RuleToken::new(
            "ACTION".to_string(),
            None,
            "==".to_string(),
            "add".to_string()
        )
        .is_ok());

        assert!(RuleToken::new(
            "ACTION".to_string(),
            None,
            "!=".to_string(),
            "add".to_string()
        )
        .is_ok());

        assert!(RuleToken::new(
            "ACTION".to_string(),
            None,
            "*=".to_string(),
            "add".to_string()
        )
        .is_err());

        assert!(RuleToken::new(
            "ACTION".to_string(),
            Some("whatever".to_string()),
            "==".to_string(),
            "add".to_string()
        )
        .is_err());
    }
}
