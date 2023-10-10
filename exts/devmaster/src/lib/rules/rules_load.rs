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
use lazy_static::lazy_static;
use nix::unistd::{Group, User};
use regex::Regex;
use std::fs::File;
use std::io::{BufRead, BufReader};

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
            files: None,
            files_tail: None,
            dirs,
            resolve_name_time,
            users: HashMap::new(),
            groups: HashMap::new(),
        }
    }

    /// enumerate and parse all rule files under rule directories
    pub(crate) fn parse_rules(rules: Arc<RwLock<Rules>>) {
        let dirs = rules.as_ref().read().unwrap().dirs.clone();
        for dir in dirs {
            let dir_path = std::path::Path::new(&dir);
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
                Self::parse_file(rules.clone(), f);
            }
        }
    }

    /// parse a single rule file, and insert it into rules
    pub(crate) fn parse_file(rules: Arc<RwLock<Rules>>, file_name: String) {
        log::debug!("Parsing rule file: {}", file_name);
        let file = RuleFile::load_file(file_name, Some(rules.clone()));
        Self::add_file(rules, file);
    }

    /// push the rule file into the tail of linked list
    pub(crate) fn add_file(rules: Arc<RwLock<Rules>>, file: Arc<RwLock<RuleFile>>) {
        let has_tail = rules.as_ref().read().unwrap().files_tail.is_none();
        if has_tail {
            rules.as_ref().write().unwrap().files = Some(file.clone());
        } else {
            rules
                .as_ref()
                .write()
                .unwrap()
                .files_tail
                .as_mut()
                .unwrap()
                .write()
                .unwrap()
                .next = Some(file.clone());
            file.write().unwrap().prev = rules.as_ref().read().unwrap().files_tail.clone();
        }

        rules.as_ref().write().unwrap().files_tail = Some(file);
    }

    /// if the user name has valid credential, insert it to rules
    pub(crate) fn resolve_user(&mut self, username: &str) -> Result<User> {
        if let Some(user) = self.users.get(username) {
            return Ok(user.clone());
        }

        match User::from_name(username) {
            Ok(user) => match user {
                Some(u) => Ok(u),
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
                Some(g) => Ok(g),
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
        rules: Option<Arc<RwLock<Rules>>>,
    ) -> Arc<RwLock<RuleFile>> {
        let rule_file = Arc::<RwLock<RuleFile>>::new(RwLock::<RuleFile>::new(Self::new(file_name)));

        // rule file is locked here, thus can not do read or write operations inside parse_lines
        rule_file
            .write()
            .unwrap()
            .parse_lines(rule_file.clone(), rules);

        rule_file
    }

    /// create a initial rule file object
    pub(crate) fn new(file_name: String) -> RuleFile {
        RuleFile {
            rule_file: file_name,
            lines: None,
            lines_tail: None,
            prev: None,
            next: None,
        }
    }

    /// parse and load all available lines in the rule file
    /// the pointer to rules is used for specific tokens, e.g., 'GOTO' and 'LABEL',
    /// which will directly modify some fields in rules
    pub(crate) fn parse_lines(
        &mut self,
        self_ptr: Arc<RwLock<RuleFile>>,
        rules: Option<Arc<RwLock<Rules>>>,
    ) {
        let file = File::open(&self.rule_file).unwrap();
        let reader = BufReader::new(file);

        let mut full_line = String::new();
        let mut offset = 0;
        for (line_number, line) in reader.lines().enumerate() {
            if let Err(e) = line {
                log::warn!("Read line failed in {} : {:?}", self.rule_file, e);
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
                let line = RuleLine::load_line(
                    full_line.to_string(),
                    (line_number + 1 - offset) as u32,
                    self_ptr.clone(),
                    rules.clone(),
                    self.rule_file.clone(),
                )
                .unwrap();
                self.add_line(line);
                full_line.clear();
                offset = 0;
            }
        }

        self.resolve_goto();
    }

    /// push the rule line to the tail of linked list
    pub(crate) fn add_line(&mut self, line: Arc<RwLock<RuleLine>>) {
        if self.lines.is_none() {
            self.lines = Some(line.clone());
        } else {
            self.lines_tail.as_mut().unwrap().write().unwrap().next = Some(line.clone());
            line.write().unwrap().prev = self.lines_tail.clone();
        }

        self.lines_tail = Some(line);
    }

    /// bind goto with label lines
    pub(crate) fn resolve_goto(&mut self) {
        let mut labels: HashMap<String, Arc<RwLock<RuleLine>>> = HashMap::new();

        for line in self.iter() {
            if line
                .as_ref()
                .read()
                .unwrap()
                .r#type
                .intersects(RuleLineType::HAS_LABEL)
            {
                labels.insert(
                    line.as_ref().read().unwrap().label.clone().unwrap(),
                    line.clone(),
                );
            }
        }

        for line in self.iter() {
            if !line
                .as_ref()
                .read()
                .unwrap()
                .r#type
                .intersects(RuleLineType::HAS_GOTO)
            {
                continue;
            }

            let label = line.as_ref().read().unwrap().goto_label.clone().unwrap();

            line.as_ref().write().unwrap().goto_line = Some(labels.get(&label).unwrap().clone());
        }
    }
}

impl RuleLine {
    /// load a rule line
    pub(crate) fn new(
        line: String,
        line_number: u32,
        file: Arc<RwLock<RuleFile>>,
        file_name: String,
    ) -> RuleLine {
        RuleLine {
            line,
            line_number,

            r#type: RuleLineType::INITIAL,

            label: None,
            goto_label: None,
            goto_line: None,

            tokens: None,
            tokens_tail: None,

            rule_file_ptr: Arc::downgrade(&file),
            rule_file: file_name,

            next: None,
            prev: None,
        }
    }

    /// create a rule line object
    /// Note: file is locked previously.
    pub(crate) fn load_line(
        line: String,
        line_number: u32,
        file: Arc<RwLock<RuleFile>>,
        rules: Option<Arc<RwLock<Rules>>>,
        file_name: String,
    ) -> Result<Arc<RwLock<RuleLine>>> {
        lazy_static! {
            static ref RE_LINE: Regex =
                Regex::new("((?P<key>[^=,\"{+\\-!:\0\\s]+)(\\{(?P<attr>[^\\{\\}]+)\\})?\\s*(?P<op>[!:+-=]?=)\\s*\"(?P<value>[^\"]+)\"\\s*,?\\s*)+").unwrap();
            static ref RE_TOKEN: Regex =
                Regex::new("(?P<key>[^=,\"{+\\-!:\0\\s]+)(\\{(?P<attr>[^\\{\\}]+)\\})?\\s*(?P<op>[!:+-=]?=)\\s*\"(?P<value>[^\"]+)\"\\s*,?\\s*").unwrap();
        }

        let mut rule_line = RuleLine::new(line.clone(), line_number, file, file_name.clone());

        if !RE_LINE.is_match(&line) {
            return Err(Error::RulesLoadError {
                msg: "Invalid rule line".to_string(),
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
            let token_str = format!(
                "{}{}{}\"{}\"",
                key,
                attr.clone()
                    .map(|s| format!("{{{}}}", s))
                    .unwrap_or_default(),
                op,
                value
            );

            // if the token is 'GOTO' or 'LABEL', parse_token will return a IgnoreError
            // the following tokens in this line, if any, will be skipped
            let rule_token = RuleToken::parse_token(
                key,
                attr,
                op,
                value,
                rules.clone(),
                (line_number, file_name.clone(), token_str),
            )?;
            match rule_token.r#type {
                TokenType::Goto => {
                    rule_line.goto_label = Some(rule_token.value.clone());
                    rule_line.r#type |= RuleLineType::HAS_GOTO;
                }
                TokenType::Label => {
                    rule_line.label = Some(rule_token.value.clone());
                    rule_line.r#type |= RuleLineType::HAS_LABEL;
                }
                TokenType::AssignName => {
                    rule_line.r#type |= RuleLineType::HAS_NAME;
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
                        rule_line.r#type |= RuleLineType::HAS_DEVLINK;
                    } else if TokenType::AssignOptionsStaticNode == t {
                        rule_line.r#type |= RuleLineType::HAS_STATIC_NODE;
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
                        rule_line.r#type |= RuleLineType::UPDATE_SOMETHING;
                    }
                }
            }
            rule_line.add_token(rule_token);
        }

        Ok(Arc::<RwLock<RuleLine>>::new(RwLock::<RuleLine>::new(
            rule_line,
        )))
    }

    /// push the rule token to the tail of linked list
    pub(crate) fn add_token(&mut self, rule_token: RuleToken) {
        let rule_token = Arc::<RwLock<RuleToken>>::new(RwLock::<RuleToken>::new(rule_token));
        if self.tokens.is_none() {
            self.tokens = Some(rule_token.clone());
        } else {
            self.tokens_tail.as_mut().unwrap().write().unwrap().next = Some(rule_token.clone());
            rule_token.write().unwrap().prev = self.tokens_tail.clone();
        }

        self.tokens_tail = Some(rule_token);
    }
}

impl RuleToken {
    /// create a rule token
    pub(crate) fn new(
        r#type: TokenType,
        op: OperatorType,
        attr: Option<String>,
        value: String,
        context: (u32, String, String),
    ) -> Result<RuleToken> {
        let mut attr_subst_type = SubstituteType::Invalid;
        let (line_number, rule_file, content) = context;

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
            prev: None,
            next: None,
            line_number,
            rule_file,
            content,
        })
    }

    /// parse strings into a rule token
    pub fn parse_token(
        key: String,
        attr: Option<String>,
        op: String,
        value: String,
        rules: Option<Arc<RwLock<Rules>>>,
        context: (u32, String, String),
    ) -> Result<RuleToken> {
        let mut op = op.parse::<OperatorType>()?;
        let op_is_match = [OperatorType::Match, OperatorType::Nomatch].contains(&op);
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
                    context,
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
                    context,
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
                    context,
                )?)
            }
            "SYMLINK" => {
                if attr.is_some() {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'SYMLINK' can not carry attribute.".location(&context),
                    });
                }
                if op == OperatorType::Remove {
                    return Err(Error::RulesLoadError {
                        msg: "Key 'SYMLINK' can not take remove operator.".location(&context),
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
                        context,
                    )?)
                } else {
                    Ok(RuleToken::new(
                        TokenType::MatchDevlink,
                        op,
                        None,
                        value,
                        context,
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
                        context,
                    )?)
                } else {
                    Ok(RuleToken::new(
                        TokenType::MatchName,
                        op,
                        None,
                        value,
                        context,
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
                        context,
                    )?)
                } else {
                    Ok(RuleToken::new(
                        TokenType::MatchEnv,
                        op,
                        attr,
                        value,
                        context,
                    )?)
                }
            }
            "CONST" => {
                if attr.is_none() || matches!(attr.as_ref().unwrap().as_str(), "arch" | "virt") {
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
                    context,
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
                        context,
                    )?)
                } else {
                    Ok(RuleToken::new(
                        TokenType::MatchTag,
                        op,
                        None,
                        value,
                        context,
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
                    context,
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
                    context,
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
                        context,
                    )?)
                } else {
                    Ok(RuleToken::new(
                        TokenType::MatchAttr,
                        op,
                        attr,
                        value,
                        context,
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
                        context,
                    )?)
                } else {
                    Ok(RuleToken::new(
                        TokenType::MatchAttr,
                        op,
                        attr,
                        value,
                        context,
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
                    context,
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
                    context,
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
                    context,
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
                    context,
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
                    context,
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
                    context,
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
                    context,
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
                        context,
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
                                context,
                            )?)
                        }
                        Err(_) => Ok(RuleToken::new(
                            TokenType::MatchImportProgram,
                            op,
                            attr,
                            value,
                            context,
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
                        context,
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

                    Ok(RuleToken::new(token_type, op, attr, value, context)?)
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
                    context,
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
                        context,
                    )?),
                    "string_escape=replace" => Ok(RuleToken::new(
                        TokenType::AssignOptionsStringEscapeReplace,
                        op,
                        None,
                        "".to_string(),
                        context,
                    )?),
                    "db_persist" => Ok(RuleToken::new(
                        TokenType::AssignOptionsDbPersist,
                        op,
                        None,
                        "".to_string(),
                        context,
                    )?),
                    "watch" => Ok(RuleToken::new(
                        TokenType::AssignOptionsWatch,
                        op,
                        None,
                        "true".to_string(),
                        context,
                    )?),
                    "nowatch" => Ok(RuleToken::new(
                        TokenType::AssignOptionsWatch,
                        op,
                        None,
                        "false".to_string(),
                        context,
                    )?),
                    _ => {
                        if let Some(strip_value) = value.strip_prefix("static_node=") {
                            Ok(RuleToken::new(
                                TokenType::AssignOptionsStaticNode,
                                op,
                                None,
                                strip_value.to_string(),
                                context,
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
                                context,
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
                                context,
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

                if let Some(rules) = rules {
                    /*
                     *  If a legal uid is provided, directly pass the uid to rules executer
                     */
                    if parse_uid(&value).is_ok() {
                        return RuleToken::new(TokenType::AssignOwnerId, op, attr, value, context);
                    }

                    let time = rules.as_ref().read().unwrap().resolve_name_time;
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
                            context,
                        );
                    } else if time != ResolveNameTime::Never {
                        /*
                         *  If the resolve_name_time is not set to 'Never', try to format the value during rules executing.
                         *  Here we only check whether the format of value is legal.
                         */
                        if let Err(e) = check_value_format("OWNER", value.as_str(), true) {
                            log::warn!("{}", e.location(&context));
                        }

                        return RuleToken::new(TokenType::AssignOwner, op, attr, value, context);
                    }
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

                if let Some(rules) = rules {
                    /*
                     *  If a legal gid is provided, directly pass the gid to rules executer
                     */
                    if parse_gid(&value).is_ok() {
                        return RuleToken::new(TokenType::AssignGroupId, op, attr, value, context);
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
                            context,
                        );
                    } else if time != ResolveNameTime::Never {
                        /*
                         * If resolve_name_time is not set to 'Never', try to format the value during rules executing.
                         * Here we only check the format of value is legal.
                         */
                        if let Err(e) = check_value_format("GROUP", value.as_str(), true) {
                            log::warn!("{}", e);
                        }

                        return RuleToken::new(TokenType::AssignGroup, op, attr, value, context);
                    }
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
                        context,
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
                        context,
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
                    context,
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
                        context,
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
                        context,
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

                Ok(RuleToken::new(TokenType::Goto, op, None, value, context)?)
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

                Ok(RuleToken::new(TokenType::Label, op, None, value, context)?)
            }
            _ => Err(Error::RulesLoadError {
                msg: format!("Key '{}' is not supported.", key).location(&context),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::*;
    use log::init_log;
    use log::Level;

    use super::*;
    use std::{fs, path::Path, thread::JoinHandle};

    fn create_test_rules_dir(dir: &'static str) {
        assert!(fs::create_dir(dir).is_ok());
        assert!(fs::write(
            format!("{}/test.rules", dir),
            "ACTION == \"change\", SYMLINK += \"test1\"
ACTION == \"change\", SYMLINK += \"test11\", \\
SYMLINK += \"test111\"
ACTION == \"change\", SYMLINK += \"test1111\", \\
SYMLINK += \"test11111\", \\
SYMLINK += \"test111111\"",
        )
        .is_ok());
    }

    fn clear_test_rules_dir(dir: &'static str) {
        if Path::new(dir).exists() {
            assert!(fs::remove_dir_all(dir).is_ok());
        }
    }

    #[test]
    fn test_rules_new() {
        init_log(
            "test_rules_new",
            Level::Debug,
            vec!["console"],
            "",
            0,
            0,
            false,
        );
        clear_test_rules_dir("test_rules_new");
        create_test_rules_dir("test_rules_new");
        let rules = Rules::load_rules(DEFAULT_RULES_DIRS.to_vec(), ResolveNameTime::Early);
        println!("{}", rules.read().unwrap());
        clear_test_rules_dir("test_rules_new");
    }

    #[test]
    fn test_rules_file() {
        fs::write(
            "test_rules_file.rules",
            "ACTION == \"change\", SYMLINK+=\"test\"\nACTION != \"change\"\n",
        )
        .unwrap();
        RuleFile::load_file("test_rules_file.rules".to_string(), None);
        fs::remove_file("test_rules_file.rules").unwrap();
    }

    #[test]
    fn test_rules_token() {
        assert!(RuleToken::parse_token(
            "ACTION".to_string(),
            None,
            "==".to_string(),
            "add".to_string(),
            None,
            (0, String::default(), String::from("ACTION==\"add\""))
        )
        .is_ok());

        assert!(RuleToken::parse_token(
            "ACTION".to_string(),
            None,
            "!=".to_string(),
            "add".to_string(),
            None,
            (0, String::default(), String::from("ACTION!=\"add\""))
        )
        .is_ok());

        assert!(RuleToken::parse_token(
            "ACTION".to_string(),
            None,
            "*=".to_string(),
            "add".to_string(),
            None,
            (0, String::default(), String::from("ACTION*=\"add\"")),
        )
        .is_err());

        assert!(RuleToken::parse_token(
            "ACTION".to_string(),
            Some("whatever".to_string()),
            "==".to_string(),
            "add".to_string(),
            None,
            (
                0,
                String::default(),
                String::from("ACTION{whatever}==\"add\"")
            ),
        )
        .is_err());
    }

    #[test]
    fn test_rules_token_regex() {
        let t = RuleToken::parse_token(
            "ACTION".to_string(),
            None,
            "==".to_string(),
            "add".to_string(),
            None,
            (0, String::default(), String::from("ACTION==\"add\"")),
        )
        .unwrap();

        println!("{:?}", t);

        let t = RuleToken::parse_token(
            "ACTION".to_string(),
            None,
            "==".to_string(),
            ".?.*".to_string(),
            None,
            (0, String::default(), String::from("ACTION==\".?.*\"")),
        )
        .unwrap();

        println!("{:?}", t);

        let t = RuleToken::parse_token(
            "ACTION".to_string(),
            None,
            "==".to_string(),
            "?*".to_string(),
            None,
            (0, String::default(), String::from("ACTION==\"?*\"")),
        )
        .unwrap();

        println!("{:?}", t);

        let t = RuleToken::parse_token(
            "ACTION".to_string(),
            None,
            "==".to_string(),
            "hello|?*|hello*|3279/tty[0-9]*".to_string(),
            None,
            (
                0,
                String::default(),
                String::from("ACTION==\"hello|?*|hello*|3279/tty[0-9]*\""),
            ),
        )
        .unwrap();

        println!("{:?}", t);

        let t = RuleToken::parse_token(
            "ACTION".to_string(),
            None,
            "==".to_string(),
            String::default(),
            None,
            (0, String::default(), String::from("ACTION==\"\"")),
        )
        .unwrap();

        println!("{:?}", t);

        let t = RuleToken::parse_token(
            "ACTION".to_string(),
            None,
            "==".to_string(),
            "|hello|?*|hello*|3279/tty[0-9]*".to_string(),
            None,
            (
                0,
                String::default(),
                String::from("ACTION==\"|hello|?*|hello*|3279/tty[0-9]*\""),
            ),
        )
        .unwrap();

        println!("{:?}", t);

        let t = RuleToken::parse_token(
            "ATTR".to_string(),
            Some("whatever".to_string()),
            "==".to_string(),
            "hello".to_string(),
            None,
            (
                0,
                String::default(),
                String::from("ACTION{whatever}==\"hello\""),
            ),
        )
        .unwrap();

        println!("{:?}", t);

        let t = RuleToken::parse_token(
            "ATTR".to_string(),
            Some("whatever$".to_string()),
            "==".to_string(),
            "hello".to_string(),
            None,
            (
                0,
                String::default(),
                String::from("ATTR{whatever$}==\"hello\""),
            ),
        )
        .unwrap();

        println!("{:?}", t);

        let t = RuleToken::parse_token(
            "ATTR".to_string(),
            Some("whatever%".to_string()),
            "==".to_string(),
            "hello".to_string(),
            None,
            (
                0,
                String::default(),
                String::from("ATTR{whatever%}==\"hello\""),
            ),
        )
        .unwrap();

        println!("{:?}", t);
    }

    #[test]
    fn test_rules_share_among_threads() {
        create_test_rules_dir("test_rules_share_among_threads");
        let rules = Rules::new(
            vec![
                "test_rules_new_1".to_string(),
                "test_rules_new_2".to_string(),
            ],
            ResolveNameTime::Early,
        );
        let mut handles = Vec::<JoinHandle<()>>::new();
        (0..5).for_each(|i| {
            let rules_clone = rules.clone();
            let handle = std::thread::spawn(move || {
                println!("thread {}", i);
                println!("{}", rules_clone);
            });

            handles.push(handle);
        });

        for thread in handles {
            thread.join().unwrap();
        }

        clear_test_rules_dir("test_rules_share_among_threads");
    }

    #[test]
    #[ignore]
    fn test_resolve_user() {
        let mut rules = Rules::new(vec![], ResolveNameTime::Early);
        assert!(rules.resolve_user("tss").is_ok());
        assert!(rules.resolve_user("root").is_ok());
        assert!(rules.users.contains_key("tss"));
        assert!(rules.users.contains_key("root"));
        assert!(rules.resolve_user("cjy").is_err());
    }
}
