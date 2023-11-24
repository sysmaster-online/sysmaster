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

//! rule loader and executer
//! the implementation of rules has referred to udev for compatibility.
//!

use crate::error::Error;
use bitflags::bitflags;
use nix::unistd::{Group, User};
use std::{
    collections::HashMap,
    fmt::{self, Display},
    str::FromStr,
    sync::{Arc, RwLock, Weak},
};

pub mod exec_mgr;
pub mod exec_unit;
pub(crate) mod node;
pub mod rules_load;

/// encapsulate all rule files
#[derive(Debug, Clone)]
pub struct Rules {
    /// the linked list to contain all rule files
    /// keeps the dictionary order
    files: Arc<RwLock<Option<RuleFile>>>,

    /// current rule file
    files_tail: Arc<RwLock<Option<RuleFile>>>,

    /// directories for searching rule files
    dirs: Vec<String>,

    /// format time
    resolve_name_time: ResolveNameTime,

    /// users declared in rules by 'OWNER'
    users: HashMap<String, User>,
    /// groups declared in rules by 'GROUP'
    groups: HashMap<String, Group>,
}

/// rule file is the basic unit to process the device
#[derive(Debug, Clone)]
pub struct RuleFile {
    /// the name of the rule file
    file_name: String,

    /// the linked list to contain all lines in the rule file
    /// keeps in order of line number
    lines: Arc<RwLock<Option<RuleLine>>>,
    /// current rule line
    lines_tail: Arc<RwLock<Option<RuleLine>>>,

    /// previous rule file
    prev: Arc<RwLock<Option<RuleFile>>>,
    /// next rule file
    next: Arc<RwLock<Option<RuleFile>>>,
}

impl RuleFile {
    #[inline]
    fn get_file_name(&self) -> String {
        self.file_name.clone()
    }
}

/// rule line contains at least a rule token
/// the regular expression pattern is as following:
///     `(<token>\s*,?\s*)+`
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RuleLine {
    /// the content of the rule line
    line_content: String,
    /// the line number in its rule file
    line_number: u32,

    /// line type
    r#type: RuleLineType,

    /// whether this line has label token
    label: Option<String>,
    /// whether this line has goto token
    goto_label: Option<String>,
    /// which line should be went to
    goto_line: Arc<RwLock<Option<RuleLine>>>,

    /// the linked list to contain all tokens in the rule line
    tokens: Arc<RwLock<Option<RuleToken>>>,
    /// current rule token
    tokens_tail: Arc<RwLock<Option<RuleToken>>>,

    /// Pointer to the rule file containing this line.
    /// Although it is optional, it must to be set actually.
    /// Thus the RuleFile can be unwrapped no need to worry.
    rule_file: Weak<RwLock<Option<RuleFile>>>,

    /// previous rule line
    prev: Arc<RwLock<Option<RuleLine>>>,
    /// next rule line
    next: Arc<RwLock<Option<RuleLine>>>,
}

impl RuleLine {
    #[inline]
    fn get_file_name(&self) -> String {
        match self.rule_file.upgrade() {
            Some(arc) => {
                if let Some(f) = arc.read().unwrap().as_ref() {
                    f.get_file_name()
                } else {
                    "".to_string()
                }
            }
            None => {
                log::error!("Rule file is not set.");
                "".to_string()
            }
        }
    }

    #[inline]
    fn get_label(&self) -> Option<String> {
        self.label.clone()
    }

    #[inline]
    fn get_goto_label(&self) -> Option<String> {
        self.goto_label.clone()
    }
}

/// rule token matches the following regular expression pattern:
/// `<key>[{attr}]\s*<op>\s*\"<value>\"`
/// where
///     key: [^={+\-!:\0\s]+
///     attr: [^\{\}]+
///     value: [^\"]+
#[derive(Debug, Clone)]
pub struct RuleToken {
    r#type: TokenType,
    op: OperatorType,
    attr_subst_type: SubstituteType,
    attr: Option<String>,
    value: String,
    prev: Arc<RwLock<Option<RuleToken>>>,
    next: Arc<RwLock<Option<RuleToken>>>,

