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

//! the process unit to apply rules on device uevent in worker thread
//!

use super::{OperatorType, RuleFile, RuleLine, RuleToken, Rules, TokenType::*};
use crate::error::{Error, Result};
use device::{Device, DeviceAction};
use std::{
    borrow::BorrowMut,
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
};

use crate::device_trace;
use crate::{execute_err, execute_none};
use nix::errno::Errno;

/// the process unit on device uevent
#[allow(missing_docs)]
struct ExecuteUnit {
    device: Arc<Mutex<Device>>,
    parent: Option<Arc<Mutex<Device>>>,
    // device_db_clone: Option<Device>,
    name: String,
    // program_result: String,
    // mode: mode_t,
    // uid: uid_t,
    // gid: gid_t,
    // seclabel_list: HashMap<String, String>,
    // run_list: HashMap<String, String>,
    // exec_delay_usec: useconds_t,
    // birth_sec: useconds_t,
    // rtnl: Option<Rc<RefCell<Netlink>>>,
    // builtin_run: u32,
    // builtin_ret: u32,
    // escape_type: RuleEscapeType,
    // inotify_watch: bool,
    // inotify_watch_final: bool,
    // group_final: bool,
    // owner_final: bool,
    // mode_final: bool,
    // name_final: bool,
    // devlink_final: bool,
    // run_final: bool,
}

impl ExecuteUnit {
    pub fn new(device: Arc<Mutex<Device>>) -> ExecuteUnit {
        // let mut unit = ProcessUnit::default();
        // unit.device = device;
        // unit
        ExecuteUnit {
            device,
            parent: None,
            // device_db_clone: None,
            name: String::new(),
            // program_result: (),
            // mode: (),
            // uid: (),
            // gid: (),
            // seclabel_list: (),
            // run_list: (),
            // exec_delay_usec: (),
            // birth_sec: (),
            // rtnl: (),
            // builtin_run: (),
            // builtin_ret: (),
            // inotify_watch: (),
            // inotify_watch_final: (),
            // group_final: (),
            // owner_final: (),
            // mode_final: (),
            // name_final: (),
            // devlink_final: (),
            // run_final: (),
        }
    }
}

/// manage processing units
pub struct ExecuteManager {
    rules: Arc<RwLock<Rules>>,

    current_rule_file: Option<Arc<RwLock<RuleFile>>>,
    current_rule_line: Option<Arc<RwLock<RuleLine>>>,
    current_rule_token: Option<Arc<RwLock<RuleToken>>>,

    current_unit: Option<ExecuteUnit>,

    properties: HashMap<String, String>,
}

impl ExecuteManager {
    /// create a execute manager object
    pub fn new(rules: Arc<RwLock<Rules>>) -> ExecuteManager {
        ExecuteManager {
            rules,
            current_rule_file: None,
            current_rule_line: None,
            current_rule_token: None,
            current_unit: None,
            properties: HashMap::new(),
        }
    }

    /// process a device object
    pub fn process_device(&mut self, device: Arc<Mutex<Device>>) -> Result<()> {
        log::debug!(
            "Processing device {}",
            device.as_ref().lock().unwrap().devpath
        );

        self.current_unit = Some(ExecuteUnit::new(device));
        // lock whole disk: todo

        // mark block device read only: todo

        self.execute_rules()?;

        self.execute_run()?;

        // update rtnl: todo

        // begin inotify watch: todo

        self.current_unit = None;

        Ok(())
    }

    /// execute rules
    pub(crate) fn execute_rules(&mut self) -> Result<()> {
        let unit = self.current_unit.as_mut().unwrap();

        let action = unit.device.as_ref().lock().unwrap().action;

        println!("{}", action);

        if action == DeviceAction::Remove {
            return self.execute_rules_on_remove();
        }

        // inotify watch end: todo

        // clone device with db: todo

        // copy all tags to device with db: todo

        // add property to device with db: todo

        self.apply_rules()?;

        // rename netif: todo

        // update devnode: todo

        // preserve old, or get new initialization timestamp: todo

        // write database file: todo

        Ok(())
    }

    /// execute rules on remove uevent
    pub(crate) fn execute_rules_on_remove(&mut self) -> Result<()> {
        todo!();
    }

    /// apply rules on device
    pub(crate) fn apply_rules(&mut self) -> Result<()> {
        self.current_rule_file = self.rules.as_ref().read().unwrap().files.clone();

        loop {
            let next_file = self
                .current_rule_file
                .clone()
                .unwrap()
                .as_ref()
                .read()
                .unwrap()
                .next
                .clone();

            self.apply_rule_file()?;

            self.current_rule_file = next_file;
            if self.current_rule_file.is_none() {
                break;
            }
        }

        Ok(())
    }

