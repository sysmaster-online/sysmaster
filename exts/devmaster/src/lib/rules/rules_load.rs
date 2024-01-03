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
use crate::builtin::BuiltinCommand;
use crate::error::{Error, Result};
use crate::utils::commons::*;
use basic::parse::parse_mode;
use basic::unistd::{parse_gid, parse_uid};
use basic::IN_SET;
use nix::unistd::{Group, User};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

trait Location {
    fn location(&self, context: &(u32, String, String)) -> String;
}

impl<T: Display> Location for T {
    fn location(&self, context: &(u32, String, String)) -> String {
        format!("{}:{} {} {}", context.1, context.0, context.2, self)
    }
}

impl Rules {
    /// load all rules under specified directories
    pub(crate) fn load_rules(
        dirs: Vec<String>,
        resolve_name_time: ResolveNameTime,
    ) -> Arc<RwLock<Rules>> {
        let rules = Arc::new(RwLock::new(Self::new(dirs, resolve_name_time)));

        Self::parse_rules(rules.clone());

        rules
    }

    /// enumerate all .rules file under the directories and generate the rules object
    pub(crate) fn new(dirs: Vec<String>, resolve_name_time: ResolveNameTime) -> Rules {
        Rules {
            files: Arc::new(RwLock::new(None)),
            files_tail: Arc::new(RwLock::new(None)),
            dirs,
            resolve_name_time,
            users: HashMap::new(),
            groups: HashMap::new(),
        }
    }

    /// enumerate and parse all rule files under rule directories
    pub(crate) fn parse_rules(rules: Arc<RwLock<Rules>>) {
        let dirs = rules.as_ref().read().unwrap().dirs.clone();

        let mut files: Vec<PathBuf> = vec![];
        for dir in dirs {
            let dir_path = std::path::Path::new(&dir);
            if !dir_path.exists() || !dir_path.is_dir() {
                log::warn!("Rule directory {} is invalid.", dir);
                continue;
            }

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
                if !buf
                    .file_name()
                    .unwrap_or_default()
                    .to_str()
                    .unwrap_or_default()
                    .ends_with(".rules")
                {
                    log::warn!("Ignore file not ending with .rules: {:?}", buf);
                    continue;
                }
                files.push(buf);
            }
        }

        files.sort_by(|a, b| {
            a.file_name()
                .unwrap_or_default()
                .cmp(b.file_name().unwrap_or_default())
        });

        for f in files {
            Self::parse_file(rules.clone(), f.to_str().unwrap().to_string());
        }
    }

    /// parse a single rule file, and insert it into rules
    pub(crate) fn parse_file(rules: Arc<RwLock<Rules>>, file_name: String) {
        log::debug!("Parsing rule file: {}", file_name);
        let file = RuleFile::load_file(file_name, rules.clone());
        Self::add_file(rules, file);
    }

    /// push the rule file into the tail of linked list
    pub(crate) fn add_file(rules: Arc<RwLock<Rules>>, file: Arc<RwLock<Option<RuleFile>>>) {
        if rules.read().unwrap().files_tail.read().unwrap().is_none() {
            rules.write().unwrap().files = file.clone();
        } else {
            rules
                .read()
                .unwrap()
                .files_tail
                .write()
                .unwrap()
                .as_mut()
                .unwrap()
                .next = file.clone();
            file.write().unwrap().as_mut().unwrap().prev =
                rules.as_ref().read().unwrap().files_tail.clone();
        }

        rules.write().unwrap().files_tail = file;
    }

    /// if the user name has valid credential, insert it to rules
    pub(crate) fn resolve_user(&mut self, username: &str) -> Result<User> {
        if let Some(user) = self.users.get(username) {
            return Ok(user.clone());
        }

        match User::from_name(username) {
            Ok(user) => match user {
                Some(u) => {
                    self.users.insert(username.to_string(), u.clone());
                    Ok(u)
                }
                None => Err(Error::RulesLoadError {
                    msg: format!("The user name {} has no credential.", username),
                }),
            },
            Err(e) => Err(Error::RulesLoadError {
                msg: format!("Failed to resolve user name {}: {}", username, e),
            }),
        }
    }

    /// if the group name has valid credential, insert it to rules
    pub(crate) fn resolve_group(&mut self, groupname: &str) -> Result<Group> {
        if let Some(group) = self.groups.get(groupname) {
            return Ok(group.clone());
        }

        match Group::from_name(groupname) {
            Ok(group) => match group {
                Some(g) => {
                    self.groups.insert(groupname.to_string(), g.clone());
                    Ok(g)
                }
                None => Err(Error::RulesLoadError {
                    msg: format!("The group name {} has no credential.", groupname),
                }),
            },
            Err(e) => Err(Error::RulesLoadError {
                msg: format!("Failed to resolve group name {}: {}", groupname, e),
            }),
        }
    }
}

impl RuleFile {
    /// rule file object is always stored in heap
    /// the pointer to rules is used for specific tokens, e.g., 'GOTO' and 'LABEL',
    /// which will directly modify some fields in rules
    pub(crate) fn load_file(
        file_name: String,
        rules: Arc<RwLock<Rules>>,
    ) -> Arc<RwLock<Option<RuleFile>>> {
        let rule_file = Arc::new(RwLock::new(Some(RuleFile::new(file_name))));

        /*
         * It is right that RuleFile will change during parsing rules lines,
         * but the fine-grained write lock guard should fall onto the RuleLine
         * object rather than the RuleFile itself. Otherwise there will be
         * deadlock when the RuleFile is accessed to read during parsing rule
         * lines.
         */
        RuleFile::parse_lines(rule_file.clone(), rules);

        rule_file
    }

    /// create a initial rule file object
    pub(crate) fn new(file_name: String) -> RuleFile {
        RuleFile {
            file_name,
            lines: Arc::new(RwLock::new(None)),
            lines_tail: Arc::new(RwLock::new(None)),
            prev: Arc::new(RwLock::new(None)),
            next: Arc::new(RwLock::new(None)),
        }
    }

    /// parse and load all available lines in the rule file
    /// the pointer to rules is used for specific tokens, e.g., 'GOTO' and 'LABEL',
    /// which will directly modify some fields in rules
    pub(crate) fn parse_lines(rule_file: Arc<RwLock<Option<RuleFile>>>, rules: Arc<RwLock<Rules>>) {
        let file_name = rule_file.read().unwrap().as_ref().unwrap().get_file_name();
        let file = File::open(&file_name).unwrap();
        let reader = BufReader::new(file);

        let mut full_line = String::new();
        let mut offset = 0;
        for (line_number, line) in reader.lines().enumerate() {
            if let Err(e) = line {
                log::warn!("Read line failed in {} : {:?}", file_name, e);
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
                let line = match RuleLine::load_line(
                    &full_line,
                    (line_number + 1 - offset) as u32,
                    rule_file.clone(),
                    rules.clone(),
                ) {
                    Ok(line) => line,
                    Err(e) => {
                        log::error!("{}:{} {}", &file_name, line_number, &e);
                        panic!();
                    }
                };
                rule_file.write().unwrap().as_mut().unwrap().add_line(line);
                full_line.clear();
                offset = 0;
            }
        }

        rule_file.write().unwrap().as_mut().unwrap().resolve_goto();
    }

    /// push the rule line to the tail of linked list
    pub(crate) fn add_line(&mut self, line: Arc<RwLock<Option<RuleLine>>>) {
        if self.lines.read().unwrap().is_none() {
            self.lines = line.clone();
        } else {
            self.lines_tail.write().unwrap().as_mut().unwrap().next = line.clone();
            line.write().unwrap().as_mut().unwrap().prev = self.lines_tail.clone();
        }

        self.lines_tail = line;
    }

    /// bind goto with label lines
    pub(crate) fn resolve_goto(&mut self) {
        let mut labels: HashMap<String, Arc<RwLock<Option<RuleLine>>>> = HashMap::new();

        for line in self.iter() {
            if line
                .read()
                .unwrap()
                .as_ref()
                .unwrap()
                .r#type
                .intersects(RuleLineType::HAS_LABEL)
            {
                labels.insert(
                    line.read().unwrap().as_ref().unwrap().get_label().unwrap(),
                    line.clone(),
                );
            }
        }

        for line in self.iter() {
            if !line
                .read()
                .unwrap()
                .as_ref()
                .unwrap()
                .r#type
                .intersects(RuleLineType::HAS_GOTO)
            {
                continue;
            }

            let label = line
                .read()
                .unwrap()
                .as_ref()
                .unwrap()
                .get_goto_label()
                .unwrap();

            line.write().unwrap().as_mut().unwrap().goto_line = labels.get(&label).unwrap().clone();
        }
    }
}