    rule_line: Weak<RwLock<Option<RuleLine>>>,
}

impl RuleToken {
    #[inline]
    pub(crate) fn is_for_parents(&self) -> bool {
        self.r#type >= TokenType::MatchParentsKernel && self.r#type <= TokenType::MatchParentsTag
    }

    #[inline]
    pub(crate) fn get_line_number(&self) -> u32 {
        match self.rule_line.upgrade() {
            Some(line) => {
                if let Some(l) = line.read().unwrap().as_ref() {
                    l.line_number
                } else {
                    log::error!("Failed to get line number: rule line is not set.");
                    0
                }
            }
            None => {
                log::error!(
                    "Failed to get line number: failed to upgrade rule line weak reference."
                );
                0
            }
        }
    }

    #[inline]
    pub(crate) fn get_file_name(&self) -> String {
        match self.rule_line.upgrade() {
            Some(line) => {
                if let Some(l) = line.read().unwrap().as_ref() {
                    l.get_file_name()
                } else {
                    log::error!("Failed to get file name: rule line is not set for the token.");
                    "".to_string()
                }
            }
            None => {
                log::error!("Failed to get file name: failed to upgrade rule line weak reference.");
                "".to_string()
            }
        }
    }

    #[inline]
    pub(crate) fn get_token_attribute(&self) -> Option<String> {
        self.attr.clone()
    }

    #[inline]
    pub(crate) fn get_token_value(&self) -> String {
        self.value.clone()
    }

    #[inline]
    pub(crate) fn get_token_content(&self) -> String {
        if let Some(attribute) = self.get_token_attribute() {
            format!(
                "{}{{{}}}{}\"{}\"",
                self.r#type, attribute, self.op, self.value
            )
        } else {
            format!("{}{}\"{}\"", self.r#type, self.op, self.value)
        }
    }
}

/// token type
#[allow(missing_docs, dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub(crate) enum TokenType {
    // the left value should take match == or unmatch != operator
    // the matching pattern is generated during loading rules
    MatchAction,
    MatchDevpath,
    MatchKernel,
    MatchDevlink,
    MatchName,
    MatchEnv,
    MatchConst,
    MatchTag,
    MatchSubsystem,
    MatchDriver,
    MatchAttr,
    MatchSysctl,

    // matches parents
    MatchParentsKernel,
    MatchParentsSubsystem,
    MatchParentsDriver,
    MatchParentsAttr,
    MatchParentsTag,

    MatchResult,

    // no need to generate matching pattern
    MatchTest,
    MatchProgram,
    MatchImportFile,
    MatchImportProgram,
    MatchImportBuiltin,
    MatchImportDb,
    MatchImportCmdline,

    // the matching pattern is generated during applying rules
    MatchImportParent,

    // the left value should take assign = += -= := operators
    AssignOptionsStringEscapeNone,
    AssignOptionsStringEscapeReplace,
    AssignOptionsDbPersist,
    AssignOptionsWatch,
    AssignOptionsDevlinkPriority,
    AssignOptionsLogLevel,
    AssignOwner,
    AssignGroup,
    AssignMode,
    AssignOwnerId,
    AssignGroupId,
    AssignModeId,
    AssignTag,
    AssignOptionsStaticNode,
    AssignSeclabel,
    AssignEnv,
    AssignName,

    /// key = "SYMLINK", operator = "=|+=|-=|:="
    AssignDevlink,
    AssignAttr,
    AssignSysctl,
    AssignRunBuiltin,
    AssignRunProgram,

    /// the rule line can contain only one token if the token is goto or label
    Goto,
    Label,
}