    /// apply rule file on device
    pub(crate) fn apply_rule_file(&mut self) -> Result<()> {
        self.current_rule_line = self
            .current_rule_file
            .clone()
            .unwrap()
            .as_ref()
            .read()
            .unwrap()
            .lines
            .clone();

        loop {
            let next_line = self.apply_rule_line()?;

            self.current_rule_line = next_line;
            if self.current_rule_line.is_none() {
                break;
            }
        }
        Ok(())
    }

    /// apply rule line on device
    /// normally return the next rule line after current line
    /// if current line has goto label, use the line with the target label as the next line
    pub(crate) fn apply_rule_line(&mut self) -> Result<Option<Arc<RwLock<RuleLine>>>> {
        self.current_rule_token = match self.current_rule_line.clone() {
            Some(line) => line.as_ref().read().unwrap().tokens.clone(),
            None => return Ok(None),
        };

        device_trace!(
            "Apply Rule Line:",
            self.current_unit
                .as_ref()
                .unwrap()
                .device
                .as_ref()
                .lock()
                .unwrap(),
            self.get_current_rule_file(),
            self.get_current_line_number()
        );

        // only apply rule token on parent device once
        // that means if some a parent device matches the token, do not match any parent tokens in the following
        let mut parents_done = false;

        for token in RuleToken::iter(self.current_rule_token.clone()) {
            self.current_rule_token = Some(token.clone());

            device_trace!(
                "Apply Rule Token:",
                self.current_unit
                    .as_ref()
                    .unwrap()
                    .device
                    .as_ref()
                    .lock()
                    .unwrap(),
                self.get_current_rule_file(),
                self.get_current_line_number(),
                token.as_ref().read().unwrap()
            );

            if self
                .current_rule_token
                .clone()
                .unwrap()
                .as_ref()
                .read()
                .unwrap()
                .is_for_parents()
            {
                if parents_done {
                    continue;
                }
                if !self.apply_rule_token_on_parent()? {
                    return Ok(self
                        .current_rule_line
                        .clone()
                        .unwrap()
                        .as_ref()
                        .read()
                        .unwrap()
                        .next
                        .clone());
                }

                parents_done = true;
                continue;
            }

            if !self.apply_rule_token(self.current_unit.as_ref().unwrap().device.clone())? {
                // if current rule token does not match, abort applying the rest tokens in this line
                return Ok(self
                    .current_rule_line
                    .clone()
                    .unwrap()
                    .as_ref()
                    .read()
                    .unwrap()
                    .next
                    .clone());
            }
        }

        let goto_line = self
            .current_rule_line
            .clone()
            .unwrap()
            .as_ref()
            .read()
            .unwrap()
            .goto_line
            .clone();

        match goto_line {
            Some(line) => Ok(Some(line)),
            None => Ok(self
                .current_rule_line
                .clone()
                .unwrap()
                .as_ref()
                .read()
                .unwrap()
                .next
                .clone()),
        }
    }

    /// apply rule token on device
    pub(crate) fn apply_rule_token(&mut self, device: Arc<Mutex<Device>>) -> Result<bool> {
        let token_type = self
            .current_rule_token
            .clone()
            .unwrap()
            .as_ref()
            .read()
            .unwrap()
            .r#type;

        let mut device = device.as_ref().lock().unwrap();

        let token = self
            .current_rule_token
            .as_ref()
            .unwrap()
            .as_ref()
            .read()
            .unwrap();

        match token_type {
            MatchAction => {
                let action = execute_err!(device.get_action(), "MatchAction")?;

                Ok(token.pattern_match(&action.to_string()))
            }
            MatchDevpath => {
                let devpath = execute_none!(device.get_devpath(), "MatchDevpath", "DEVPATH")?;

                Ok(token.pattern_match(&devpath.to_string()))
            }
            MatchKernel | MatchParentsKernel => {
                let sysname = execute_none!(
                    device.get_sysname(),
                    "MatchKernel|MatchParentsKernel",
                    "SYSNAME"
                )?;

                Ok(token.pattern_match(&sysname.to_string()))
            }
            MatchDevlink => {
                for devlink in device.devlinks.iter() {
                    if token.pattern_match(devlink) {
                        return Ok(token.op == OperatorType::Match);
                    }
                }

                Ok(token.op == OperatorType::Nomatch)
            }
            MatchName => {
                let name = self.current_unit.as_ref().unwrap().name.clone();

                Ok(token.pattern_match(&name))
            }
            MatchEnv => {
                let value = match device.get_property_value(token.attr.clone().unwrap()) {
                    Ok(v) => v,
                    Err(e) => {
                        if e.get_errno() != Errno::ENOENT {
                            return Err(Error::RulesExecuteError {
                                msg: format!("Apply 'MatchEnv' error: {}", e),
                                errno: e.get_errno(),
                            });
                        }

                        self.properties
                            .get(&token.attr.clone().unwrap())
                            .unwrap_or(&"".to_string())
                            .to_string()
                    }
                };

                Ok(token.pattern_match(&value))
            }
            MatchConst => {
                todo!()
            }
            MatchTag | MatchParentsTag => {
                for tag in device.current_tags.iter() {
                    if token.pattern_match(tag) {
                        return Ok(token.op == OperatorType::Match);
                    }
                }

                Ok(token.op == OperatorType::Nomatch)
            }
            MatchSubsystem => {
                todo!()
            }
            AssignDevlink => {
                println!("\x1b[31mHello world!\x1b[0m");
                Ok(true)
            }
            _ => {
                println!("cjy");
                todo!();
            }
        }
    }