impl RuleLine {
    /// load a rule line
    pub(crate) fn new(
        line_content: String,
        line_number: u32,
        file: Arc<RwLock<Option<RuleFile>>>,
    ) -> RuleLine {
        RuleLine {
            line_content,
            line_number,

            r#type: RuleLineType::INITIAL,

            label: None,
            goto_label: None,
            goto_line: Arc::new(RwLock::new(None)),

            tokens: Arc::new(RwLock::new(None)),
            tokens_tail: Arc::new(RwLock::new(None)),

            rule_file: Arc::downgrade(&file),

            next: Arc::new(RwLock::new(None)),
            prev: Arc::new(RwLock::new(None)),
        }
    }

    /// create a rule line object
    /// Note: file is locked previously.
    pub(crate) fn load_line(
        line: &str,
        line_number: u32,
        file: Arc<RwLock<Option<RuleFile>>>,
        rules: Arc<RwLock<Rules>>,
    ) -> Result<Arc<RwLock<Option<RuleLine>>>> {
        debug_assert!(file.read().unwrap().is_some());

        let rule_line = Arc::new(RwLock::new(Some(RuleLine::new(
            line.to_string(),
            line_number,
            file,
        ))));

        #[derive(Debug)]
        enum State {
            Pre,
            Key,
            Attribute,
            PreOp,
            Op,
            PostOp,
            Value,
            PostValue,
        }

        let mut state = State::Pre;
        let mut key = "".to_string();
        let mut attribute = "".to_string();
        let mut op = "".to_string();
        let mut value = "".to_string();

        for (idx, ch) in line.chars().enumerate() {
            match state {
                State::Pre => {
                    if ch.is_ascii_whitespace() || ch == ',' {
                        continue;
                    }

                    if ch.is_ascii_uppercase() {
                        key.push(ch);
                        state = State::Key;
                    } else {
                        return Err(Error::RulesLoadError {
                            msg: format!("Invalid rule line: {} {} {:?}", line, idx, state),
                        });
                    }
                }
                State::Key => {
                    if ch.is_ascii_uppercase() {
                        key.push(ch);
                        state = State::Key;
                    } else if ch.is_ascii_whitespace() {
                        state = State::PreOp;
                    } else if ch == '{' {
                        state = State::Attribute;
                    } else if IN_SET!(ch, '!', '+', '-', ':', '=') {
                        op.push(ch);
                        state = State::Op;
                    } else {
                        return Err(Error::RulesLoadError {
                            msg: format!("Invalid rule line: {} {} {:?}", line, idx, state),
                        });
                    }
                }
                State::Attribute => {
                    if ch == '}' {
                        state = State::PreOp;
                        continue;
                    }

                    if ch.is_ascii_alphanumeric() || IN_SET!(ch, '$', '%', '*', '.', '/', '_') {
                        attribute.push(ch);
                    } else {
                        return Err(Error::RulesLoadError {
                            msg: format!("Invalid rule line: {} {} {:?}", line, idx, state),
                        });
                    }
                }
                State::PreOp => {
                    if ch.is_ascii_whitespace() {
                        continue;
                    }

                    if IN_SET!(ch, '!', '+', '-', ':', '=') {
                        op.push(ch);
                        state = State::Op;
                    } else {
                        return Err(Error::RulesLoadError {
                            msg: format!("Invalid rule line: {} {} {:?}", line, idx, state),
                        });
                    }
                }
                State::Op => {
                    if ch == '=' {
                        op.push(ch);
                        state = State::PostOp;
                    } else if ch.is_ascii_whitespace() {
                        state = State::PostOp;
                    } else if ch == '"' {
                        state = State::Value;
                    } else {
                        return Err(Error::RulesLoadError {
                            msg: format!("Invalid rule line: {} {} {:?}", line, idx, state),
                        });
                    }
                }
                State::PostOp => {
                    if ch.is_ascii_whitespace() {
                        continue;
                    } else if ch == '"' {
                        state = State::Value;
                    } else {
                        return Err(Error::RulesLoadError {
                            msg: format!("Invalid rule line: {} {} {:?}", line, idx, state),
                        });
                    }
                }
                State::Value => {
                    if ch == '"' {
                        state = State::PostValue;

                        let attr = if attribute.is_empty() {
                            None
                        } else {
                            Some(attribute.clone())
                        };

                        // if the token is 'GOTO' or 'LABEL', parse_token will return a IgnoreError
                        // the following tokens in this line, if any, will be skipped
                        let rule_token = RuleToken::parse_token(
                            key.clone(),
                            attr,
                            op.clone(),
                            value.clone(),
                            rules.clone(),
                            rule_line.clone(),
                        )?;
                        match rule_token.r#type {
                            TokenType::Goto => {
                                rule_line.write().unwrap().as_mut().unwrap().goto_label =
                                    Some(rule_token.value.clone());
                                rule_line.write().unwrap().as_mut().unwrap().r#type |=
                                    RuleLineType::HAS_GOTO;
                            }
                            TokenType::Label => {
                                rule_line.write().unwrap().as_mut().unwrap().label =
                                    Some(rule_token.value.clone());
                                rule_line.write().unwrap().as_mut().unwrap().r#type |=
                                    RuleLineType::HAS_LABEL;
                            }
                            TokenType::AssignName => {
                                rule_line.write().unwrap().as_mut().unwrap().r#type |=
                                    RuleLineType::HAS_NAME;
                            }

                            t => {
                                if [
                                    TokenType::AssignDevlink,
                                    TokenType::AssignOwner,
                                    TokenType::AssignGroup,
                                    TokenType::AssignMode,
                                    TokenType::AssignOwnerId,
                                    TokenType::AssignGroupId,
                                    TokenType::AssignModeId,
                                ]
                                .contains(&t)
                                {
                                    rule_line.write().unwrap().as_mut().unwrap().r#type |=
                                        RuleLineType::HAS_DEVLINK;
                                } else if TokenType::AssignOptionsStaticNode == t {
                                    rule_line.write().unwrap().as_mut().unwrap().r#type |=
                                        RuleLineType::HAS_STATIC_NODE;
                                } else if t >= TokenType::AssignOptionsStringEscapeNone
                                    || [
                                        TokenType::MatchProgram,
                                        TokenType::MatchImportFile,
                                        TokenType::MatchImportProgram,
                                        TokenType::MatchImportBuiltin,
                                        TokenType::MatchImportDb,
                                        TokenType::MatchImportCmdline,
                                        TokenType::MatchImportParent,
                                    ]
                                    .contains(&t)
                                {
                                    rule_line.write().unwrap().as_mut().unwrap().r#type |=
                                        RuleLineType::UPDATE_SOMETHING;
                                }
                            }
                        }
                        rule_line
                            .write()
                            .unwrap()
                            .as_mut()
                            .unwrap()
                            .add_token(rule_token);
                    } else {
                        value.push(ch);
                    }
                }
                State::PostValue => {
                    if ch.is_ascii_whitespace() || ch == ',' {
                        state = State::Pre;
                        key.clear();
                        attribute.clear();
                        op.clear();
                        value.clear();
                    } else {
                        return Err(Error::RulesLoadError {
                            msg: format!("Invalid rule line: {} {} {:?}", line, idx, state),
                        });
                    }
                }
            }
        }

        Ok(rule_line)
    }

    /// push the rule token to the tail of linked list
    pub(crate) fn add_token(&mut self, rule_token: RuleToken) {
        let rule_token = Arc::new(RwLock::new(Some(rule_token)));
        if self.tokens.read().unwrap().is_none() {
            self.tokens = rule_token.clone();
        } else {
            self.tokens_tail.write().unwrap().as_mut().unwrap().next = rule_token.clone();
            rule_token.write().unwrap().as_mut().unwrap().prev = self.tokens_tail.clone();
        }

        self.tokens_tail = rule_token;
    }
}

impl RuleToken {
    /// create a rule token
    pub(crate) fn new(
        r#type: TokenType,
        op: OperatorType,
        attr: Option<String>,
        value: String,
        rule_line: Arc<RwLock<Option<RuleLine>>>,
    ) -> Result<RuleToken> {
        let mut attr_subst_type = SubstituteType::Invalid;

        if matches!(r#type, TokenType::MatchAttr | TokenType::MatchParentsAttr) {
            attr_subst_type = match attr.clone().unwrap_or_default().parse::<SubstituteType>() {
                Ok(t) => t,
                Err(_) => {
                    return Err(Error::RulesLoadError {
                        msg: "Failed to parse the subsittution type of attribute.".to_string(),
                    });
                }
            }
        }

        Ok(RuleToken {
            r#type,
            op,
            attr_subst_type,
            attr,
            value,
            prev: Arc::new(RwLock::new(None)),
            next: Arc::new(RwLock::new(None)),
            rule_line: Arc::downgrade(&rule_line),
        })
    }