impl Display for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let t = match *self {
            Self::MatchAction => "ACTION",
            Self::MatchDevpath => "DEVPATH",
            Self::MatchKernel => "KERNEL",
            Self::MatchDevlink | Self::AssignDevlink => "SYMLINK",
            Self::MatchName | Self::AssignName => "NAME",
            Self::MatchEnv | Self::AssignEnv => "ENV",
            Self::MatchConst => "CONST",
            Self::MatchTag | Self::AssignTag => "TAG",
            Self::MatchSubsystem => "SUBSYSTEM",
            Self::MatchDriver => "DRIVER",
            Self::MatchAttr | Self::AssignAttr => "ATTR",
            Self::MatchSysctl | Self::AssignSysctl => "SYSCTL",
            Self::MatchParentsKernel => "KERNELS",
            Self::MatchParentsSubsystem => "SUBSYSTEMS",
            Self::MatchParentsDriver => "DRIVERS",
            Self::MatchParentsAttr => "ATTRS",
            Self::MatchParentsTag => "TAGS",
            Self::MatchResult => "RESULT",
            Self::MatchTest => "TEST",
            Self::MatchProgram => "PROGRAM",
            Self::MatchImportFile
            | Self::MatchImportProgram
            | Self::MatchImportBuiltin
            | Self::MatchImportDb
            | Self::MatchImportCmdline
            | Self::MatchImportParent => "IMPORT",
            Self::AssignOptionsStringEscapeNone
            | Self::AssignOptionsStringEscapeReplace
            | Self::AssignOptionsDbPersist
            | Self::AssignOptionsWatch
            | Self::AssignOptionsDevlinkPriority
            | Self::AssignOptionsLogLevel
            | Self::AssignOptionsStaticNode => "OPTIONS",
            Self::AssignOwner | Self::AssignOwnerId => "OWNER",
            Self::AssignGroup | Self::AssignGroupId => "GROUP",
            Self::AssignMode | Self::AssignModeId => "MODE",
            Self::AssignSeclabel => "SECLABEL",
            Self::AssignRunBuiltin | Self::AssignRunProgram => "RUN",
            Self::Goto => "GOTO",
            Self::Label => "LABEL",
        };

        write!(f, "{}", t)
    }
}

/// operator type
#[derive(PartialEq, Eq, Debug, Clone)]
pub(crate) enum OperatorType {
    /// ==
    Match,
    /// !=
    Nomatch,
    /// +=
    Add,
    /// -=
    Remove,
    /// =
    Assign,
    /// :=
    AssignFinal,
}

impl FromStr for OperatorType {
    type Err = Error;

    fn from_str(s: &str) -> Result<OperatorType, Self::Err> {
        match s {
            "==" => Ok(OperatorType::Match),
            "!=" => Ok(OperatorType::Nomatch),
            "=" => Ok(OperatorType::Assign),
            "+=" => Ok(OperatorType::Add),
            "-=" => Ok(OperatorType::Remove),
            ":=" => Ok(OperatorType::AssignFinal),
            _ => Err(Error::RulesLoadError {
                msg: "Invalid operator".to_string(),
            }),
        }
    }
}

impl Display for OperatorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                OperatorType::Match => "==",
                OperatorType::Nomatch => "!=",
                OperatorType::Add => "+=",
                OperatorType::Remove => "-=",
                OperatorType::AssignFinal => ":=",
                OperatorType::Assign => "=",
            }
        )
    }
}

impl Display for Rules {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = String::new();
        // write!(f, "{}", s)
        for file in self.iter() {
            if let Some(f) = file.read().unwrap().as_ref() {
                s = format!("{}\n{}\n", s, f);
            }
        }
        write!(f, "{}", s)
    }
}

/// iterate over all rule files
pub struct RulesIter {
    current_file: Arc<RwLock<Option<RuleFile>>>,
}

impl Iterator for RulesIter {
    /// iterate over each rule file in the rules
    type Item = Arc<RwLock<Option<RuleFile>>>;