    /// apply rule token on the parent device
    pub(crate) fn apply_rule_token_on_parent(&mut self) -> Result<bool> {
        self.current_unit.as_mut().unwrap().borrow_mut().parent = Some(
            self.current_unit
                .as_ref()
                .unwrap()
                .borrow_mut()
                .device
                .clone(),
        );

        let head = self.current_rule_token.clone();
        let mut match_rst = true;

        loop {
            // udev try to traverse the following parent tokens
            // this seems useless and redundant
            for token in RuleToken::iter(head.clone()) {
                if !token.as_ref().read().unwrap().is_for_parents() {
                    return Ok(true);
                }

                self.current_rule_token = Some(token);
                if !self
                    .apply_rule_token(self.current_unit.as_ref().unwrap().parent.clone().unwrap())?
                {
                    match_rst = false;
                    break;
                }
            }

            if match_rst {
                return Ok(true);
            }

            let tmp = self.current_unit.as_ref().unwrap().parent.clone().unwrap();
            match tmp.as_ref().lock().unwrap().get_parent() {
                Ok(d) => {
                    self.current_unit.as_mut().unwrap().borrow_mut().parent = Some(d);
                }
                Err(e) => {
                    if e.get_errno() != Errno::ENOENT {
                        return Err(Error::RulesExecuteError {
                            msg: format!("failed to get parent: ({})", e),
                            errno: e.get_errno(),
                        });
                    }

                    return Ok(false);
                }
            };
        }
    }

    /// execute run
    pub(crate) fn execute_run(&mut self) -> Result<()> {
        Ok(())
    }

    /// get the current rule file name
    pub(crate) fn get_current_rule_file(&self) -> String {
        self.current_rule_file
            .as_ref()
            .unwrap()
            .read()
            .unwrap()
            .file_name
            .clone()
    }

    /// get the current rule line number
    pub(crate) fn get_current_line_number(&self) -> u32 {
        self.current_rule_line
            .as_ref()
            .unwrap()
            .read()
            .unwrap()
            .line_number
    }
}

impl RuleToken {
    pub(crate) fn pattern_match(&self, s: &String) -> bool {
        for regex in self.value_regex.iter() {
            if regex.is_match(&s) {
                return true;
            }
        }
        false
    }
}

/// translate execution error from downside call chain
#[macro_export]
macro_rules! execute_err {
    ($e:expr, $k:expr) => {
        $e.map_err(|err| Error::RulesExecuteError {
            msg: format!("Apply '{}' error: {}", $k, err),
            errno: err.get_errno(),
        })
    };
}

/// translate execution error on none return from downside call chain
#[macro_export]
macro_rules! execute_none {
    ($e:expr, $k:expr, $v:expr) => {
        if $e.is_none() {
            Err(Error::RulesExecuteError {
                msg: format!("Apply '{}' error: have no {}", $k, $v),
                errno: Errno::EINVAL,
            })
        } else {
            Ok($e.unwrap())
        }
    };
}

/// tokens iterator
struct RuleTokenIter {
    token: Option<Arc<RwLock<RuleToken>>>,
}

impl RuleToken {
    fn iter(token: Option<Arc<RwLock<RuleToken>>>) -> RuleTokenIter {
        RuleTokenIter { token }
    }
}

impl Iterator for RuleTokenIter {
    type Item = Arc<RwLock<RuleToken>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.token.is_some() {
            let ret = self.token.clone();
            let next = self.token.clone().unwrap().read().unwrap().next.clone();
            self.token = next;
            return ret;
        }

        None
    }
}