    /// parse strings into a rule token
    pub fn parse_token(
        key: String,
        attr: Option<String>,
        op_str: String,
        value: String,
        rules: Arc<RwLock<Rules>>,
        rule_line: Arc<RwLock<Option<RuleLine>>>,
    ) -> Result<RuleToken> {
        let mut op = op_str.parse::<OperatorType>()?;
        let op_is_match = [OperatorType::Match, OperatorType::Nomatch].contains(&op);
        let line_number = rule_line.read().unwrap().as_ref().unwrap().line_number;
        let rule_file_name = rule_line.read().unwrap().as_ref().unwrap().get_file_name();
        let rule_token_content = format!(
            "{}{}{}{}",
            key,
            attr.as_ref()
                .map(|s| format!("{{{}}}", s))
                .unwrap_or_default(),
            op_str,
            value
        );
        let context = (line_number, rule_file_name, rule_token_content);
        match key.as_str() {
            "ACTION" => {
                if attr.is_some() {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'ACTION' can not carry attribute.".location(&context),
                    });
                }
                if !op_is_match {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'ACTION' can only take match operator.".location(&context),
                    });
                }

                Ok(RuleToken::new(
                    TokenType::MatchAction,
                    op,
                    None,
                    value,
                    rule_line,
                )?)
            }
            "DEVPATH" => {
                if attr.is_some() {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'DEVPATH' can not carry attribute.".location(&context),
                    });
                }
                if !op_is_match {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'DEVPATH' can only take match operator.".location(&context),
                    });
                }

                Ok(RuleToken::new(
                    TokenType::MatchDevpath,
                    op,
                    None,
                    value,
                    rule_line,
                )?)
            }
            "KERNEL" => {
                if attr.is_some() {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'KERNEL' can not carry attribute.".location(&context),
                    });
                }
                if !op_is_match {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'KERNEL' can only take match operator.".location(&context),
                    });
                }

                Ok(RuleToken::new(
                    TokenType::MatchKernel,
                    op,
                    attr,
                    value,
                    rule_line,
                )?)
            }
            "SYMLINK" => {
                if attr.is_some() {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'SYMLINK' can not carry attribute.".location(&context),
                    });
                }

                if !op_is_match {
                    if let Err(e) = check_value_format(key.as_str(), value.as_str(), false) {
                        log::warn!("{}", e.location(&context));
                    }
                    Ok(RuleToken::new(
                        TokenType::AssignDevlink,
                        op,
                        None,
                        value,
                        rule_line,
                    )?)
                } else {
                    Ok(RuleToken::new(
                        TokenType::MatchDevlink,
                        op,
                        None,
                        value,
                        rule_line,
                    )?)
                }
            }
            "NAME" => {
                if attr.is_some() {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'NAME' can not carry attribute.".location(&context),
                    });
                }
                if op == OperatorType::Remove {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'NAME' can not take remove operator.".location(&context),
                    });
                }

                if op == OperatorType::Add {
                    log::warn!("{}", "Key 'NAME' can only take '==', '!=', '=', or ':=' operator, change '+=' to '=' implicitly.".location(&context));
                    op = OperatorType::Assign;
                }

                if !op_is_match {
                    if value.eq("%k") {
                        return Err(Error::RulesLoadError {
                            msg: "Ignore token NAME=\"%k\", as it takes no effect."
                                .location(&context),
                        });
                    }
                    if value.is_empty() {
                        return Err(Error::RulesLoadError {
                            msg: "Ignore token NAME=\"\", as it takes no effect."
                                .location(&context),
                        });
                    }
                    if let Err(e) = check_value_format(key.as_str(), value.as_str(), false) {
                        log::warn!("{}", e.location(&context));
                    }

                    Ok(RuleToken::new(
                        TokenType::AssignName,
                        op,
                        None,
                        value,
                        rule_line,
                    )?)
                } else {
                    Ok(RuleToken::new(
                        TokenType::MatchName,
                        op,
                        None,
                        value,
                        rule_line,
                    )?)
                }
            }
            "ENV" => {
                if attr.is_none() {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'ENV' must have attribute.".location(&context),
                    });
                }
                if op == OperatorType::Remove {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'ENV' can not take '-=' operator.".location(&context),
                    });
                }
                if op == OperatorType::AssignFinal {
                    log::warn!(
                        "{}",
                        "Key 'ENV' can not take ':=' operator, change ':=' to '=' implicitly."
                            .location(&context)
                    );
                    op = OperatorType::Assign;
                }

                if !op_is_match {
                    if matches!(
                        attr.as_ref().unwrap().as_str(),
                        "ACTION"
                            | "DEVLINKS"
                            | "DEVNAME"
                            | "DEVTYPE"
                            | "DRIVER"
                            | "IFINDEX"
                            | "MAJOR"
                            | "MINOR"
                            | "SEQNUM"
                            | "SUBSYSTEM"
                            | "TAGS"
                    ) {
                        return Err(Error::RulesLoadError {
                            msg: format!(
                                "Key 'ENV' has invalid attribute. '{}' can not be set.",
                                attr.as_ref().unwrap()
                            )
                            .location(&context),
                        });
                    }

                    if let Err(e) = check_value_format(key.as_str(), value.as_str(), false) {
                        log::warn!("{}", e);
                    }

                    Ok(RuleToken::new(
                        TokenType::AssignEnv,
                        op,
                        attr,
                        value,
                        rule_line,
                    )?)
                } else {
                    Ok(RuleToken::new(
                        TokenType::MatchEnv,
                        op,
                        attr,
                        value,
                        rule_line,
                    )?)
                }
            }
            "CONST" => {
                if attr.is_none() || !matches!(attr.as_ref().unwrap().as_str(), "arch" | "virt") {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'CONST' has invalid attribute.".location(&context),
                    });
                }

                if !op_is_match {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'CONST' must take match operator.".location(&context),
                    });
                }

                Ok(RuleToken::new(
                    TokenType::MatchConst,
                    op,
                    attr,
                    value,
                    rule_line,
                )?)
            }
            "TAG" => {
                if attr.is_some() {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'TAG' can not have attribute.".location(&context),
                    });
                }

                if op == OperatorType::AssignFinal {
                    log::warn!(
                        "{}",
                        "Key 'TAG' can not take ':=' operator, change ':=' to '=' implicitly."
                            .location(&context)
                    );
                    op = OperatorType::Assign;
                }

                if !op_is_match {
                    if let Err(e) = check_value_format(key.as_str(), value.as_str(), true) {
                        log::warn!("{}", e);
                    }

                    Ok(RuleToken::new(
                        TokenType::AssignTag,
                        op,
                        None,
                        value,
                        rule_line,
                    )?)
                } else {
                    Ok(RuleToken::new(
                        TokenType::MatchTag,
                        op,
                        None,
                        value,
                        rule_line,
                    )?)
                }
            }
            "SUBSYSTEM" => {
                if attr.is_some() {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'SUBSYSTEM' can not have attribute.".location(&context),
                    });
                }

                if !op_is_match {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'SUBSYSTEM' must take match operator.".location(&context),
                    });
                }

                if matches!(value.as_str(), "bus" | "class") {
                    log::warn!(
                        "{}",
                        "The value of key 'SUBSYSTEM' must be specified as 'subsystem'"
                            .location(&context)
                    );
                }

                Ok(RuleToken::new(
                    TokenType::MatchSubsystem,
                    op,
                    None,
                    value,
                    rule_line,
                )?)
            }
            "DRIVER" => {
                if attr.is_some() {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'DRIVER' can not have attribute.".location(&context),
                    });
                }

                if !op_is_match {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'DRIVER' must take match operator".location(&context),
                    });
                }

                Ok(RuleToken::new(
                    TokenType::MatchDriver,
                    op,
                    None,
                    value,
                    rule_line,
                )?)
            }
            "ATTR" => {
                if let Err(e) = check_attr_format(
                    key.as_str(),
                    attr.as_ref().unwrap_or(&"".to_string()).as_str(),
                ) {
                    log::warn!("{}", e);
                    return Err(e);
                }

                if op == OperatorType::Remove {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'ATTR' can not take remove operator.".location(&context),
                    });
                }

                if matches!(op, OperatorType::Add | OperatorType::AssignFinal) {
                    log::warn!(
                        "{}",
                        "Key 'ATTR' can not take '+=' and ':=' operator, change to '=' implicitly."
                            .location(&context)
                    );
                    op = OperatorType::Assign;
                }

                if !op_is_match {
                    if let Err(e) = check_value_format(key.as_str(), value.as_str(), false) {
                        log::warn!("{}", e.location(&context));
                    }
                    Ok(RuleToken::new(
                        TokenType::AssignAttr,
                        op,
                        attr,
                        value,
                        rule_line,
                    )?)
                } else {
                    Ok(RuleToken::new(
                        TokenType::MatchAttr,
                        op,
                        attr,
                        value,
                        rule_line,
                    )?)
                }
            }
            "SYSCTL" => {
                if let Err(e) = check_attr_format(
                    key.as_str(),
                    attr.as_ref().unwrap_or(&"".to_string()).as_str(),
                ) {
                    log::warn!("{}", e.location(&context));
                    return Err(e);
                }

                if op == OperatorType::Remove {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'SYSCTL' can not take remove operator.".location(&context),
                    });
                }

                if matches!(op, OperatorType::Add | OperatorType::AssignFinal) {
                    log::warn!("{}", "Key 'SYSCTL' can not take '+=' and ':=' operator, change to '=' implicitly.".location(&context));
                    op = OperatorType::Assign;
                }

                if !op_is_match {
                    if let Err(e) = check_value_format(key.as_str(), value.as_str(), false) {
                        log::warn!("{}", e.location(&context));
                    }

                    Ok(RuleToken::new(
                        TokenType::AssignAttr,
                        op,
                        attr,
                        value,
                        rule_line,
                    )?)
                } else {
                    Ok(RuleToken::new(
                        TokenType::MatchAttr,
                        op,
                        attr,
                        value,
                        rule_line,
                    )?)
                }
            }
            "KERNELS" => {
                if attr.is_some() {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'KERNELS' can not have attribute.".location(&context),
                    });
                }
                if !op_is_match {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'KERNELS' should take match operator.".location(&context),
                    });
                }

                Ok(RuleToken::new(
                    TokenType::MatchParentsKernel,
                    op,
                    None,
                    value,
                    rule_line,
                )?)
            }
            "SUBSYSTEMS" => {
                if attr.is_some() {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'SUBSYSTEMS' can not have attribute.".location(&context),
                    });
                }
                if !op_is_match {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'SUBSYSTEMS' should take match operator.".location(&context),
                    });
                }

                Ok(RuleToken::new(
                    TokenType::MatchParentsSubsystem,
                    op,
                    None,
                    value,
                    rule_line,
                )?)
            }
            "DRIVERS" => {
                if attr.is_some() {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'DRIVERS' can not have attribute.".location(&context),
                    });
                }
                if !op_is_match {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'DRIVERS' should take match operator.".location(&context),
                    });
                }

                Ok(RuleToken::new(
                    TokenType::MatchParentsDriver,
                    op,
                    None,
                    value,
                    rule_line,
                )?)
            }
            "ATTRS" => {
                if let Err(e) =
                    check_attr_format(key.as_str(), attr.clone().unwrap_or_default().as_str())
                {
                    log::warn!("{}", e.location(&context));
                    return Err(e);
                }

                if !op_is_match {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'ATTRS' must take match operators.".location(&context),
                    });
                }

                if attr.clone().unwrap().starts_with("device/") {
                    log::warn!(
                        "{}",
                        "'device' may be deprecated in future.".location(&context)
                    );
                }

                if attr.clone().unwrap().starts_with("../") {
                    log::warn!(
                        "{}",
                        "direct reference to parent directory may be deprecated in future."
                            .location(&context)
                    );
                }

                Ok(RuleToken::new(
                    TokenType::MatchParentsAttr,
                    op,
                    attr,
                    value,
                    rule_line,
                )?)
            }
            "TAGS" => {
                if attr.is_some() {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'TAGS' can not have attribute.".location(&context),
                    });
                }

                if !op_is_match {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'TAGS' can only take match operator.".location(&context),
                    });
                }

                Ok(RuleToken::new(
                    TokenType::MatchParentsTag,
                    op,
                    None,
                    value,
                    rule_line,
                )?)
            }
            "TEST" => {
                if attr.is_some() {
                    parse_mode(&attr.clone().unwrap()).map_err(|e| Error::RulesLoadError {
                        msg: format!("Key 'TEST' failed to parse mode: {}", e).location(&context),
                    })?;
                }

                if let Err(e) = check_value_format(key.as_str(), value.as_str(), true) {
                    log::warn!("{}", e.location(&context));
                }

                if !op_is_match {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'TEST' must tate match operator.".location(&context),
                    });
                }

                Ok(RuleToken::new(
                    TokenType::MatchTest,
                    op,
                    attr,
                    value,
                    rule_line,
                )?)
            }
            "PROGRAM" => {
                if attr.is_some() {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'PROGRAM' can not have attribute.".location(&context),
                    });
                }

                if let Err(e) = check_value_format(key.as_str(), value.as_str(), true) {
                    log::warn!("{}", e.location(&context));
                }

                if op == OperatorType::Remove {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'PROGRAM' must have nonempty value.".location(&context),
                    });
                }

                if !op_is_match {
                    op = OperatorType::Match;
                }

                Ok(RuleToken::new(
                    TokenType::MatchProgram,
                    op,
                    attr,
                    value,
                    rule_line,
                )?)
            }
            "IMPORT" => {
                if attr.is_none() {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'IMPORT' must have attribute.".location(&context),
                    });
                }

                if let Err(e) = check_value_format(key.as_str(), value.as_str(), true) {
                    log::warn!("{}", e.location(&context));
                }

                if !op_is_match {
                    log::warn!(
                        "{}",
                        "Key 'IMPORT' must take match operator, implicitly change to '='."
                            .location(&context)
                    );
                    op = OperatorType::Match;
                }

                if attr.as_ref().unwrap() == "file" {
                    Ok(RuleToken::new(
                        TokenType::MatchImportFile,
                        op,
                        attr,
                        value,
                        rule_line,
                    )?)
                } else if attr.as_ref().unwrap() == "program" {
                    match value.parse::<BuiltinCommand>() {
                        Ok(_) => {
                            log::debug!(
                                "{}",
                                "Parse the program into builtin command.".location(&context)
                            );
                            Ok(RuleToken::new(
                                TokenType::MatchImportBuiltin,
                                op,
                                attr,
                                value,
                                rule_line,
                            )?)
                        }
                        Err(_) => Ok(RuleToken::new(
                            TokenType::MatchImportProgram,
                            op,
                            attr,
                            value,
                            rule_line,
                        )?),
                    }
                } else if attr.as_ref().unwrap() == "builtin" {
                    if value.parse::<BuiltinCommand>().is_err() {
                        return Err(Error::RulesLoadError {
                            msg: format!("Invalid builtin command: {}", value).location(&context),
                        });
                    }

                    Ok(RuleToken::new(
                        TokenType::MatchImportBuiltin,
                        op,
                        attr,
                        value,
                        rule_line,
                    )?)
                } else {
                    let token_type = match attr.as_ref().unwrap().as_str() {
                        "db" => TokenType::MatchImportDb,
                        "cmdline" => TokenType::MatchImportCmdline,
                        "parent" => TokenType::MatchImportParent,
                        _ => {
                            return Err(Error::RulesLoadError {
                                msg: "Key 'IMPORT' has invalid attribute.".location(&context),
                            })
                        }
                    };

                    Ok(RuleToken::new(token_type, op, attr, value, rule_line)?)
                }
            }
            "RESULT" => {
                if attr.is_some() {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'RESULT' can not have attribute.".location(&context),
                    });
                }

                if !op_is_match {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'RESULT' must take match operator.".location(&context),
                    });
                }

                Ok(RuleToken::new(
                    TokenType::MatchResult,
                    op,
                    attr,
                    value,
                    rule_line,
                )?)
            }
            "OPTIONS" => {
                if attr.is_some() {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'OPTIONS' can not have attribute.".location(&context),
                    });
                }
                if op_is_match || op == OperatorType::Remove {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'OPTIONS' can not take match or remove operator."
                            .location(&context),
                    });
                }
                if op == OperatorType::Add {
                    op = OperatorType::Assign;
                }

                match value.as_str() {
                    "string_escape=none" => Ok(RuleToken::new(
                        TokenType::AssignOptionsStringEscapeNone,
                        op,
                        None,
                        "".to_string(),
                        rule_line,
                    )?),
                    "string_escape=replace" => Ok(RuleToken::new(
                        TokenType::AssignOptionsStringEscapeReplace,
                        op,
                        None,
                        "".to_string(),
                        rule_line,
                    )?),
                    "db_persist" => Ok(RuleToken::new(
                        TokenType::AssignOptionsDbPersist,
                        op,
                        None,
                        "".to_string(),
                        rule_line,
                    )?),
                    "watch" => Ok(RuleToken::new(
                        TokenType::AssignOptionsWatch,
                        op,
                        None,
                        "true".to_string(),
                        rule_line,
                    )?),
                    "nowatch" => Ok(RuleToken::new(
                        TokenType::AssignOptionsWatch,
                        op,
                        None,
                        "false".to_string(),
                        rule_line,
                    )?),
                    _ => {
                        if let Some(strip_value) = value.strip_prefix("static_node=") {
                            Ok(RuleToken::new(
                                TokenType::AssignOptionsStaticNode,
                                op,
                                None,
                                strip_value.to_string(),
                                rule_line,
                            )?)
                        } else if let Some(strip_value) = value.strip_prefix("link_priority=") {
                            if value["link_priority=".len()..].parse::<i32>().is_err() {
                                return Err(Error::RulesLoadError { msg: "Key 'OPTIONS' failed to parse link priority into a valid number.".location(&context) });
                            }

                            Ok(RuleToken::new(
                                TokenType::AssignOptionsDevlinkPriority,
                                op,
                                None,
                                strip_value.to_string(),
                                rule_line,
                            )?)
                        } else if let Some(strip_value) = value.strip_prefix("log_level=") {
                            let level = if strip_value == "rest" {
                                "-1"
                            } else {
                                if let Err(e) = strip_value.parse::<i32>() {
                                    return Err(Error::RulesLoadError {
                                        msg: format!(
                                            "Key 'OPTIONS' failed to parse log level: {}",
                                            e
                                        )
                                        .location(&context),
                                    });
                                }
                                strip_value
                            };

                            Ok(RuleToken::new(
                                TokenType::AssignOptionsLogLevel,
                                op,
                                None,
                                level.to_string(),
                                rule_line,
                            )?)
                        } else {
                            Err(Error::RulesLoadError {
                                msg: "Key 'OPTIONS' has invalid value.".location(&context),
                            })
                        }
                    }
                }
            }
            "OWNER" => {
                if attr.is_some() {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'OWNER' can not have attribute.".location(&context),
                    });
                }

                if op_is_match || op == OperatorType::Remove {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'OWNER' can not take match or remove operator."
                            .location(&context),
                    });
                }

                if op == OperatorType::Add {
                    log::warn!(
                        "{}",
                        "Key 'OWNER' can not take add operator, change to '=' implicitly."
                            .location(&context)
                    );
                    op = OperatorType::Assign;
                }

                /*
                 *  If a legal uid is provided, directly pass the uid to rules executer
                 */
                if parse_uid(&value).is_ok() {
                    return RuleToken::new(TokenType::AssignOwnerId, op, attr, value, rule_line);
                }

                let time = rules.read().unwrap().resolve_name_time;
                if time == ResolveNameTime::Early
                    && SubstituteType::Plain == value.parse::<SubstituteType>().unwrap()
                {
                    /*
                     *  If the OWNER value is a legal user name, and resolve_name_time is set to 'Early',
                     *  try to get the uid by resolving the user name.
                     */
                    let user = rules.as_ref().write().unwrap().resolve_user(&value)?;

                    log::debug!(
                        "{}",
                        format!(
                            "owner '{}' is parsed into uid '{}' during rules loading",
                            value, user.uid
                        )
                        .location(&context)
                    );

                    return RuleToken::new(
                        TokenType::AssignOwnerId,
                        op,
                        attr,
                        user.uid.to_string(),
                        rule_line,
                    );
                } else if time != ResolveNameTime::Never {
                    /*
                     *  If the resolve_name_time is not set to 'Never', try to format the value during rules executing.
                     *  Here we only check whether the format of value is legal.
                     */
                    if let Err(e) = check_value_format("OWNER", value.as_str(), true) {
                        log::warn!("{}", e.location(&context));
                    }

                    return RuleToken::new(TokenType::AssignOwner, op, attr, value, rule_line);
                }

                Err(Error::IgnoreError {
                    msg: format!("Ignore resolving user name: 'OWNER=\"{}\"'", value)
                        .location(&context),
                })
            }
            "GROUP" => {
                if attr.is_some() {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'GROUP' can not have attribute.".location(&context),
                    });
                }

                if op_is_match || op == OperatorType::Remove {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'GROUP' can not take match or remove operator."
                            .location(&context),
                    });
                }

                if op == OperatorType::Add {
                    log::warn!(
                        "{}",
                        "Key 'GROUP' can not take add operator, change to '=' implicitly."
                            .location(&context)
                    );
                    op = OperatorType::Assign;
                }

                /*
                 *  If a legal gid is provided, directly pass the gid to rules executer
                 */
                if parse_gid(&value).is_ok() {
                    return RuleToken::new(TokenType::AssignGroupId, op, attr, value, rule_line);
                }

                let time = rules.as_ref().read().unwrap().resolve_name_time;
                if time == ResolveNameTime::Early
                    && SubstituteType::Plain == value.parse::<SubstituteType>().unwrap()
                {
                    /*
                     *  If the GROUP value is a legal group name, and resolve_name_time is set to 'Early',
                     *  try to get the gid by resolving the group name.
                     */
                    let group: Group = rules.as_ref().write().unwrap().resolve_group(&value)?;

                    log::debug!(
                        "{}",
                        format!(
                            "group '{}' is parsed into gid '{}' during rules loading",
                            value, group.gid
                        )
                        .location(&context)
                    );

                    return RuleToken::new(
                        TokenType::AssignGroupId,
                        op,
                        attr,
                        group.gid.to_string(),
                        rule_line,
                    );
                } else if time != ResolveNameTime::Never {
                    /*
                     * If resolve_name_time is not set to 'Never', try to format the value during rules executing.
                     * Here we only check the format of value is legal.
                     */
                    if let Err(e) = check_value_format("GROUP", value.as_str(), true) {
                        log::warn!("{}", e);
                    }

                    return RuleToken::new(TokenType::AssignGroup, op, attr, value, rule_line);
                }

                Err(Error::IgnoreError {
                    msg: format!("Ignore resolving user name: 'GROUP=\"{}\"'", value)
                        .location(&context),
                })
            }
            "MODE" => {
                if attr.is_some() {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'MODE' can not have attribute.".location(&context),
                    });
                }

                if op_is_match || op == OperatorType::Remove {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'MODE' can not take match or remove operator.".location(&context),
                    });
                }

                if op == OperatorType::Add {
                    log::warn!(
                        "{}",
                        "Key 'MODE' can not take add operator, change to '=' implicitly."
                            .location(&context)
                    );
                    op = OperatorType::Assign;
                }

                if parse_mode(&value).is_ok() {
                    /*
                     * todo: if a legal mode is provided, directly pass the mode to rules executer.
                     * However, currently rules token uses string to carry token value, which leads
                     * to repeatedly string parse during rules executing. In future, we will let rules
                     * token carry raw data and transform the data into specific structure object directly.
                     */
                    Ok(RuleToken::new(
                        TokenType::AssignModeId,
                        op,
                        None,
                        value,
                        rule_line,
                    )?)
                } else {
                    if let Err(e) = check_value_format(key.as_str(), value.as_str(), true) {
                        log::warn!("{}", e.location(&context));
                    }

                    Ok(RuleToken::new(
                        TokenType::AssignMode,
                        op,
                        None,
                        value,
                        rule_line,
                    )?)
                }
            }
            "SECLABEL" => {
                if attr.is_none() {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'SECLABEL' should take attribute.".location(&context),
                    });
                }

                if let Err(e) = check_value_format("SECLABEL", value.as_str(), true) {
                    log::warn!("{}", e.location(&context));
                }

                if op_is_match || op == OperatorType::Remove {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'SECLABEL' can not take match or remove operator."
                            .location(&context),
                    });
                }

                if op == OperatorType::AssignFinal {
                    log::warn!(
                        "{}",
                        "Key 'SECLABEL' can not take ':=' operator, change to '=' implicitly."
                            .location(&context)
                    );
                    op = OperatorType::Assign;
                }

                Ok(RuleToken::new(
                    TokenType::AssignSeclabel,
                    op,
                    attr,
                    value,
                    rule_line,
                )?)
            }
            "RUN" => {
                if op_is_match || op == OperatorType::Remove {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'RUN' can not take match or remove operator.".location(&context),
                    });
                }

                if let Err(e) = check_value_format("RUN", value.as_str(), true) {
                    log::warn!("{}", e.location(&context));
                }

                let attr_content = attr.clone().unwrap_or_default();
                if attr.is_none() || attr_content == "program" {
                    Ok(RuleToken::new(
                        TokenType::AssignRunProgram,
                        op,
                        None,
                        value,
                        rule_line,
                    )?)
                } else if attr_content == "builtin" {
                    if value.parse::<BuiltinCommand>().is_err() {
                        return Err(Error::RulesLoadError {
                            msg: format!("Key 'RUN' failed to parse builin command '{}'", value)
                                .location(&context),
                        });
                    }

                    Ok(RuleToken::new(
                        TokenType::AssignRunBuiltin,
                        op,
                        attr,
                        value,
                        rule_line,
                    )?)
                } else {
                    Err(Error::IgnoreError {
                        msg: format!(
                            "Ignore 'Run' token with invalid attribute {}.",
                            attr.unwrap()
                        )
                        .location(&context),
                    })
                }
            }
            "GOTO" => {
                if attr.is_some() {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'GOTO' can not have attribute.".location(&context),
                    });
                }

                if op != OperatorType::Assign {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'GOTO' should take '=' operator.".location(&context),
                    });
                }

                Ok(RuleToken::new(TokenType::Goto, op, None, value, rule_line)?)
            }
            "LABEL" => {
                if attr.is_some() {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'LABEL' can not have attribute.".location(&context),
                    });
                }

                if op != OperatorType::Assign {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'LABEL' should take '=' operator.".location(&context),
                    });
                }

                Ok(RuleToken::new(
                    TokenType::Label,
                    op,
                    None,
                    value,
                    rule_line,
                )?)
            }
            _ => Err(Error::RulesLoadError {
                msg: format!("Key '{}' is not supported.", key).location(&context),
            }),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use basic::fs::touch_file;
    use log::init_log;
    use log::Level;

    use super::*;
    use std::fs::create_dir_all;
    use std::fs::remove_dir_all;
    use std::io::Write;
    use std::panic::catch_unwind;
    use std::{fs, path::Path};

    pub(crate) fn create_tmp_file(dir: &'static str, file: &str, content: &str, truncate: bool) {
        fs::create_dir_all(dir).unwrap();
        let s = format!("{}/{}", dir, file);
        let p = Path::new(&s);
        fs::write(p, content).unwrap();
        let mut f = fs::OpenOptions::new()
            .write(true)
            .truncate(truncate)
            .open(p)
            .unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f.flush().unwrap();
        while !p.exists() {}
    }

    fn clear_tmp_rules(dir: &'static str) {
        if Path::new(dir).exists() {
            fs::remove_dir_all(dir).unwrap();
        }
    }

    #[test]
    fn test_load_rules() {
        init_log(
            "test_load_rules",
            Level::Debug,
            vec!["console"],
            "",
            0,
            0,
            false,
        );
        clear_tmp_rules("/tmp/devmaster/test_load_rules");

        let legal_rule = vec![
            "ACTION == \"change\", SYMLINK += \"test1\"", // Test legal rules.
            "ACTION == \"change\", SYMLINK += \"test11\", \\
            SYMLINK += \"test111\"", // Test double line tying.
            "ACTION == \"change\", SYMLINK += \"test1111\", \\
            SYMLINK += \"test11111\", \\
            SYMLINK += \"test111111\"", // Test triple line tying.
            "SYMLINK == \"$hello\"", // Illegal placeholder will throw warning rather than panic.
            "NAME += \"xxx\"",       // NAME will transfer operator += to =.
            "NAME == \"$hello\"",    // Illegal placeholder will throw warning rather than panic.
            "ENV{xxx}:=\"xxx\"",     // ENV will transfer final assignment := to =.
            "ENV{xxx}=\"$hello\"",   // Illegal placeholder will throw warning rather than panic.
            "CONST{arch}==\"x86_64\"", // Test legal CONST usage.
            "CONST{virt}==\"qemu\"", // Test legal CONST usage.
            "SUBSYSTEM==\"bus\"", // SUBSYSTEM will throw warning if the value is 'bus' or 'class'.
            "DRIVER==\"xxx\"",    // Test DRIVER usage.
            /* ATTR will throw warning if the operator is += or :=,
             * and transfer the operator into =.
             */
            "ATTR{xxx}+=\"xxx\"",
            "ATTR{xxx}:=\"xxx\"",
            "ATTR{xxx}=\"$hello\"", // Illegal placeholder in value will throw warning rather than panic.
            /* Test SYSCTL usage. */
            "SYSCTL{hello}=\"world\"",
            "SYSCTL{hello}==\"world\"",
            /* SYSCTL will transfer the += and := operator to =, and trow warning. */
            "SYSCTL{hello}+=\"world\"",
            "SYSCTL{hello}:=\"world\"",
            "SYSCTL{hello}=\"$hello\"", // Illegal placeholder in value will throw warning rather than panic.
            "ATTRS{device/xxx}==\"xxx\"", // The attribute with prefix of 'device/' will trow warning.
            "ATTRS{../xxx}==\"xxx\"", // The attribute with prefix of 'device/' will throw warning.
            "TAGS==\"xxx\"",          // Test TAGS usage.
            "TEST{777}==\"xx\"",      // Test TEST usage.
            "TEST{777}==\"$hello\"", // Illegal placeholder in value will throw warning rather than panic.
            "PROGRAM==\"$hello\"", // Illegal placeholder in value will throw warning rather than panic.
            "IMPORT{program}==\"$hello\"", // Illegal placeholder in value will throw warning rather than panic.
            "IMPORT{file}=\"x\"", // IMPORT will throw warning if the operator is not matching or unmatching and transfer it to ==.
            "IMPORT{program}==\"path_id $kernel\"", // If the program is a built-in command, IMPORT will identify it.
            /* Test OPTIONS usages. */
            "OPTIONS+=\"string_escape=none\"",
            "OPTIONS+=\"db_persist\"",
            "OPTIONS+=\"log_level=rest\"",
            "OPTIONS+=\"log_level=10\"",
            "OWNER+=\"0\"",    // OWNER will transfer += to =, and trow a warning.
            "GROUP+=\"0\"",    // GROUP will transfer += to =, and trow a warning.
            "MODE+=\"777\"",   // MODE will transfer += to =, and trow a warning.
            "MODE=\"$hello\"", // Illegal placeholder in value will throw warning rather than panic.
            "SECLABEL{x}:=\"$hello\"", // Illegal placeholder in value will throw warning rather than panic.
        ];

        for &content in legal_rule.iter() {
            create_tmp_file(
                "/tmp/devmaster/test_load_rules",
                "00-test.rules",
                content,
                true,
            );

            let _ = Rules::load_rules(
                vec!["/tmp/devmaster/test_load_rules".to_string()],
                ResolveNameTime::Early,
            );
        }

        clear_tmp_rules("/tmp/devmaster/test_load_rules");
    }

    #[test]
    fn test_load_rules_panic() {
        init_log(
            "test_load_rules_panic",
            Level::Debug,
            vec!["console"],
            "",
            0,
            0,
            false,
        );
        clear_tmp_rules("/tmp/devmaster/test_load_rules_panic");

        let illegal_rule = vec![
            "action==\"change\"",       // Error in State::Pre
            "ACtion==\"change\"",       // Error in State::Key
            "ENV{!}==\"hello\"",        // Error in State::Attribute
            "ACTION #= \"hello\"",      // Error in State::PreOp
            "ACTION =# \"hello\"",      // Error in State::Op
            "ACTION == hello",          // Error in State::PostOp
            "ACTION == \"change\"x",    // Error in State::PostValue
            "ACTION = \"change\"",      // ACTION can not take assign operator.
            "DEVPATH{xxx} == \"xxx\"",  // DEVPATH can not take attribute.
            "DEVPATH = \"xxx\"",        // DEVPATH can no take assign operator.
            "KERNEL{xxx} == \"xxx\"",   // KERNEL can not take attribute.
            "KERNEL = \"xxx\"",         // KERNEL can not take assign operator.
            "SYMLINK{xxx} = \"hello\"", // SYMLINK can not take attribute.
            "NAME{xxx} = \"xxx\"",      // NAME can not take attribute.
            "NAME -= \"xxx\"",          // NAME can not take removal operator.
            "NAME=\"%k\"",              // NAME can not take '%k' value.
            "NAME=\"\"",                // NAME can not take empty value.
            "ENV=\"xxx\"",              // ENV must take attribute.
            "ENV{xxx}-=\"xxx\"",        // ENV can not take removal operator.
            /* ENV with non-match operator can not take the following attributes:
             *  "ACTION"
             *  "DEVLINKS"
             *  "DEVNAME"
             *  "DEVTYPE"
             *  "DRIVER"
             *  "IFINDEX"
             *  "MAJOR"
             *  "MINOR"
             *  "SEQNUM"
             *  "SUBSYSTEM"
             *  "TAGS"
             */
            "ENV{ACTION}=\"xxx\"",
            "ENV{DEVLINKS}=\"xxx\"",
            "ENV{DEVNAME}=\"xxx\"",
            "ENV{DEVTYPE}=\"xxx\"",
            "ENV{DRIVER}=\"xxx\"",
            "ENV{IFINDEX}=\"xxx\"",
            "ENV{MAJOR}=\"xxx\"",
            "ENV{MINOR}=\"xxx\"",
            "ENV{SEQNUM}=\"xxx\"",
            "ENV{SUBSYSTEM}=\"xxx\"",
            "ENV{TAGS}=\"xxx\"",
            "CONST==\"xxx\"",            // CONST must take an attribute.
            "CONST{xxx}==\"xxx\"",       // CONST can only take "arch" or "virt" attribute.
            "CONST{virt}=\"qemu\"",      // CONST can not take assignment operator.
            "TAG{xxx}+=\"xxx\"",         // TAG can not take attribute.
            "SUBSYSTEM{xxx}==\"block\"", // SUBSYSTEM can not take attribute.
            "SUBSYSTEM=\"block\"", // SUBSYSTEM can only take matching or unmatching operators.
            "DRIVER{xxx}==\"xxx\"", // DRIVER can not take attribute.
            "DRIVER=\"xxx\"",      // DRIVER can only take matching or unmatching operators.
            "ATTR{$hello}==\"xxx\"", // ATTR can not take illegal attribute.
            "ATTR{hello}-=\"xxx\"", // ATTR can not take removal operator.
            /* SYSCTL must take attribute. */
            "SYSCTL=\"xxx\"",
            "SYSCTL==\"xxx\"",
            "SYSCTL{xxx}-=\"xxx\"",  // SYSCTL can not take removal operator.
            "KERNELS{xxx}==\"xxx\"", // KERNELS can not take attribute.
            "KERNELS=\"xxx\"",       // KERNELS can only take matching or unmatching operators.
            "SUBSYSTEMS{xxx}==\"xxx\"", // SUBSYSTEMS can not take attribute.
            "SUBSYSTEMS=\"xxx\"",    // SUBSYSTEMS can not take assignment operators.
            "DRIVERS{xxx}=\"xxx\"",  // DRIVERS can not take attribute.
            "DRIVERS=\"xxx\"",       // DRIVERS can not take assignment operators.
            "ATTRS==\"xxx\"",        // ATTRS must take an attribute.
            "ATTRS{xxx}=\"x\"",      // ATTRS can not take assignment operators.
            "TAGS{xxx}=\"xxx\"",     // TAGS can not take attribute.
            "TAGS=\"xxx\"",          // TAGS can not take assignment operators.
            "TEST{777}=\"x\"",       // TEST can not take assignment operators.
            "PROGRAM{x}==\"x\"",     // PROGRAM can not take attribute.
            "PROGRAM-=\"x\"",        // PROGRAM can not take removal attribute.
            "IMPORT==\"x\"",         // IMPORT must take an attribute.
            "IMPORT{builtin}==\"xxx $kernel\"", // IMPORT{builtin} will panic if the command is not a valid built-in.
            "IMPORT{x}==\"x\"",                 // IMPORT will panic if the attribute is invalid.
            "RESULT{x}==\"x\"",                 // RESULT can not take attribute.
            "RESULT{x}=\"x\"", // RESULT can only take matching or unmatching operator.
            "OPTIONS{x}+=\"x\"", // OPTIONS can not take attribute.
            "OPTIONS{x}==\"x\"", // OPTIONS can not take matching or unmatching operator.
            "OPTIONS{x}-=\"x\"", // OPTIONS can not take removal operator.
            "OPTIONS+=\"link_priority=x\"", // Invalid number of link priority.
            "OPTIONS+=\"log_level=xxx\"", // Invalid log_level.
            "OWNER{x}==\"x\"", // OWNER can not take attribute.
            "OWNER==\"0\"",    // OWNER can not take matching or unmatching operator.
            "OWNER-=\"0\"",    // OWNER can not take removal operator.
            "GROUP==\"0\"",    // OWNER can not take matching or unmatching operator.
            "GROUP-=\"0\"",    // OWNER can not take removal operator.
            "MODE{x}=\"777\"", // MODE can not take attribute.
            "MODE==\"777\"",   // MODE can not take matching or unmatching operator.
            "MODE-=\"777\"",   // MODE can not take removal operator.
            "SECLABEL=\"xxx\"", // SECLABEL must take an attribute.
            "SECLABEL{x}==\"x\"", // SECLABEL can not take matching or unmatching operator.
            "SECLABEL{x}-=\"x\"", // SECLABEL can not take removal operator.
            "RUN==\"xxx\"",    // RUN can not take matching or unmatching operator.
            "RUN-=\"xxx\"",    // RUN can not take removal operator.
            "RUN{builtin}==\"xxx\"", // RUN will panic if the builtin is invalid.
            "RUN{xxx}==\"xxx\"", // RUN will panic if the attribute is not builtin or program.
            "GOTO{xx}=\"xx\"", // GOTO can not take attribute.
            "GOTO==\"xx\"",    // GOTO can only take assignment operator.
            "LABEL{x}==\"x\"", // LABEL can not take attribute.
            "LABEL==\"x\"",    // LABEL can only take assignment operator.
            "XXX=\"xxx\"",     // Invalid token key.
        ];

        for content in illegal_rule.iter() {
            create_tmp_file(
                "/tmp/devmaster/test_load_rules_panic",
                "00-test.rules",
                content,
                true,
            );

            assert!(catch_unwind(|| {
                let _ = Rules::load_rules(
                    vec!["/tmp/devmaster/test_load_rules_panic".to_string()],
                    ResolveNameTime::Early,
                );
            })
            .is_err());
        }

        clear_tmp_rules("/tmp/devmaster/test_load_rules_panic");
    }

    #[test]
    fn test_resolve_name_time() {
        init_log(
            "test_load_rules",
            Level::Debug,
            vec!["console"],
            "",
            0,
            0,
            false,
        );
        clear_tmp_rules("/tmp/devmaster/test_resolve_name_time");

        let legal = vec!["OWNER=\"root\"", "GROUP=\"root\""];
        let illegal = vec!["OWNER=\"xxxx\"", "GROUP=\"xxxx\""];

        for &content in legal.iter() {
            create_tmp_file(
                "/tmp/devmaster/test_resolve_name_time",
                "00-test.rules",
                content,
                true,
            );

            let _ = Rules::load_rules(
                vec!["/tmp/devmaster/test_resolve_name_time".to_string()],
                ResolveNameTime::Early,
            );
        }

        for &content in illegal.iter() {
            create_tmp_file(
                "/tmp/devmaster/test_resolve_name_time",
                "00-test.rules",
                content,
                true,
            );

            let _ = Rules::load_rules(
                vec!["/tmp/devmaster/test_resolve_name_time".to_string()],
                ResolveNameTime::Late,
            );

            assert!(catch_unwind(|| {
                let _ = Rules::load_rules(
                    vec!["/tmp/devmaster/test_resolve_name_time".to_string()],
                    ResolveNameTime::Early,
                );
            })
            .is_err());
        }

        clear_tmp_rules("/tmp/devmaster/test_resolve_name_time");
    }

    #[test]
    fn test_rules_file() {
        let rules = Arc::new(RwLock::new(Rules::new(vec![], ResolveNameTime::Never)));
        fs::write(
            "test_rules_file.rules",
            "ACTION == \"change\", SYMLINK+=\"test\"\nACTION != \"change\"\n",
        )
        .unwrap();
        RuleFile::load_file("test_rules_file.rules".to_string(), rules);
        fs::remove_file("test_rules_file.rules").unwrap();
    }

    #[test]
    fn test_rules_token() {
        let rules = Arc::new(RwLock::new(Rules::new(vec![], ResolveNameTime::Never)));
        let rule_file = Arc::new(RwLock::new(Some(RuleFile::new("test".to_string()))));
        let rule_line = Arc::new(RwLock::new(Some(RuleLine::new(
            "".to_string(),
            0,
            rule_file,
        ))));

        assert!(RuleToken::parse_token(
            "ACTION".to_string(),
            None,
            "==".to_string(),
            "add".to_string(),
            rules.clone(),
            rule_line.clone(),
        )
        .is_ok());

        assert!(RuleToken::parse_token(
            "ACTION".to_string(),
            None,
            "!=".to_string(),
            "add".to_string(),
            rules.clone(),
            rule_line.clone(),
        )
        .is_ok());

        assert!(RuleToken::parse_token(
            "ACTION".to_string(),
            None,
            "*=".to_string(),
            "add".to_string(),
            rules.clone(),
            rule_line.clone(),
        )
        .is_err());

        assert!(RuleToken::parse_token(
            "ACTION".to_string(),
            Some("whatever".to_string()),
            "==".to_string(),
            "add".to_string(),
            rules,
            rule_line,
        )
        .is_err());
    }

    #[test]
    fn test_rules_token_regex() {
        let rules = Arc::new(RwLock::new(Rules::new(vec![], ResolveNameTime::Never)));
        let rule_file = Arc::new(RwLock::new(Some(RuleFile::new("test".to_string()))));
        let rule_line = Arc::new(RwLock::new(Some(RuleLine::new(
            "".to_string(),
            0,
            rule_file,
        ))));

        let t = RuleToken::parse_token(
            "ACTION".to_string(),
            None,
            "==".to_string(),
            "add".to_string(),
            rules.clone(),
            rule_line.clone(),
        )
        .unwrap();

        println!("{:?}", t);

        let t = RuleToken::parse_token(
            "ACTION".to_string(),
            None,
            "==".to_string(),
            ".?.*".to_string(),
            rules.clone(),
            rule_line.clone(),
        )
        .unwrap();

        println!("{:?}", t);

        let t = RuleToken::parse_token(
            "ACTION".to_string(),
            None,
            "==".to_string(),
            "?*".to_string(),
            rules.clone(),
            rule_line.clone(),
        )
        .unwrap();

        println!("{:?}", t);

        let t = RuleToken::parse_token(
            "ACTION".to_string(),
            None,
            "==".to_string(),
            "hello|?*|hello*|3279/tty[0-9]*".to_string(),
            rules.clone(),
            rule_line.clone(),
        )
        .unwrap();

        println!("{:?}", t);

        let t = RuleToken::parse_token(
            "ACTION".to_string(),
            None,
            "==".to_string(),
            String::default(),
            rules.clone(),
            rule_line.clone(),
        )
        .unwrap();

        println!("{:?}", t);

        let t = RuleToken::parse_token(
            "ACTION".to_string(),
            None,
            "==".to_string(),
            "|hello|?*|hello*|3279/tty[0-9]*".to_string(),
            rules.clone(),
            rule_line.clone(),
        )
        .unwrap();

        println!("{:?}", t);

        let t = RuleToken::parse_token(
            "ATTR".to_string(),
            Some("whatever".to_string()),
            "==".to_string(),
            "hello".to_string(),
            rules.clone(),
            rule_line.clone(),
        )
        .unwrap();

        println!("{:?}", t);

        let t = RuleToken::parse_token(
            "ATTR".to_string(),
            Some("whatever$".to_string()),
            "==".to_string(),
            "hello".to_string(),
            rules.clone(),
            rule_line.clone(),
        )
        .unwrap();

        println!("{:?}", t);

        let t = RuleToken::parse_token(
            "ATTR".to_string(),
            Some("whatever%".to_string()),
            "==".to_string(),
            "hello".to_string(),
            rules,
            rule_line,
        )
        .unwrap();

        println!("{:?}", t);
    }

    #[test]
    fn test_resolve_user_group() {
        let mut rules = Rules::new(vec![], ResolveNameTime::Early);
        assert!(rules.resolve_user("root").is_ok());
        assert!(rules.users.contains_key("root"));
        assert!(rules.resolve_user("abcdefg").is_err());

        assert!(rules.resolve_group("root").is_ok());
        assert!(rules.groups.contains_key("root"));
        assert!(rules.resolve_group("abcdefg").is_err());
    }

    #[test]
    fn test_load_line() {
        let rules = Arc::new(RwLock::new(Rules::new(
            vec![
                "test_rules_new_1".to_string(),
                "test_rules_new_2".to_string(),
            ],
            ResolveNameTime::Early,
        )));
        let rule_file = Arc::new(RwLock::new(Some(RuleFile::new("test".to_string()))));
        let line =
            RuleLine::load_line("TAG+=\"hello\"", 0, rule_file.clone(), rules.clone()).unwrap();

        let mut iter = line.read().unwrap().as_ref().unwrap().iter();

        let token = iter.next().unwrap();
        assert_eq!(
            token.read().unwrap().as_ref().unwrap().r#type,
            TokenType::AssignTag
        );
        assert_eq!(
            token.read().unwrap().as_ref().unwrap().op,
            OperatorType::Add
        );
        assert_eq!(token.read().unwrap().as_ref().unwrap().attr, None);
        assert_eq!(token.read().unwrap().as_ref().unwrap().value, "hello");

        let line = RuleLine::load_line(
            "TAG   += \"hello\", ENV{hello}=\"world\"",
            0,
            rule_file.clone(),
            rules.clone(),
        )
        .unwrap();
        let mut iter = line.read().unwrap().as_ref().unwrap().iter();
        let token = iter.next().unwrap();
        assert_eq!(
            token.read().unwrap().as_ref().unwrap().r#type,
            TokenType::AssignTag
        );
        assert_eq!(
            token.read().unwrap().as_ref().unwrap().op,
            OperatorType::Add
        );
        assert_eq!(token.read().unwrap().as_ref().unwrap().attr, None);
        assert_eq!(token.read().unwrap().as_ref().unwrap().value, "hello");
        let token = iter.next().unwrap();
        assert_eq!(
            token.read().unwrap().as_ref().unwrap().r#type,
            TokenType::AssignEnv
        );
        assert_eq!(
            token.read().unwrap().as_ref().unwrap().op,
            OperatorType::Assign
        );
        assert_eq!(
            token.read().unwrap().as_ref().unwrap().attr,
            Some("hello".to_string())
        );
        assert_eq!(token.read().unwrap().as_ref().unwrap().value, "world");

        let line = RuleLine::load_line(
            "KERNEL==\"md*\", TEST==\"/run/mdadm/creating-$kernel\", ENV{SYSTEMD_READY}=\"0\"",
            0,
            rule_file,
            rules,
        )
        .unwrap();
        let mut iter = line.read().unwrap().as_ref().unwrap().iter();
        let token = iter.next().unwrap();
        assert_eq!(
            token.read().unwrap().as_ref().unwrap().r#type,
            TokenType::MatchKernel
        );
        assert_eq!(
            token.read().unwrap().as_ref().unwrap().op,
            OperatorType::Match
        );
        assert_eq!(token.read().unwrap().as_ref().unwrap().attr, None);
        assert_eq!(token.read().unwrap().as_ref().unwrap().value, "md*");
        let token = iter.next().unwrap();
        assert_eq!(
            token.read().unwrap().as_ref().unwrap().r#type,
            TokenType::MatchTest
        );
        assert_eq!(
            token.read().unwrap().as_ref().unwrap().op,
            OperatorType::Match
        );
        assert_eq!(token.read().unwrap().as_ref().unwrap().attr, None);
        assert_eq!(
            token.read().unwrap().as_ref().unwrap().value,
            "/run/mdadm/creating-$kernel"
        );
        let token = iter.next().unwrap();
        assert_eq!(
            token.read().unwrap().as_ref().unwrap().r#type,
            TokenType::AssignEnv
        );
        assert_eq!(
            token.read().unwrap().as_ref().unwrap().op,
            OperatorType::Assign
        );
        assert_eq!(
            token.read().unwrap().as_ref().unwrap().attr,
            Some("SYSTEMD_READY".to_string())
        );
        assert_eq!(token.read().unwrap().as_ref().unwrap().value, "0");
    }

    #[test]
    fn test_parse_rules() {
        create_dir_all("/tmp/devmaster/test_parse_rules").unwrap();

        /* Normal rule file. */
        touch_file(
            "/tmp/devmaster/test_parse_rules/00-a.rules",
            false,
            Some(0o777),
            None,
            None,
        )
        .unwrap();
        /* Skip parsing the file with invalid suffix. */
        touch_file(
            "/tmp/devmaster/test_parse_rules/01-b",
            false,
            Some(0o777),
            None,
            None,
        )
        .unwrap();
        /* Failed to parse the file as it is not readable. */
        touch_file(
            "/tmp/devmaster/test_parse_rules/02-c.rules",
            false,
            Some(0o000),
            None,
            None,
        )
        .unwrap();

        let rules = Arc::new(RwLock::new(Rules::new(
            vec!["/tmp/devmaster/test_parse_rules".to_string()],
            ResolveNameTime::Never,
        )));

        RuleFile::load_file(
            "/tmp/devmaster/test_parse_rules/00-a.rules".to_string(),
            rules.clone(),
        );

        if nix::unistd::getuid().as_raw() != 0 {
            assert!(catch_unwind(|| {
                RuleFile::load_file(
                    "/tmp/devmaster/test_parse_rules/02-c.rules".to_string(),
                    rules.clone(),
                );
            })
            .is_err());
        }

        remove_dir_all("/tmp/devmaster/test_parse_rules").unwrap();
    }
}