    /// iterate over the rule file list
    fn next(&mut self) -> Option<Self::Item> {
        let ret = self.current_file.clone();
        let next = match self.current_file.read().unwrap().as_ref() {
            Some(file) => file.next.clone(),
            None => return None,
        };
        self.current_file = next;
        Some(ret)
    }
}

impl Rules {
    /// return the iterator wrapper
    pub fn iter(&self) -> RulesIter {
        let first_file = self.files.clone();
        RulesIter {
            current_file: first_file,
        }
    }
}

impl Display for RuleFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = format!("File: {}", self.get_file_name());
        for line in self.iter() {
            if let Some(l) = line.read().unwrap().as_ref() {
                s.push_str(&format!("\n{}", l));
            }
        }

        write!(f, "{}", s)
    }
}

/// iterator over lines in the rule file
pub struct RuleFileIter {
    current_line: Arc<RwLock<Option<RuleLine>>>,
}

impl Iterator for RuleFileIter {
    /// iterate over each rule file in the rules
    type Item = Arc<RwLock<Option<RuleLine>>>;

    /// iterate over the rule file list
    fn next(&mut self) -> Option<Self::Item> {
        let ret = self.current_line.clone();
        let next = match self.current_line.read().unwrap().as_ref() {
            Some(line) => line.next.clone(),
            None => return None,
        };
        self.current_line = next;
        Some(ret)
    }
}

impl RuleFile {
    /// return the iterator wrapper
    pub fn iter(&self) -> RuleFileIter {
        let first_line = self.lines.clone();
        RuleFileIter {
            current_line: first_line,
        }
    }
}

impl Display for RuleLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = format!(
            "{}:{}:  {}",
            self.rule_file
                .upgrade()
                .unwrap()
                .read()
                .unwrap()
                .as_ref()
                .unwrap()
                .get_file_name(),
            self.line_number,
            self.line_content
        );
        for token in self.iter() {
            if let Some(t) = token.read().unwrap().as_ref() {
                s.push_str(&format!("\n{}", t));
            }
        }
        write!(f, "{}", s)
    }
}

/// iterator over tokens in the rule line
pub struct RuleLineIter {
    current_token: Arc<RwLock<Option<RuleToken>>>,
}

impl Iterator for RuleLineIter {
    /// iterate over each rule file in the rules
    type Item = Arc<RwLock<Option<RuleToken>>>;

    /// iterate over the rule file list
    fn next(&mut self) -> Option<Self::Item> {
        let ret = self.current_token.clone();
        let next = match self.current_token.read().unwrap().as_ref() {
            Some(token) => token.next.clone(),
            None => return None,
        };
        self.current_token = next;
        Some(ret)
    }
}

impl RuleLine {
    /// return the iterator wrapper
    pub fn iter(&self) -> RuleLineIter {
        let first_token = self.tokens.clone();
        RuleLineIter {
            current_token: first_token,
        }
    }
}

impl Display for RuleToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // let s = String::new();
        write!(f, "{}", self.get_token_content())
    }
}

bitflags! {
    /// value matching type
    pub(crate) struct RuleLineType: u8 {
        /// initial type
        const INITIAL = 0;
        /// has NAME=
        const HAS_NAME = 1<<0;
        /// has SYMLINK=, OWNER=, GROUP= or MODE=
        const HAS_DEVLINK = 1<<1;
        /// has OPTIONS=static_node
        const HAS_STATIC_NODE = 1<<2;
        /// has GOTO=
        const HAS_GOTO = 1<<3;
        /// has LABEL=
        const HAS_LABEL = 1<<4;
        /// has other Assign* or MatchImport* tokens
        const UPDATE_SOMETHING = 1<<5;
    }
}

/// match type
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub(crate) enum MatchType {
    Pattern,
    Subsystem,
    Invalid,
}

