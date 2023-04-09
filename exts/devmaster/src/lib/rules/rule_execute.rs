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

use super::{RuleFile, RuleLine, RuleToken, Rules, TokenType};
use crate::error::Result;
use device::{Device, DeviceAction};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, RwLock},
};

/// the process unit on device uevent
#[allow(missing_docs)]
struct ExecuteUnit {
    device: Rc<RefCell<Device>>,
    // parent: Option<Arc<Mutex<Device>>>,
    // device_db_clone: Option<Device>,
    // name: String,
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
    pub fn new(device: Rc<RefCell<Device>>) -> ExecuteUnit {
        // let mut unit = ProcessUnit::default();
        // unit.device = device;
        // unit
        ExecuteUnit {
            device,
            // parent: None,
            // device_db_clone: None,
            // name: (),
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
    rules: Rules,

    current_rule_file: Option<Arc<RwLock<RuleFile>>>,
    current_rule_line: Option<Arc<RwLock<RuleLine>>>,
    current_rule_token: Option<Arc<RwLock<RuleToken>>>,

    current_unit: Option<ExecuteUnit>,
}

impl ExecuteManager {
    /// create a execute manager object
    pub fn new(rules: Rules) -> ExecuteManager {
        ExecuteManager {
            rules,
            current_rule_file: None,
            current_rule_line: None,
            current_rule_token: None,
            current_unit: None,
        }
    }

    /// process a device object
    pub fn process_device(&mut self, device: Rc<RefCell<Device>>) -> Result<()> {
        log::debug!("Processing device {}", device.as_ref().borrow_mut().devpath);

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

    /// excute rules
    pub(crate) fn execute_rules(&mut self) -> Result<()> {
        let unit = self.current_unit.as_mut().unwrap();

        let action = unit.device.as_ref().borrow_mut().action;

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
        self.current_rule_file = self.rules.files.clone();

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
    pub(crate) fn apply_rule_line(&mut self) -> Result<Option<Arc<RwLock<RuleLine>>>> {
        self.current_rule_token = self
            .current_rule_line
            .clone()
            .unwrap()
            .as_ref()
            .read()
            .unwrap()
            .tokens
            .clone();

        // only apply rule token on parent device once
        // that means if some a parent device matches the token, do not match any parent tokens in the following
        let mut parents_done = false;

        loop {
            let next_token = self
                .current_rule_token
                .clone()
                .unwrap()
                .as_ref()
                .read()
                .unwrap()
                .next
                .clone();

            if self
                .current_rule_token
                .clone()
                .unwrap()
                .as_ref()
                .read()
                .unwrap()
                .is_for_parents()
            {
                if !parents_done {
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
                }
            } else if !self.apply_rule_token()? {
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

            self.current_rule_token = next_token;
            if self.current_rule_token.is_none() {
                break;
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
    pub(crate) fn apply_rule_token(&mut self) -> Result<bool> {
        let token_type = self
            .current_rule_token
            .clone()
            .unwrap()
            .as_ref()
            .read()
            .unwrap()
            .r#type;

        let device = self
            .current_unit
            .as_ref()
            .unwrap()
            .device
            .as_ref()
            .borrow_mut();

        let token = self
            .current_rule_token
            .as_ref()
            .unwrap()
            .as_ref()
            .read()
            .unwrap();

        match token_type {
            TokenType::MatchAction => {
                let action = device.action;

                return Ok(action.to_string() == token.value);
            }
            TokenType::AssignDevlink => {
                println!("{}", token.value);
            }
            _ => {
                todo!();
            }
        }

        Ok(true)
    }

    /// apply rule token on the parent device
    pub(crate) fn apply_rule_token_on_parent(&mut self) -> Result<bool> {
        todo!();
    }

    /// execute run
    pub(crate) fn execute_run(&mut self) -> Result<()> {
        Ok(())
    }
}
