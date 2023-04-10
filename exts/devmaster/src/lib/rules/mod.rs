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
//!

use crate::error::Error;
use std::{
    cell::RefCell,
    fmt::{self, Display},
    rc::{Rc, Weak},
    str::FromStr,
};

pub mod rule_load;

/// encapsulate all rule files
pub struct Rules {
    /// the linked list to contain all rule files
    /// keeps the dictionary order
    files: Option<Rc<RefCell<RuleFile>>>,

    /// current rule file
    current_file: Option<Rc<RefCell<RuleFile>>>,
}

/// rule file is the basic unit to process the device
pub struct RuleFile {
    /// the name of the rule file
    file_name: String,

    /// the linked list to contain all lines in the rule file
    /// keeps in order of line number
    lines: Option<Rc<RefCell<RuleLine>>>,
    /// current rule line
    current_line: Option<Rc<RefCell<RuleLine>>>,

    /// previous rule file
    prev: Option<Rc<RefCell<RuleFile>>>,
    /// next rule file
    next: Option<Rc<RefCell<RuleFile>>>,
}

/// rule line contains at least a rule token
/// the regex is as following:
///     (<token>\s*,?\s*)+
pub struct RuleLine {
    /// the content of the rule line
    line: String,
    /// the line number in its rule file
    line_number: u32,

    /// the linked list to contain all tokens in the rule line
    tokens: Option<Rc<RefCell<RuleToken>>>,
    /// current rule token
    current_token: Option<Rc<RefCell<RuleToken>>>,

    /// the rule file to contain this line
    file: Weak<RefCell<RuleFile>>,

    /// previous rule line
    prev: Option<Rc<RefCell<RuleLine>>>,
    /// next rule line
    next: Option<Rc<RefCell<RuleLine>>>,
}

/// rule token matches regex:
/// <key>[{attr}]\s*<op>\s*\"<value>\"
/// where
///     key: [^={+\-!:\0\s]+
///     attr: [^\{\}]+
///     value: [^\"]+
pub struct RuleToken {
    r#type: TokenType,
    op: OperatorType,
    // match_type: MatchType,
    // attr_subst_type: SubstituteType,
    attr: Option<String>,
    value: String,
    prev: Option<Rc<RefCell<RuleToken>>>,
    next: Option<Rc<RefCell<RuleToken>>>,
}

/// token type
#[allow(missing_docs, dead_code)]
#[derive(Debug)]
pub(crate) enum TokenType {
    // the left value should take match == or unmatch != operator
    /// key = "ACTION", operator = "==|!="
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

    //
    MatchTest,
    MatchProgram,
    MatchImportFile,
    MatchImportProgram,
    MatchImportBuiltin,
    MatchImportDb,
    MatchImportCmdline,
    MatchImportParent,
    MatchResult,

    // the left value should take assign = += -= := operators
    AssignOptionsStringEscapeNone,
    AssignOptionsStringEscapeReplace,
    AssignOptionsDbPersist,
    AssignOptionsInotifyWatch,
    AssignOptionsDevlinkPriority,
    AssignOptionsLogLevel,
    AssignOwner,
    AssignGroup,
    AssignMode,
    AssignOwnerId,
    AssignGroupId,
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

/// operator type
#[derive(PartialEq, Eq, Debug)]
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
            _ => Err(Error::RulesLoaderError {
                msg: "Invalid operator",
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
            s = format!("{}\n{}\n", s, file.as_ref().borrow());
        }
        write!(f, "{}", s)
    }
}

/// iterate over all rule files
pub struct RulesIter {
    current_file: Option<Rc<RefCell<RuleFile>>>,
}

impl Iterator for RulesIter {
    /// iterate over each rule file in the rules
    type Item = Rc<RefCell<RuleFile>>;

    /// iterate over the rule file list
    fn next(&mut self) -> Option<Self::Item> {
        let ret = self.current_file.clone();
        self.current_file = match self.current_file.clone() {
            Some(file) => file.borrow_mut().next.clone(),
            None => None,
        };
        ret
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
        let mut s = format!("File: {}", self.file_name);
        for line in self.iter() {
            s.push_str(format!("\n{}", line.as_ref().borrow()).as_str());
        }

        write!(f, "{}", s)
    }
}

/// iterator over lines in the rule file
pub struct RuleFileIter {
    current_line: Option<Rc<RefCell<RuleLine>>>,
}

impl Iterator for RuleFileIter {
    /// iterate over each rule file in the rules
    type Item = Rc<RefCell<RuleLine>>;

    /// iterate over the rule file list
    fn next(&mut self) -> Option<Self::Item> {
        let ret = self.current_line.clone();
        self.current_line = match self.current_line.clone() {
            Some(file) => file.borrow_mut().next.clone(),
            None => None,
        };
        ret
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
            self.file.upgrade().unwrap().as_ref().borrow().file_name,
            self.line_number,
            self.line
        );
        for token in self.iter() {
            s.push_str(format!("\n{}", token.as_ref().borrow()).as_str());
        }
        write!(f, "{}", s)
    }
}

/// iterator over tokens in the rule line
pub struct RuleLineIter {
    current_token: Option<Rc<RefCell<RuleToken>>>,
}

impl Iterator for RuleLineIter {
    /// iterate over each rule file in the rules
    type Item = Rc<RefCell<RuleToken>>;

    /// iterate over the rule file list
    fn next(&mut self) -> Option<Self::Item> {
        let ret = self.current_token.clone();
        self.current_token = match self.current_token.clone() {
            Some(file) => file.borrow_mut().next.clone(),
            None => None,
        };
        ret
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
        write!(
            f,
            "Token: {:?} {:?} {:?} {}",
            self.r#type, self.attr, self.op, self.value
        )
    }
}

// bitflags! {
//     /// value matching type
//     pub(crate) struct MatchType: u8 {
//         /// match empty string
//         const EMPTY = 1<<0;
//         /// use shell glob parttern to match
//         const PATTERN = 1<<1;
//         /// match "subsystem", "bus", or "class"
//         const SUBSYSTEM = 1<<2;
//     }
// }

// /// substitute string
// pub(crate) enum SubstituteType {
//     /// no substitution
//     Plain,
//     /// contain $ or %
//     Format,
//     /// [<SBUSTYEM>|<KERNEL>]<attribute>
//     Subsys,
// }