/// substitute string
/// can not use multiple kinds of substitution formatter
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum SubstituteType {
    /// no substitution
    Plain,
    /// contain $ or %
    Format,
    /// [<SBUSTYEM>|<KERNEL>]<attribute>
    Subsys,
    /// invalid
    Invalid,
}

impl FromStr for SubstituteType {
    type Err = Error;

    fn from_str(s: &str) -> Result<SubstituteType, Self::Err> {
        if s.is_empty() {
            return Ok(SubstituteType::Plain);
        }

        if s.starts_with('[') {
            return Ok(SubstituteType::Subsys);
        }

        if s.contains(|c| ['%', '$'].contains(&c)) {
            return Ok(SubstituteType::Format);
        }

        Ok(SubstituteType::Plain)
    }
}

/// the time when to format a string
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[allow(dead_code)]
pub(crate) enum ResolveNameTime {
    /// never format a string
    Never,
    /// do not format the string until rule execution
    Late,
    /// format the string when loading the rules
    Early,
}

/// formatter substitution type
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Copy, Clone)]
pub(crate) enum FormatSubstitutionType {
    Devnode,
    Attr,
    Env,
    Kernel,
    KernelNumber,
    Driver,
    Devpath,
    Id,
    Major,
    Minor,
    Result,
    Parent,
    Name,
    Links,
    Root,
    Sys,
    Invalid,
}

pub(crate) const FORMAT_SUBST_TABLE: [(&str, &str, FormatSubstitutionType); 18] = [
    ("devnode", "N", FormatSubstitutionType::Devnode),
    ("tempnode", "N", FormatSubstitutionType::Devnode),
    ("attr", "s", FormatSubstitutionType::Attr),
    ("sysfs", "s", FormatSubstitutionType::Attr),
    ("env", "E", FormatSubstitutionType::Env),
    ("kernel", "k", FormatSubstitutionType::Kernel),
    ("number", "n", FormatSubstitutionType::KernelNumber),
    ("driver", "d", FormatSubstitutionType::Driver),
    ("devpath", "p", FormatSubstitutionType::Devpath),
    ("id", "b", FormatSubstitutionType::Id),
    ("major", "M", FormatSubstitutionType::Major),
    ("minor", "m", FormatSubstitutionType::Minor),
    ("result", "c", FormatSubstitutionType::Result),
    ("parent", "P", FormatSubstitutionType::Parent),
    ("name", "D", FormatSubstitutionType::Name),
    ("links", "L", FormatSubstitutionType::Links),
    ("root", "r", FormatSubstitutionType::Root),
    ("sys", "S", FormatSubstitutionType::Sys),
];

impl FromStr for FormatSubstitutionType {
    type Err = Error;

    fn from_str(s: &str) -> Result<FormatSubstitutionType, Self::Err> {
        match s {
            "devnode" | "N" | "tempnode" => Ok(FormatSubstitutionType::Devnode),
            "attr" | "sysfs" | "s" => Ok(FormatSubstitutionType::Attr),
            "env" | "E" => Ok(FormatSubstitutionType::Env),
            "kernel" | "k" => Ok(FormatSubstitutionType::Kernel),
            "number" | "n" => Ok(FormatSubstitutionType::KernelNumber),
            "driver" | "d" => Ok(FormatSubstitutionType::Driver),
            "devpath" | "p" => Ok(FormatSubstitutionType::Devpath),
            "id" | "b" => Ok(FormatSubstitutionType::Id),
            "major" | "M" => Ok(FormatSubstitutionType::Major),
            "minor" | "m" => Ok(FormatSubstitutionType::Minor),
            "result" | "c" => Ok(FormatSubstitutionType::Result),
            "parent" | "P" => Ok(FormatSubstitutionType::Parent),
            "name" | "D" => Ok(FormatSubstitutionType::Name),
            "links" | "L" => Ok(FormatSubstitutionType::Links),
            "root" | "r" => Ok(FormatSubstitutionType::Root),
            "sys" | "S" => Ok(FormatSubstitutionType::Sys),
            _ => Err(Error::RulesLoadError {
                msg: "Invalid substitute formatter".to_string(),
            }),
        }
    }
}

impl Display for FormatSubstitutionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            FormatSubstitutionType::Devnode => "devnode",
            FormatSubstitutionType::Attr => "attr",
            FormatSubstitutionType::Env => "env",
            FormatSubstitutionType::Kernel => "kernel",
            FormatSubstitutionType::KernelNumber => "kernel number",
            FormatSubstitutionType::Driver => "driver",
            FormatSubstitutionType::Devpath => "devpath",
            FormatSubstitutionType::Id => "id",
            FormatSubstitutionType::Major => "major",
            FormatSubstitutionType::Minor => "minor",
            FormatSubstitutionType::Result => "result",
            FormatSubstitutionType::Parent => "parent",
            FormatSubstitutionType::Name => "name",
            FormatSubstitutionType::Links => "links",
            FormatSubstitutionType::Root => "root",
            FormatSubstitutionType::Sys => "sys",
            FormatSubstitutionType::Invalid => "invalid substitute formatter",
        };

        write!(f, "{}", s)
    }
}

impl Default for FormatSubstitutionType {
    fn default() -> Self {
        Self::Invalid
    }
}

/// escape type of execute unit
#[derive(PartialEq, Eq, Copy, Clone)]
pub(crate) enum EscapeType {
    Unset,
    None,
    Replace,
}

#[cfg(test)]
mod test {
    use log::{init_log, Level};

    use super::*;
    use crate::rules::rules_load::tests::create_tmp_file;

    #[test]
    fn test_rules_display() {
        init_log(
            "test_rules_display",
            Level::Debug,
            vec!["console"],
            "",
            0,
            0,
            false,
        );

        create_tmp_file(
            "/tmp/test_rules_display/rules.d",
            "00-test.rules",
            "
ACTION==\"change\"
DEVPATH==\"xxx\"
KERNEL==\"xxx\"
SYMLINK==\"xxx\"
SYMLINK+=\"xxx\"
NAME==\"xxx\"
NAME=\"x\"
ENV{x}=\"x\"
ENV{x}==\"x\"
CONST{virt}==\"x\"
TAG+=\"x\"
TAG==\"x\"
SUBSYSTEM==\"x\"
DRIVER==\"x\"
ATTR{x}==\"x\"
ATTR{x}=\"x\"
SYSCTL{x}==\"x\"
SYSCTL{x}=\"x\"
KERNELS==\"x\"
SUBSYSTEMS==\"x\"
DRIVERS==\"x\"
ATTRS{x}==\"x\"
TAGS==\"x\"
RESULT==\"x\"
TEST==\"x\"
PROGRAM==\"x\"
IMPORT{file}==\"x\"
IMPORT{program}==\"echo hello\"
IMPORT{builtin}==\"path_id\"
IMPORT{db}==\"x\"
IMPORT{cmdline}==\"x\"
IMPORT{parent}==\"x\"
OPTIONS+=\"string_escape=none\"
OPTIONS+=\"string_escape=replace\"
OPTIONS+=\"db_persist\"
OPTIONS+=\"watch\"
OPTIONS+=\"link_priority=10\"
OPTIONS+=\"log_level=1\"
OPTIONS+=\"static_node=/dev/sda\"
SECLABEL{x}+=\"x\"
RUN{builtin}+=\"path_id\"
RUN{program}+=\"x\"
GOTO=\"x\"
LABEL=\"x\"
",
            true,
        );

        let rule = Arc::new(RwLock::new(Rules::new(
            vec!["/tmp/test_rules_display/rules.d".to_string()],
            ResolveNameTime::Late,
        )));

        Rules::parse_rules(rule.clone());

        println!("{}", rule.read().unwrap());

        std::fs::remove_dir_all("/tmp/test_rules_display").unwrap();
    }
}
