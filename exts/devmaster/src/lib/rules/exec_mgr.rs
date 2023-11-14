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

use crate::{
    builtin::*,
    device_trace,
    error::*,
    execute_err, execute_err_ignore_ENOENT,
    framework::devmaster::*,
    log_dev, log_rule_line, log_rule_token,
    rules::exec_unit::*,
    rules::node::*,
    rules::TokenType::*,
    rules::*,
    utils::{commons::*, spawn::*},
};
use basic::{
    cmdline::cmdline_get_item, fs_util::write_string_file, naming_scheme::*, parse::parse_mode,
    unistd::*,
};
use device::{Device, DeviceAction};
use fnmatch_sys::fnmatch;
use libc::{gid_t, mode_t, uid_t};
use nix::{
    errno::Errno,
    unistd::{Gid, Uid},
};
use snafu::ResultExt;
use std::{
    cell::RefCell,
    collections::HashMap,
    fs::OpenOptions,
    io::Read,
    os::{raw::c_char, unix::fs::PermissionsExt},
    rc::Rc,
    sync::Arc,
    sync::RwLock,
    time::Duration,
};

/// manage processing units
pub(crate) struct ExecuteManager {
    cache: Arc<RwLock<Cache>>,
    builtin_mgr: BuiltinManager,

    current_rule_file: RefCell<Arc<RwLock<Option<RuleFile>>>>,
    current_rule_line: RefCell<Arc<RwLock<Option<RuleLine>>>>,
    current_rule_token: RefCell<Arc<RwLock<Option<RuleToken>>>>,

    current_unit: RefCell<Option<ExecuteUnit>>,

    properties: RefCell<HashMap<String, String>>,

    unit_spawn_timeout_usec: u64,
}

impl ExecuteManager {
    /// create a execute manager object
    pub(crate) fn new(cache: Arc<RwLock<Cache>>) -> ExecuteManager {
        let builtin_mgr = BuiltinManager::new(cache.clone());

        builtin_mgr.init();

        ExecuteManager {
            cache,
            builtin_mgr,
            current_rule_file: RefCell::new(Arc::new(RwLock::new(None))),
            current_rule_line: RefCell::new(Arc::new(RwLock::new(None))),
            current_rule_token: RefCell::new(Arc::new(RwLock::new(None))),
            current_unit: RefCell::new(None),
            properties: RefCell::new(HashMap::new()),
            unit_spawn_timeout_usec: 3,
        }
    }

    /// process a device object
    pub(crate) fn process_device(&self, device: Rc<RefCell<Device>>) -> Result<()> {
        log::debug!(
            "{}",
            device_trace!("Start processing device", device.borrow())
        );

        *self.current_unit.borrow_mut() = Some(ExecuteUnit::new(device.clone()));
        // lock whole disk: todo

        // mark block device read only: todo

        self.execute_rules()?;

        self.execute_run();

        // update rtnl

        // begin inotify watch: todo

        log::debug!(
            "{}",
            device_trace!("Finish processing device", device.borrow())
        );

        *self.current_unit.borrow_mut() = None;

        Ok(())
    }

    /// execute rules
    pub(crate) fn execute_rules(&self) -> Result<()> {
        debug_assert!(self.current_unit.borrow().is_some());

        let device = self.current_unit.borrow().as_ref().unwrap().get_device();

        let action = device
            .borrow()
            .get_action()
            .context(DeviceSnafu)
            .log_dev_error(&device.borrow(), "not from uevent")?;

        if action == DeviceAction::Remove {
            return self.execute_rules_on_remove();
        }

        // inotify watch end: todo

        // clone device with db
        let _ = self
            .current_unit
            .borrow()
            .as_ref()
            .unwrap()
            .clone_device_db()
            .log_dev_debug(&device.borrow(), "failed to clone with db");
        let device_db_clone = self
            .current_unit
            .borrow()
            .as_ref()
            .unwrap()
            .get_device_db_clone();

        // copy all tags to cloned device
        for tag in &device.borrow().tag_iter() {
            device_db_clone.borrow().add_tag(tag, false);
        }

        // add property to cloned device
        device_db_clone
            .borrow()
            .add_property("ID_RENAMING", "")
            .map_err(|e| Error::RulesExecuteError {
                msg: format!("failed to add tag ({})", e),
                errno: e.get_errno(),
            })?;

        self.apply_rules()?;

        // rename netif
        if action == DeviceAction::Add {
            self.current_unit
                .borrow()
                .as_ref()
                .unwrap()
                .rename_netif()
                .log_dev_error(&device.borrow(), "rename ifname failed")?;
        }

        // update devnode
        self.current_unit
            .borrow()
            .as_ref()
            .unwrap()
            .update_devnode()?;

        // preserve old, or get new initialization timestamp
        initialize_device_usec(device.clone(), device_db_clone.clone())
            .log_dev_error(&device.borrow(), "failed to initialize device timestamp")?;

        // update tags and database
        let _ = device_update_tag(device.clone(), Some(device_db_clone), true);

        device
            .borrow()
            .update_db()
            .context(DeviceSnafu)
            .log_error("failed to update db")?;

        device.borrow().set_is_initialized();

        Ok(())
    }

    /// execute rules on remove uevent
    pub(crate) fn execute_rules_on_remove(&self) -> Result<()> {
        debug_assert!(self.current_unit.borrow().is_some());

        let device = self.current_unit.borrow().as_ref().unwrap().get_device();

        device
            .borrow()
            .read_db_internal(true)
            .context(DeviceSnafu)
            .log_error("remove event failed")?;

        device_update_tag(device.clone(), None, false).log_error("remove event failed")?;

        cleanup_db(device.clone()).log_error("cleanup db failed")?;

        // todo: watch end

        let ret = self.apply_rules();

        if device.borrow().get_devnum().is_err() {
            return ret;
        }

        let _ = cleanup_node(device);

        ret
    }

    /// apply rules on device
    pub(crate) fn apply_rules(&self) -> Result<()> {
        *self.current_rule_file.borrow_mut() = self
            .cache
            .read()
            .unwrap()
            .rules
            .read()
            .unwrap()
            .files
            .clone();

        while self.current_rule_file.borrow().read().unwrap().is_some() {
            let next_file = self
                .current_rule_file
                .borrow()
                .read()
                .unwrap()
                .as_ref()
                .unwrap()
                .next
                .clone();

            self.apply_rule_file()?;

            *self.current_rule_file.borrow_mut() = next_file;
        }

        Ok(())
    }

    /// apply rule file on device
    pub(crate) fn apply_rule_file(&self) -> Result<()> {
        *self.current_rule_line.borrow_mut() = self
            .current_rule_file
            .borrow()
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
            .lines
            .clone();

        while self.current_rule_line.borrow().read().unwrap().is_some() {
            let next_line = self.apply_rule_line()?;

            *self.current_rule_line.borrow_mut() = next_line;
        }

        Ok(())
    }

    /// apply rule line on device
    /// normally return the next rule line after current line
    /// if current line has goto label, use the line with the target label as the next line
    pub(crate) fn apply_rule_line(&self) -> Result<Arc<RwLock<Option<RuleLine>>>> {
        debug_assert!(self.current_rule_line.borrow().read().unwrap().is_some());
        debug_assert!(self.current_rule_file.borrow().read().unwrap().is_some());

        // only apply rule token on parent device once
        // that means if some a parent device matches the token.read().unwrap().as_ref().unwrap(), do not match any parent tokens in the following
        let mut parents_done = false;

        // if the current line does not intersect with the mask, skip applying current line.
        let mut mask = RuleLineType::HAS_GOTO | RuleLineType::UPDATE_SOMETHING;

        let current_unit = self.current_unit.borrow();
        let device = current_unit.as_ref().unwrap().get_device();
        let action = device.borrow().get_action().context(DeviceSnafu)?;
        let current_line = self.current_rule_line.borrow().clone();

        if action != DeviceAction::Remove {
            if device.borrow().get_devnum().is_ok() {
                mask |= RuleLineType::HAS_DEVLINK;
            }

            if device.borrow().get_ifindex().is_ok() {
                mask |= RuleLineType::HAS_NAME;
            }
        }

        // if the current line does not match the mask, skip to next line.
        if (current_line.read().unwrap().as_ref().unwrap().r#type & mask).bits() == 0 {
            log_rule_line!(
                debug,
                current_line.read().unwrap().as_ref().unwrap(),
                "mask does not match, ignoring this line"
            );
            return Ok(current_line.read().unwrap().as_ref().unwrap().next.clone());
        }

        *self.current_rule_token.borrow_mut() = current_line
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
            .tokens
            .clone();

        current_unit
            .as_ref()
            .unwrap()
            .set_escape_type(EscapeType::Unset);

        let current_token = self.current_rule_token.borrow().clone();

        for token in RuleToken::iter(current_token) {
            *self.current_rule_token.borrow_mut() = token.clone();

            if token.read().unwrap().as_ref().unwrap().is_for_parents() {
                if parents_done {
                    continue;
                }
                if !self.apply_rule_token_on_parent()? {
                    // if current rule token does not match, abort applying the rest tokens in this line
                    log_rule_token!(
                        debug,
                        token.read().unwrap().as_ref().unwrap(),
                        "fails to match."
                    );

                    return Ok(current_line.read().unwrap().as_ref().unwrap().next.clone());
                }

                parents_done = true;
                continue;
            }

            if !self.apply_rule_token(device.clone())? {
                // if current rule token does not match, abort applying the rest tokens in this line
                log_rule_token!(
                    debug,
                    token.read().unwrap().as_ref().unwrap(),
                    "fails to match."
                );

                return Ok(current_line.read().unwrap().as_ref().unwrap().next.clone());
            }
        }

        let goto_line = current_line
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
            .goto_line
            .clone();

        if goto_line.read().unwrap().is_some() {
            Ok(goto_line)
        } else {
            Ok(current_line.read().unwrap().as_ref().unwrap().next.clone())
        }
    }

    /// apply rule token on device
    pub(crate) fn apply_rule_token(&self, device: Rc<RefCell<Device>>) -> Result<bool> {
        debug_assert!(self.current_unit.borrow().is_some());
        debug_assert!(self.current_rule_token.borrow().read().unwrap().is_some());
        let token = self.current_rule_token.borrow().clone();
        debug_assert!(token.read().unwrap().is_some());

        let token_value = token.read().unwrap().as_ref().unwrap().get_token_value();
        let token_attribute = token
            .read()
            .unwrap()
            .as_ref()
            .unwrap()
            .get_token_attribute();
        let current_unit = self.current_unit.borrow();

        log_rule_token!(
            debug,
            token.read().unwrap().as_ref().unwrap(),
            "applying token"
        );

        let token_type = token.read().unwrap().as_ref().unwrap().r#type;
        match token_type {
            MatchAction => {
                let action = execute_err!(
                    token.read().unwrap().as_ref().unwrap(),
                    device.borrow().get_action()
                )?;

                Ok(token
                    .read()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .pattern_match(&action.to_string()))
            }
            MatchDevpath => {
                let devpath = execute_err!(
                    token.read().unwrap().as_ref().unwrap(),
                    device.borrow().get_devpath()
                )?;

                Ok(token
                    .read()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .pattern_match(&devpath))
            }
            MatchKernel | MatchParentsKernel => {
                let sysname = execute_err!(
                    token.read().unwrap().as_ref().unwrap(),
                    device.borrow().get_sysname()
                )?;

                Ok(token
                    .read()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .pattern_match(&sysname))
            }
            MatchDevlink => {
                for devlink in &device.borrow().devlink_iter() {
                    if token
                        .read()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .pattern_match(devlink)
                        ^ (token.read().unwrap().as_ref().unwrap().op == OperatorType::Nomatch)
                    {
                        return Ok(
                            token.read().unwrap().as_ref().unwrap().op == OperatorType::Match
                        );
                    }
                }

                Ok(token.read().unwrap().as_ref().unwrap().op == OperatorType::Nomatch)
            }
            MatchName => Ok(token
                .read()
                .unwrap()
                .as_ref()
                .unwrap()
                .pattern_match(&current_unit.as_ref().unwrap().get_name())),
            MatchEnv => {
                let value = match device
                    .borrow()
                    .get_property_value(token_attribute.as_ref().unwrap())
                {
                    Ok(v) => v,
                    Err(e) => {
                        if e.get_errno() != Errno::ENOENT {
                            return Err(Error::RulesExecuteError {
                                msg: format!("{}", e),
                                errno: e.get_errno(),
                            });
                        }

                        self.properties
                            .borrow()
                            .get(token_attribute.as_ref().unwrap())
                            .unwrap_or(&"".to_string())
                            .to_string()
                    }
                };

                Ok(token
                    .read()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .pattern_match(&value))
            }
            MatchConst => {
                todo!()
            }
            MatchTag | MatchParentsTag => {
                for tag in &device.borrow().current_tag_iter() {
                    if token.read().unwrap().as_ref().unwrap().pattern_match(tag)
                        ^ (token.read().unwrap().as_ref().unwrap().op == OperatorType::Nomatch)
                    {
                        return Ok(
                            token.read().unwrap().as_ref().unwrap().op == OperatorType::Match
                        );
                    }
                }

                Ok(token.read().unwrap().as_ref().unwrap().op == OperatorType::Nomatch)
            }
            MatchSubsystem | MatchParentsSubsystem => {
                let subsystem = execute_err_ignore_ENOENT!(
                    token.read().unwrap().as_ref().unwrap(),
                    device.borrow().get_subsystem()
                )?;

                Ok(token
                    .read()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .pattern_match(&subsystem))
            }
            MatchDriver | MatchParentsDriver => {
                let driver = execute_err_ignore_ENOENT!(
                    token.read().unwrap().as_ref().unwrap(),
                    device.borrow().get_driver()
                )?;

                Ok(token
                    .read()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .pattern_match(&driver))
            }
            MatchAttr | MatchParentsAttr => token
                .read()
                .unwrap()
                .as_ref()
                .unwrap()
                .attr_match(device, current_unit.as_ref().unwrap())
                .map_err(|e| {
                    log_rule_token!(debug, token.read().unwrap().as_ref().unwrap(), e);
                    e
                }),
            MatchTest => {
                let mut val = match current_unit
                    .as_ref()
                    .unwrap()
                    .apply_format(&token_value, false)
                {
                    Ok(v) => v,
                    Err(e) => {
                        log::debug!("failed to apply formatter: ({})", e);
                        return Ok(
                            token.read().unwrap().as_ref().unwrap().op == OperatorType::Nomatch
                        );
                    }
                };

                // if the value is not an absolute path, try to find it under sysfs
                if !val.starts_with('/') {
                    val = match resolve_subsystem_kernel(&val, false) {
                        Ok(v) => v,
                        Err(_) => {
                            // only throw out error when getting the syspath of device
                            let syspath = execute_err!(
                                token.read().unwrap().as_ref().unwrap(),
                                device.borrow().get_syspath()
                            )
                            .map_err(|e| {
                                log_rule_token!(
                                    debug,
                                    token.read().unwrap().as_ref().unwrap(),
                                    "failed to apply token."
                                );
                                e
                            })?;

                            syspath + "/" + val.as_str()
                        }
                    }
                }

                match sysattr_subdir_subst(&val) {
                    Ok(s) => {
                        let path = std::path::Path::new(&s);

                        if !path.exists() {
                            return Ok(
                                token.read().unwrap().as_ref().unwrap().op == OperatorType::Nomatch
                            );
                        }

                        if let Some(attr) = &token_attribute {
                            let mode = mode_t::from_str_radix(attr, 8).unwrap_or_else(|_| {
                                log::debug!("failed to parse mode: {}", attr);
                                0
                            });

                            let metadata = match std::fs::metadata(path) {
                                Ok(m) => m,
                                Err(_) => {
                                    return Ok(token.read().unwrap().as_ref().unwrap().op
                                        == OperatorType::Nomatch);
                                }
                            };

                            let permissions = metadata.permissions().mode();

                            Ok((mode & permissions > 0)
                                ^ (token.read().unwrap().as_ref().unwrap().op
                                    == OperatorType::Nomatch))
                        } else {
                            Ok(token.read().unwrap().as_ref().unwrap().op == OperatorType::Match)
                        }
                    }
                    Err(e) => {
                        if e.get_errno() == nix::errno::Errno::ENOENT {
                            return Ok(
                                token.read().unwrap().as_ref().unwrap().op == OperatorType::Nomatch
                            );
                        }

                        Err(Error::RulesExecuteError {
                            msg: format!(
                                "Apply '{}' error: {}",
                                token.read().unwrap().as_ref().unwrap(),
                                e
                            ),
                            errno: e.get_errno(),
                        })
                    }
                }
            }
            MatchImportFile => {
                let file_name = match current_unit
                    .as_ref()
                    .unwrap()
                    .apply_format(&token_value, false)
                {
                    Ok(v) => v,
                    Err(e) => {
                        log::debug!("failed to apply formatter: ({})", e);
                        return Ok(false);
                    }
                };

                log_rule_token!(
                    debug,
                    token.read().unwrap().as_ref().unwrap(),
                    format!("Importing properties from file '{}'", file_name)
                );

                let mut file = match OpenOptions::new().read(true).open(&file_name) {
                    Ok(f) => f,
                    Err(e) => {
                        if e.kind() != std::io::ErrorKind::NotFound {
                            log_rule_token!(
                                error,
                                token.read().unwrap().as_ref().unwrap(),
                                format!("failed to open '{}'.", file_name)
                            );
                            return Err(Error::RulesExecuteError {
                                msg: e.to_string(),
                                errno: Errno::from_i32(e.raw_os_error().unwrap_or_default()),
                            });
                        }

                        return Ok(
                            token.read().unwrap().as_ref().unwrap().op == OperatorType::Nomatch
                        );
                    }
                };

                let mut content = String::new();
                if let Err(e) = file.read_to_string(&mut content) {
                    log_rule_token!(
                        debug,
                        token.read().unwrap().as_ref().unwrap(),
                        format!("failed to read '{}': {}", file_name, e)
                    );
                    return Ok(token.read().unwrap().as_ref().unwrap().op == OperatorType::Nomatch);
                }

                for line in content.split('\n') {
                    match get_property_from_string(line) {
                        Ok((key, value)) => {
                            execute_err!(
                                token.read().unwrap().as_ref().unwrap(),
                                device.borrow().add_property(&key, &value)
                            )?;
                        }
                        Err(e) => {
                            log_rule_token!(debug, token.read().unwrap().as_ref().unwrap(), e);
                        }
                    }
                }

                Ok(token.read().unwrap().as_ref().unwrap().op == OperatorType::Match)
            }
            MatchImportProgram => {
                let cmd = match current_unit
                    .as_ref()
                    .unwrap()
                    .apply_format(&token_value, false)
                {
                    Ok(v) => v,
                    Err(e) => {
                        log::debug!("failed to apply formatter: ({})", e);
                        return Ok(false);
                    }
                };

                log_rule_token!(
                    debug,
                    token.read().unwrap().as_ref().unwrap(),
                    format!("Importing properties from output of cmd '{}'", cmd)
                );

                let result = match spawn(
                    &cmd,
                    Duration::from_secs(self.unit_spawn_timeout_usec),
                    current_unit.as_ref().unwrap(),
                ) {
                    Ok(s) => {
                        if s.1 < 0 {
                            log_rule_token!(
                                debug,
                                token.read().unwrap().as_ref().unwrap(),
                                format!("command returned {}, ignoring.", s.1)
                            );
                            return Ok(
                                token.read().unwrap().as_ref().unwrap().op == OperatorType::Nomatch
                            );
                        }
                        s.0
                    }
                    Err(e) => {
                        log_rule_token!(
                            debug,
                            token.read().unwrap().as_ref().unwrap(),
                            format!("failed execute command: ({})", e)
                        );
                        return Ok(
                            token.read().unwrap().as_ref().unwrap().op == OperatorType::Nomatch
                        );
                    }
                };

                for line in result.split('\n') {
                    if line.is_empty() {
                        continue;
                    }

                    match get_property_from_string(line) {
                        Ok((key, value)) => {
                            execute_err!(
                                token.read().unwrap().as_ref().unwrap(),
                                device.borrow().add_property(&key, &value)
                            )?;

                            log_rule_token!(
                                debug,
                                token.read().unwrap().as_ref().unwrap(),
                                format!("add key-value ({}={})", key, value)
                            )
                        }
                        Err(e) => {
                            log_rule_token!(debug, token.read().unwrap().as_ref().unwrap(), e);
                        }
                    }
                }

                Ok(token.read().unwrap().as_ref().unwrap().op == OperatorType::Match)
            }
            MatchImportBuiltin => {
                let builtin = match token_value.parse::<BuiltinCommand>() {
                    Ok(cmd) => cmd,
                    Err(_) => {
                        log_rule_token!(
                            error,
                            token.read().unwrap().as_ref().unwrap(),
                            "invalid builtin command."
                        );
                        return Ok(false);
                    }
                };

                let mask = 0b1 << builtin as u32;
                let already_run = current_unit.as_ref().unwrap().get_builtin_run();
                let run_result = current_unit.as_ref().unwrap().get_builtin_ret();

                if self.builtin_mgr.run_once(builtin) {
                    if already_run & mask != 0 {
                        log_rule_token!(
                            debug,
                            token.read().unwrap().as_ref().unwrap(),
                            format!(
                                "builtin '{}' can only run once and has run before.",
                                builtin
                            )
                        );
                        return Ok((token.read().unwrap().as_ref().unwrap().op
                            == OperatorType::Match)
                            ^ (run_result & mask > 0));
                    }

                    current_unit
                        .as_ref()
                        .unwrap()
                        .set_builtin_run(already_run | mask);
                }

                let cmd = match current_unit
                    .as_ref()
                    .unwrap()
                    .apply_format(&token_value, false)
                {
                    Ok(v) => v,
                    Err(e) => {
                        log_rule_token!(
                            error,
                            token.read().unwrap().as_ref().unwrap(),
                            format!("failed to apply formatter: ({})", e)
                        );
                        return Ok(false);
                    }
                };

                let argv = shell_words::split(&cmd).map_err(|e| Error::RulesExecuteError {
                    msg: format!(
                        "failed to split command '{}' into shell tokens: ({})",
                        cmd, e
                    ),
                    errno: nix::errno::Errno::EINVAL,
                })?;

                log_rule_token!(
                    debug,
                    token.read().unwrap().as_ref().unwrap(),
                    format!("Importing properties from builtin cmd '{}'", cmd)
                );

                match self.builtin_mgr.run(
                    current_unit.as_ref().unwrap(),
                    builtin,
                    argv.len() as i32,
                    argv,
                    false,
                ) {
                    Ok(ret) => {
                        // if builtin command returned false, set the mask bit to 1
                        current_unit
                            .as_ref()
                            .unwrap()
                            .set_builtin_ret(run_result | ((!ret as u32) << builtin as u32));
                        Ok(
                            (token.read().unwrap().as_ref().unwrap().op == OperatorType::Nomatch)
                                ^ ret,
                        )
                    }
                    Err(e) => {
                        log_rule_token!(
                            error,
                            token.read().unwrap().as_ref().unwrap(),
                            format!("failed to run builtin ({})", e)
                        );
                        Ok(token.read().unwrap().as_ref().unwrap().op == OperatorType::Nomatch)
                    }
                }
            }
            MatchImportDb => {
                let dev_db_clone = current_unit.as_ref().unwrap().get_device_db_clone();

                let val = match dev_db_clone.borrow().get_property_value(&token_value) {
                    Ok(v) => v,
                    Err(e) => {
                        if e.get_errno() == Errno::ENOENT {
                            return Ok(
                                token.read().unwrap().as_ref().unwrap().op == OperatorType::Nomatch
                            );
                        }

                        log_rule_token!(
                            error,
                            token.read().unwrap().as_ref().unwrap(),
                            format!("failed to get property '{}' from db: ({})", token_value, e)
                        );
                        return Err(Error::RulesExecuteError {
                            msg: format!(
                                "Apply '{}' error: {}",
                                token.read().unwrap().as_ref().unwrap(),
                                e
                            ),
                            errno: e.get_errno(),
                        });
                    }
                };

                log_rule_token!(
                    debug,
                    token.read().unwrap().as_ref().unwrap(),
                    format!("Importing property '{}={}' from db", token_value, val)
                );

                execute_err!(
                    token.read().unwrap().as_ref().unwrap(),
                    device.borrow().add_property(&token_value, &val)
                )?;

                Ok(token.read().unwrap().as_ref().unwrap().op == OperatorType::Match)
            }
            MatchImportCmdline => {
                let s = cmdline_get_item(&token_value).map_err(|e| {
                    log_rule_token!(error, token.read().unwrap().as_ref().unwrap(), e);
                    Error::RulesExecuteError {
                        msg: format!(
                            "Apply '{}' failed: {}",
                            token.read().unwrap().as_ref().unwrap(),
                            e
                        ),
                        errno: Errno::EINVAL,
                    }
                })?;

                if s.is_none() {
                    return Ok(token.read().unwrap().as_ref().unwrap().op == OperatorType::Nomatch);
                }

                let value = match s.as_ref().unwrap().split_once('=') {
                    Some(ret) => ret.1,
                    None => "",
                };

                execute_err!(
                    token.read().unwrap().as_ref().unwrap(),
                    device
                        .borrow()
                        .add_property(&token_value, if value.is_empty() { "1" } else { value })
                )?;

                Ok(token.read().unwrap().as_ref().unwrap().op == OperatorType::Match)
            }
            MatchImportParent => {
                let value = match current_unit
                    .as_ref()
                    .unwrap()
                    .apply_format(&token_value, false)
                {
                    Ok(v) => v,
                    Err(e) => {
                        log::debug!("failed to apply formatter: ({})", e);
                        return Ok(false);
                    }
                };

                let mut glob_patterns: Vec<String> = vec![];

                // generate regular expression depending on the formatted value
                for s in value.split('|') {
                    let pat = format!("{}\0", s);
                    glob_patterns.push(pat);
                }

                let parent = match device.borrow().get_parent() {
                    Ok(p) => p,
                    Err(e) => {
                        // do not match if the device has no parent
                        if e.get_errno() == Errno::ENOENT {
                            return Ok(
                                token.read().unwrap().as_ref().unwrap().op == OperatorType::Nomatch
                            );
                        }

                        log_rule_token!(error, token.read().unwrap().as_ref().unwrap(), e);

                        return Err(Error::RulesExecuteError {
                            msg: format!(
                                "Apply '{}' failed: {}",
                                token.read().unwrap().as_ref().unwrap(),
                                e
                            ),
                            errno: e.get_errno(),
                        });
                    }
                };

                for (k, v) in &parent.borrow().property_iter() {
                    let source = format!("{}\0", k);

                    // check whether the key of property matches the
                    if !{
                        let mut matched = false;
                        for p in glob_patterns.iter() {
                            if unsafe {
                                fnmatch(
                                    p.as_ptr() as *const c_char,
                                    source.as_ptr() as *const c_char,
                                    0,
                                )
                            } == 0
                            {
                                matched = true;
                                break;
                            }
                        }
                        matched
                    } {
                        continue;
                    }

                    log_rule_token!(
                        debug,
                        token.read().unwrap().as_ref().unwrap(),
                        format!("Importing '{}={}' from parent.", k, v)
                    );

                    execute_err!(
                        token.read().unwrap().as_ref().unwrap(),
                        device.borrow().add_property(k, v)
                    )?;
                }

                Ok(token.read().unwrap().as_ref().unwrap().op == OperatorType::Match)
            }
            MatchResult => Ok(token
                .read()
                .unwrap()
                .as_ref()
                .unwrap()
                .pattern_match(&current_unit.as_ref().unwrap().get_program_result())),
            MatchProgram => {
                let cmd = match current_unit
                    .as_ref()
                    .unwrap()
                    .apply_format(&token_value, false)
                {
                    Ok(v) => v,
                    Err(e) => {
                        log_rule_token!(
                            error,
                            token.read().unwrap().as_ref().unwrap(),
                            format!("failed to apply formatter: ({})", e)
                        );
                        return Ok(false);
                    }
                };

                let result = match spawn(
                    &cmd,
                    Duration::from_secs(self.unit_spawn_timeout_usec),
                    current_unit.as_ref().unwrap(),
                ) {
                    Ok(s) => {
                        if s.1 != 0 {
                            return Ok(
                                token.read().unwrap().as_ref().unwrap().op == OperatorType::Nomatch
                            );
                        }
                        s.0
                    }
                    Err(e) => {
                        log_rule_token!(
                            debug,
                            token.read().unwrap().as_ref().unwrap(),
                            format!("failed to apply token: ({})", e)
                        );
                        return Ok(false);
                    }
                };

                let result = replace_chars(result.trim_end(), DEVMASTER_LEGAL_CHARS);

                log::debug!(
                    "\x1b[34mCapture stdout from command '{}': '{}'\x1b[0m",
                    cmd,
                    &result
                );

                current_unit.as_ref().unwrap().set_program_result(result);

                Ok(token.read().unwrap().as_ref().unwrap().op == OperatorType::Match)
            }
            AssignOptionsStringEscapeNone => {
                current_unit
                    .as_ref()
                    .unwrap()
                    .set_escape_type(EscapeType::None);
                log_rule_token!(
                    debug,
                    token.read().unwrap().as_ref().unwrap(),
                    "set string escape to 'none'"
                );
                Ok(true)
            }
            AssignOptionsStringEscapeReplace => {
                current_unit
                    .as_ref()
                    .unwrap()
                    .set_escape_type(EscapeType::Replace);
                log_rule_token!(
                    debug,
                    token.read().unwrap().as_ref().unwrap(),
                    "set string escape to 'replace'"
                );
                Ok(true)
            }
            AssignOptionsDbPersist => {
                device.borrow().set_db_persist();
                log_rule_token!(
                    debug,
                    token.read().unwrap().as_ref().unwrap(),
                    format!(
                        "set db '{}' to persistence",
                        execute_err!(
                            token.read().unwrap().as_ref().unwrap(),
                            device.borrow().get_device_id()
                        )?
                    )
                );
                Ok(true)
            }
            AssignOptionsWatch => {
                if current_unit.as_ref().unwrap().get_watch_final() {
                    log_rule_token!(
                        debug,
                        token.read().unwrap().as_ref().unwrap(),
                        format!(
                            "watch is fixed to '{}'",
                            current_unit.as_ref().unwrap().is_watch()
                        )
                    );
                    return Ok(true);
                }

                if token.read().unwrap().as_ref().unwrap().op == OperatorType::AssignFinal {
                    current_unit.as_ref().unwrap().set_watch_final(true);
                }

                // token value is either "true" or "false"
                current_unit.as_ref().unwrap().set_watch(execute_err!(
                    token.read().unwrap().as_ref().unwrap(),
                    token_value.parse::<bool>().context(ParseBoolSnafu)
                )?);

                log_rule_token!(
                    debug,
                    token.read().unwrap().as_ref().unwrap(),
                    format!("set watch to '{}'", token_value)
                );

                Ok(true)
            }
            AssignOptionsDevlinkPriority => {
                let r = execute_err!(
                    token.read().unwrap().as_ref().unwrap(),
                    token_value.parse::<i32>().context(ParseIntSnafu)
                )?;
                device.borrow().set_devlink_priority(r);
                log_rule_token!(
                    debug,
                    token.read().unwrap().as_ref().unwrap(),
                    format!("set devlink priority to '{}'", r)
                );
                Ok(true)
            }
            AssignOptionsLogLevel => {
                todo!()
            }
            AssignOwner => {
                if current_unit.as_ref().unwrap().is_owner_final() {
                    log_rule_token!(
                        debug,
                        token.read().unwrap().as_ref().unwrap(),
                        "owner is final-assigned previously, ignore this assignment"
                    );
                    return Ok(true);
                }

                if token.read().unwrap().as_ref().unwrap().op == OperatorType::AssignFinal {
                    current_unit.as_ref().unwrap().set_owner_final(true);
                }

                let owner = match current_unit
                    .as_ref()
                    .unwrap()
                    .apply_format(&token_value, false)
                {
                    Ok(v) => v,
                    Err(e) => {
                        log_rule_token!(
                            error,
                            token.read().unwrap().as_ref().unwrap(),
                            format!("failed to apply formatter: ({})", e)
                        );
                        return Ok(true);
                    }
                };

                match get_user_creds(&owner) {
                    Ok(u) => {
                        log_rule_token!(
                            debug,
                            token.read().unwrap().as_ref().unwrap(),
                            format!("assign uid '{}' from owner '{}'", u.uid, owner)
                        );

                        current_unit.as_ref().unwrap().set_uid(Some(u.uid));
                    }
                    Err(_) => {
                        log_rule_token!(
                            error,
                            token.read().unwrap().as_ref().unwrap(),
                            format!("unknown user '{}'", owner)
                        );
                    }
                }

                Ok(true)
            }
            AssignOwnerId => {
                /*
                 *  owner id is already resolved during rules loading, token value is the uid string
                 */
                if current_unit.as_ref().unwrap().is_owner_final() {
                    log_rule_token!(
                        debug,
                        token.read().unwrap().as_ref().unwrap(),
                        "owner is final-assigned previously, ignore this assignment"
                    );
                    return Ok(true);
                }

                if token.read().unwrap().as_ref().unwrap().op == OperatorType::AssignFinal {
                    current_unit.as_ref().unwrap().set_owner_final(true);
                }

                log_rule_token!(
                    debug,
                    token.read().unwrap().as_ref().unwrap(),
                    format!("assign uid '{}'", token_value)
                );

                let uid = execute_err!(
                    token.read().unwrap().as_ref().unwrap(),
                    token_value.parse::<uid_t>().context(ParseIntSnafu)
                )?;

                current_unit
                    .as_ref()
                    .unwrap()
                    .set_uid(Some(Uid::from_raw(uid)));

                Ok(true)
            }
            AssignGroup => {
                if current_unit.as_ref().unwrap().is_group_final() {
                    log_rule_token!(
                        debug,
                        token.read().unwrap().as_ref().unwrap(),
                        "group is final-assigned previously, ignore this assignment"
                    );
                    return Ok(true);
                }

                if token.read().unwrap().as_ref().unwrap().op == OperatorType::AssignFinal {
                    current_unit.as_ref().unwrap().set_group_final(true);
                }

                let group = match current_unit
                    .as_ref()
                    .unwrap()
                    .apply_format(&token_value, false)
                {
                    Ok(v) => v,
                    Err(e) => {
                        log_rule_token!(
                            error,
                            token.read().unwrap().as_ref().unwrap(),
                            format!("failed to apply formatter: ({})", e)
                        );
                        return Ok(true);
                    }
                };

                match get_group_creds(&group) {
                    Ok(g) => {
                        log_rule_token!(
                            debug,
                            token.read().unwrap().as_ref().unwrap(),
                            format!("assign gid '{}' from group '{}'", g.gid, group)
                        );

                        current_unit.as_ref().unwrap().set_gid(Some(g.gid));
                    }
                    Err(_) => {
                        log_rule_token!(
                            error,
                            token.read().unwrap().as_ref().unwrap(),
                            format!("unknown group '{}'", group)
                        );
                    }
                }

                Ok(true)
            }
            AssignGroupId => {
                /*
                 *  group id is already resolved during rules loading, token value is the gid string
                 */
                if current_unit.as_ref().unwrap().is_group_final() {
                    log_rule_token!(
                        debug,
                        token.read().unwrap().as_ref().unwrap(),
                        "group is final-assigned previously, ignore this assignment"
                    );
                    return Ok(true);
                }

                if token.read().unwrap().as_ref().unwrap().op == OperatorType::AssignFinal {
                    current_unit.as_ref().unwrap().set_group_final(true);
                }

                log_rule_token!(
                    debug,
                    token.read().unwrap().as_ref().unwrap(),
                    format!("assign gid '{}'", token_value)
                );

                let gid = execute_err!(
                    token.read().unwrap().as_ref().unwrap(),
                    token_value.parse::<gid_t>().context(ParseIntSnafu)
                )?;

                current_unit
                    .as_ref()
                    .unwrap()
                    .set_gid(Some(Gid::from_raw(gid)));

                Ok(true)
            }
            AssignMode => {
                if current_unit.as_ref().unwrap().is_mode_final() {
                    log_rule_token!(
                        debug,
                        token.read().unwrap().as_ref().unwrap(),
                        "mode is final-assigned previously, ignore this assignment"
                    );
                    return Ok(true);
                }

                if token.read().unwrap().as_ref().unwrap().op == OperatorType::AssignFinal {
                    current_unit.as_ref().unwrap().set_mode_final(true);
                }

                let mode = match current_unit
                    .as_ref()
                    .unwrap()
                    .apply_format(&token_value, false)
                {
                    Ok(v) => v,
                    Err(e) => {
                        log_rule_token!(
                            error,
                            token.read().unwrap().as_ref().unwrap(),
                            format!("failed to apply formatter: ({})", e)
                        );
                        return Ok(true);
                    }
                };

                match parse_mode(&mode) {
                    Ok(v) => {
                        log_rule_token!(
                            debug,
                            token.read().unwrap().as_ref().unwrap(),
                            format!("assign mode '{}'", v)
                        );
                        current_unit.as_ref().unwrap().set_mode(Some(v));
                    }
                    Err(_) => {
                        log_rule_token!(
                            error,
                            token.read().unwrap().as_ref().unwrap(),
                            format!("unknown mode string '{}'", mode)
                        );
                    }
                }

                Ok(true)
            }
            AssignModeId => {
                /*
                 * todo: if the value of 'Mode', 'Owner' or 'Group' is plain string,
                 * it can be parsed during rules loading. Currently, rules token carries
                 * string and thus the string will be repeatedly parsed during loading and
                 * executing. This will lead to performance loss. In future, we can let
                 * the rules token carry the raw data and automatically transform to
                 * specific object during executing for acceleration.
                 */
                if current_unit.as_ref().unwrap().is_mode_final() {
                    log_rule_token!(
                        debug,
                        token.read().unwrap().as_ref().unwrap(),
                        "mode is final-assigned previously, ignore this assignment"
                    );
                    return Ok(true);
                }

                if token.read().unwrap().as_ref().unwrap().op == OperatorType::AssignFinal {
                    current_unit.as_ref().unwrap().set_mode_final(true);
                }

                match parse_mode(&token_value) {
                    Ok(v) => {
                        log_rule_token!(
                            debug,
                            token.read().unwrap().as_ref().unwrap(),
                            format!("assign mode '{}'", v)
                        );
                        current_unit.as_ref().unwrap().set_mode(Some(v));
                    }
                    Err(_) => {
                        log_rule_token!(
                            error,
                            token.read().unwrap().as_ref().unwrap(),
                            format!("unknown mode string '{}'", token_value)
                        );
                    }
                }

                Ok(true)
            }
            AssignSeclabel => {
                todo!()
            }
            AssignEnv => {
                if token_value.is_empty() {
                    if token.read().unwrap().as_ref().unwrap().op == OperatorType::Add {
                        return Ok(true);
                    }

                    /*
                     * The attribute of token is checked to be non-empty during rules loading,
                     * thus we can safely unwrap it.
                     */
                    execute_err!(
                        token.read().unwrap().as_ref().unwrap(),
                        device
                            .borrow()
                            .add_property(&token_attribute.unwrap(), &token_value)
                    )?;
                    return Ok(true);
                }

                let mut value: String = String::new();

                if token.read().unwrap().as_ref().unwrap().op == OperatorType::Add {
                    if let Ok(old_value) = device
                        .borrow()
                        .get_property_value(token_attribute.as_ref().unwrap())
                    {
                        value.push_str(&old_value);
                        value.push(' ');
                    }
                }

                value.push_str(&execute_err!(
                    token.read().unwrap().as_ref().unwrap(),
                    current_unit
                        .as_ref()
                        .unwrap()
                        .apply_format(&token_value, false)
                )?);

                let v = if current_unit.as_ref().unwrap().get_escape_type() == EscapeType::Replace {
                    replace_chars(&value, "")
                } else {
                    value
                };

                execute_err!(
                    token.read().unwrap().as_ref().unwrap(),
                    device
                        .borrow()
                        .add_property(token_attribute.as_ref().unwrap(), &v)
                )?;

                Ok(true)
            }
            AssignTag => {
                let value = match current_unit
                    .as_ref()
                    .unwrap()
                    .apply_format(&token_value, false)
                {
                    Ok(v) => v,
                    Err(e) => {
                        log_rule_token!(
                            error,
                            token.read().unwrap().as_ref().unwrap(),
                            format!("failed to apply formatter: ({})", e)
                        );
                        return Ok(true);
                    }
                };

                if token.read().unwrap().as_ref().unwrap().op == OperatorType::Assign {
                    device.borrow().cleanup_tags();
                }

                if value
                    .find(|c: char| !(c.is_alphanumeric() || "-_".contains(c)))
                    .is_some()
                {
                    log_rule_token!(
                        error,
                        token.read().unwrap().as_ref().unwrap(),
                        format!("Invalid tag name '{}'", value)
                    );
                    return Ok(true);
                }

                if token.read().unwrap().as_ref().unwrap().op == OperatorType::Remove {
                    device.borrow().remove_tag(&value);
                } else {
                    device.borrow().add_tag(&value, true);
                }

                Ok(true)
            }
            AssignName => {
                if current_unit.as_ref().unwrap().is_name_final() {
                    return Ok(true);
                }

                if token.read().unwrap().as_ref().unwrap().op == OperatorType::AssignFinal {
                    current_unit.as_ref().unwrap().set_name_final(true);
                }

                if device.borrow().get_ifindex().is_err() {
                    log_rule_token!(
                        error,
                        token.read().unwrap().as_ref().unwrap(),
                        "Only network interfaces can be renamed, ignoring this token"
                    );

                    return Ok(true);
                }

                let value = match current_unit
                    .as_ref()
                    .unwrap()
                    .apply_format(&token_value, false)
                {
                    Ok(v) => v,
                    Err(e) => {
                        log_rule_token!(
                            error,
                            token.read().unwrap().as_ref().unwrap(),
                            format!("failed to apply formatter: ({})", e)
                        );
                        return Ok(true);
                    }
                };

                let name = if [EscapeType::Unset, EscapeType::Replace]
                    .contains(&current_unit.as_ref().unwrap().get_escape_type())
                {
                    if naming_scheme_has(NamingSchemeFlags::REPLACE_STRICTLY) {
                        replace_ifname(&value)
                    } else {
                        replace_chars(&value, "/")
                    }
                } else {
                    value
                };

                log_rule_token!(
                    debug,
                    token.read().unwrap().as_ref().unwrap(),
                    format!("renaming network interface to '{}'", name)
                );

                current_unit.as_ref().unwrap().set_name(name);

                Ok(true)
            }
            AssignDevlink => {
                if current_unit.as_ref().unwrap().is_devlink_final() {
                    return Ok(true);
                }

                if device.borrow().get_devnum().is_err() {
                    return Ok(true);
                }

                if token.read().unwrap().as_ref().unwrap().op == OperatorType::AssignFinal {
                    current_unit.as_ref().unwrap().set_devlink_final(true);
                }

                if [OperatorType::Assign, OperatorType::AssignFinal]
                    .contains(&token.read().unwrap().as_ref().unwrap().op)
                {
                    device.borrow().cleanup_devlinks();
                }

                let value = match current_unit.as_ref().unwrap().apply_format(
                    &token_value,
                    current_unit.as_ref().unwrap().get_escape_type() != EscapeType::None,
                ) {
                    Ok(v) => v,
                    Err(e) => {
                        log_rule_token!(
                            error,
                            token.read().unwrap().as_ref().unwrap(),
                            format!("failed to apply formatter: ({})", e)
                        );
                        return Ok(true);
                    }
                };

                /*
                 * If the string escape type is set to 'replace', the whitespaces
                 * in the token value will be replaced and the whole string will
                 * be treated as a single symlink.
                 * Otherwise, if the string escape type is not explicitly set or set
                 * to none, the token value will be split by whitespaces and
                 * creat multiple symlinks.
                 */
                let value_escaped = match current_unit.as_ref().unwrap().get_escape_type() {
                    EscapeType::Unset => replace_chars(&value, "/ "),
                    EscapeType::Replace => replace_chars(&value, "/"),
                    _ => value,
                };

                if !value_escaped.trim().is_empty() {
                    for i in value_escaped.trim().split(' ') {
                        let devlink = format!("/dev/{}", i.trim());

                        log_rule_token!(
                            debug,
                            token.read().unwrap().as_ref().unwrap(),
                            format!("add DEVLINK '{}'", devlink)
                        );

                        execute_err!(
                            token.read().unwrap().as_ref().unwrap(),
                            device.borrow().add_devlink(&devlink)
                        )?;
                    }
                }

                Ok(true)
            }
            AssignAttr => {
                let buf = if let Ok(v) =
                    resolve_subsystem_kernel(token_attribute.as_ref().unwrap(), false)
                {
                    v
                } else {
                    let syspath = execute_err!(
                        token.read().unwrap().as_ref().unwrap(),
                        device.borrow().get_syspath()
                    )?;
                    format!("{}/{}", syspath, token_attribute.as_ref().unwrap())
                };

                let sysattr = match sysattr_subdir_subst(&buf) {
                    Ok(s) => s,
                    Err(e) => {
                        log_rule_token!(
                            error,
                            token.read().unwrap().as_ref().unwrap(),
                            format!(
                                "could not find matching sysattr '{}': {}",
                                token_attribute.as_ref().unwrap(),
                                e
                            )
                        );
                        return Ok(true);
                    }
                };

                let value = match current_unit
                    .as_ref()
                    .unwrap()
                    .apply_format(&token_value, false)
                {
                    Ok(v) => v,
                    Err(e) => {
                        log_rule_token!(
                            error,
                            token.read().unwrap().as_ref().unwrap(),
                            format!("failed to apply formatter: ({})", e)
                        );
                        return Ok(true);
                    }
                };

                log_rule_token!(
                    debug,
                    token.read().unwrap().as_ref().unwrap(),
                    format!("ATTR '{}' is set to '{}'", sysattr, value)
                );

                execute_err!(
                    token.read().unwrap().as_ref().unwrap(),
                    write_string_file(&sysattr, value).context(IoSnafu { filename: sysattr })
                )?;

                Ok(true)
            }
            AssignRunBuiltin | AssignRunProgram => {
                if current_unit.as_ref().unwrap().is_run_final() {
                    return Ok(true);
                }

                if token.read().unwrap().as_ref().unwrap().op == OperatorType::AssignFinal {
                    current_unit.as_ref().unwrap().set_run_final(true);
                }

                if [OperatorType::Assign, OperatorType::AssignFinal]
                    .contains(&token.read().unwrap().as_ref().unwrap().op)
                {
                    current_unit.as_ref().unwrap().builtin_run_list_clear();
                    current_unit.as_ref().unwrap().program_run_list_clear();
                }

                let cmd = match current_unit
                    .as_ref()
                    .unwrap()
                    .apply_format(&token_value, false)
                {
                    Ok(v) => v,
                    Err(e) => {
                        log_rule_token!(
                            error,
                            token.read().unwrap().as_ref().unwrap(),
                            format!("failed to apply formatter: ({})", e)
                        );
                        return Ok(true);
                    }
                };

                if token_attribute.is_some() {
                    current_unit
                        .as_ref()
                        .unwrap()
                        .builtin_run_list_push(cmd.clone());
                    log_rule_token!(
                        debug,
                        token.read().unwrap().as_ref().unwrap(),
                        format!("insert Run builtin '{}'", cmd)
                    );
                } else {
                    current_unit
                        .as_ref()
                        .unwrap()
                        .program_run_list_push(cmd.clone());
                    log_rule_token!(
                        debug,
                        token.read().unwrap().as_ref().unwrap(),
                        format!("insert Run program '{}'", cmd)
                    );
                }

                Ok(true)
            }
            AssignOptionsStaticNode => {
                /*
                 * This token is used to set the permission of static device node after
                 * devmaster started and is not applied during rule executing.
                 */
                Ok(true)
            }
            Label | Goto => Ok(true),
            _ => {
                todo!();
            }
        }
    }

    /// apply rule token on the parent device
    pub(crate) fn apply_rule_token_on_parent(&self) -> Result<bool> {
        debug_assert!(self.current_unit.borrow().is_some());
        debug_assert!(self.current_rule_token.borrow().read().unwrap().is_some());

        let current_unit = self.current_unit.borrow();
        current_unit
            .as_ref()
            .unwrap()
            .set_parent(Some(current_unit.as_ref().unwrap().get_device()));

        let head = self.current_rule_token.borrow().clone();
        let mut match_rst = true;

        loop {
            // udev try to traverse the following parent tokens
            // this seems useless and redundant
            for token in RuleToken::iter(head.clone()) {
                if !token.read().unwrap().as_ref().unwrap().is_for_parents() {
                    return Ok(true);
                }

                *self.current_rule_token.borrow_mut() = token;
                if !self.apply_rule_token(current_unit.as_ref().unwrap().get_parent().unwrap())? {
                    match_rst = false;
                    break;
                }
            }

            if match_rst {
                return Ok(true);
            }

            let tmp = current_unit.as_ref().unwrap().get_parent().unwrap();
            match tmp.borrow().get_parent() {
                Ok(d) => {
                    current_unit.as_ref().unwrap().set_parent(Some(d));
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
    pub(crate) fn execute_run(&self) {
        self.execute_run_builtin();
        self.execute_run_program();
    }

    pub(crate) fn execute_run_builtin(&self) {
        debug_assert!(self.current_unit.borrow().is_some());

        let current_unit = self.current_unit.borrow();
        let device = current_unit.as_ref().unwrap().get_device();

        /*
         * todo: redundant string vector clone
         */
        for builtin_str in &current_unit.as_ref().unwrap().builtin_run_list_iter() {
            if let Ok(builtin) = builtin_str.parse::<BuiltinCommand>() {
                let argv = match shell_words::split(builtin_str) {
                    Ok(ret) => ret,
                    Err(e) => {
                        log_dev!(
                            debug,
                            device.borrow(),
                            format!("Failed to run builtin command '{}': {}", builtin_str, e)
                        );
                        continue;
                    }
                };

                log_dev!(
                    debug,
                    device.borrow(),
                    format!("Running builtin command '{}'", builtin_str)
                );

                if let Err(e) = self.builtin_mgr.run(
                    current_unit.as_ref().unwrap(),
                    builtin,
                    argv.len() as i32,
                    argv,
                    false,
                ) {
                    log_dev!(
                        debug,
                        &device.borrow(),
                        format!("Failed to run builtin command '{}': '{}'", builtin_str, e)
                    );
                }
            }
        }
    }

    pub(crate) fn execute_run_program(&self) {
        let current_unit = self.current_unit.borrow();
        let device = current_unit.as_ref().unwrap().get_device();

        /*
         * todo: redundant string vector clone
         */
        for cmd_str in &current_unit.as_ref().unwrap().program_run_list_iter() {
            log_dev!(
                debug,
                device.borrow(),
                format!("Running program '{}'", cmd_str)
            );

            if let Err(e) = spawn(
                cmd_str,
                Duration::from_secs(self.unit_spawn_timeout_usec),
                current_unit.as_ref().unwrap(),
            ) {
                log_dev!(
                    debug,
                    device.borrow(),
                    format!("Failed to run program '{}': '{}'", cmd_str, e)
                );
            }
        }
    }
}

impl RuleToken {
    fn pattern_match(&self, s: &str) -> bool {
        debug_assert!(self.r#type < TokenType::MatchTest || self.r#type == TokenType::MatchResult);

        let mut value_match = false;

        /*
         * fnmatch ffi function requires the raw string pointer as the input parameter, as it is in C language.
         *
         * However, C strings commonly end with '\0', while it is not in Rust. This is because Rust
         * string objects use a field to indicate string length, thus string literals will be arranged
         * tightly without '\0' as delimiter. Thus transferring normal Rust string to fnmatch ffi function
         * will append garbled characters.
         *
         * To deal with this problem, create a temporary String with '\0' as suffix and transfer
         * this String to fnmatch.
         * */
        let source = format!("{}\0", s);

        let value = self.get_token_value();

        for sub in value.split('|') {
            if sub.is_empty() {
                return (self.op == OperatorType::Nomatch) ^ (s.is_empty());
            }

            /* As mentioned above. */
            let pattern = format!("{}\0", sub);

            if 0 == unsafe {
                fnmatch_sys::fnmatch(
                    pattern.as_ptr() as *const c_char,
                    source.as_ptr() as *const c_char,
                    0,
                )
            } {
                value_match = true;
                break;
            }
        }

        (self.op == OperatorType::Nomatch) ^ value_match
    }

    fn attr_match(&self, device: Rc<RefCell<Device>>, unit: &ExecuteUnit) -> Result<bool> {
        debug_assert!(self.attr.is_some());

        let attr = self.get_token_attribute().unwrap();

        let val =
            match self.attr_subst_type {
                SubstituteType::Plain => {
                    if let Ok(v) = device.borrow().get_sysattr_value(&attr).map_err(|e| {
                        Error::RulesExecuteError {
                            msg: format!("failed to match sysattr: ({})", e),
                            errno: e.get_errno(),
                        }
                    }) {
                        v
                    } else {
                        return Ok(false);
                    }
                }
                SubstituteType::Format => {
                    let attr_name =
                        unit.apply_format(&attr, false)
                            .map_err(|e| Error::RulesExecuteError {
                                msg: format!("failed to match sysattr: ({})", e),
                                errno: e.get_errno(),
                            })?;
                    if let Ok(v) = device.borrow().get_sysattr_value(&attr_name).map_err(|e| {
                        Error::RulesExecuteError {
                            msg: format!("failed to match sysattr: ({})", e),
                            errno: e.get_errno(),
                        }
                    }) {
                        v
                    } else {
                        return Ok(false);
                    }
                }
                SubstituteType::Subsys => {
                    resolve_subsystem_kernel(&attr, true).map_err(|e| Error::RulesExecuteError {
                        msg: format!("failed to match sysattr: ({})", e),
                        errno: e.get_errno(),
                    })?
                }
                _ => {
                    return Err(Error::RulesExecuteError {
                        msg: "invalid substitute type.".to_string(),
                        errno: Errno::EINVAL,
                    })
                }
            };

        Ok(self.pattern_match(&val))
    }
}

/// tokens iterator
struct RuleTokenIter {
    token: Arc<RwLock<Option<RuleToken>>>,
}

impl RuleToken {
    fn iter(token: Arc<RwLock<Option<RuleToken>>>) -> RuleTokenIter {
        RuleTokenIter { token }
    }
}

impl Iterator for RuleTokenIter {
    type Item = Arc<RwLock<Option<RuleToken>>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.token.read().unwrap().is_some() {
            let ret = Some(self.token.clone());
            let next = self.token.read().unwrap().as_ref().unwrap().next.clone();
            self.token = next;
            return ret;
        }

        None
    }
}

impl Rules {
    pub(crate) fn apply_static_dev_permission(&self) -> Result<()> {
        for file in self.iter() {
            for line in file.read().unwrap().as_ref().unwrap().iter() {
                line.read()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .apply_static_dev_permission()?;
            }
        }

        Ok(())
    }
}

impl RuleLine {
    fn apply_static_dev_permission(&self) -> Result<()> {
        if !self.r#type.intersects(RuleLineType::HAS_STATIC_NODE) {
            return Ok(());
        }

        let mut uid: Option<Uid> = None;
        let mut gid: Option<Gid> = None;
        let mut mode: Option<mode_t> = None;
        let mut tags: Vec<String> = vec![];

        for token in self.iter() {
            match token.read().unwrap().as_ref().unwrap().r#type {
                AssignOwnerId => {
                    let v = execute_err!(
                        token.read().unwrap().as_ref().unwrap(),
                        token
                            .read()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .get_token_value()
                            .parse::<uid_t>()
                            .context(ParseIntSnafu)
                    )?;
                    uid = Some(Uid::from_raw(v));
                }
                AssignGroupId => {
                    let v = execute_err!(
                        token.read().unwrap().as_ref().unwrap(),
                        token
                            .read()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .get_token_value()
                            .parse::<gid_t>()
                            .context(ParseIntSnafu)
                    )?;
                    gid = Some(Gid::from_raw(v));
                }
                AssignModeId => {
                    let v = execute_err!(
                        token.read().unwrap().as_ref().unwrap(),
                        mode_t::from_str_radix(
                            &token.read().unwrap().as_ref().unwrap().get_token_value(),
                            8
                        )
                        .context(ParseIntSnafu)
                    )?;
                    mode = Some(v);
                }
                AssignTag => {
                    tags.push(token.read().unwrap().as_ref().unwrap().get_token_value());
                }
                AssignOptionsStaticNode => {
                    static_node_apply_permissions(
                        token.read().unwrap().as_ref().unwrap().get_token_value(),
                        mode,
                        uid,
                        gid,
                        &tags,
                    )?;
                }
                _ => {
                    // do nothing for other types of token
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::FormatSubstitutionType;

    #[test]
    #[ignore]
    fn test_subst_format() {
        let device = Rc::new(RefCell::new(Device::from_path("/dev/sda1").unwrap()));
        let unit = ExecuteUnit::new(device);
        println!(
            "{:?}",
            unit.subst_format(FormatSubstitutionType::Devnode, None)
                .unwrap()
        );
        println!(
            "{:?}",
            unit.subst_format(
                FormatSubstitutionType::Attr,
                Some("[net/lo]address".to_string())
            )
            .unwrap()
        );

        let device = Rc::new(RefCell::new(
            Device::from_subsystem_sysname("net", "lo").unwrap(),
        ));
        let unit = ExecuteUnit::new(device);
        println!(
            "{:?}",
            unit.subst_format(FormatSubstitutionType::Attr, Some("address".to_string()))
                .unwrap()
        );
    }

    #[test]
    fn test_apply_format() {
        let device = Rc::new(RefCell::new(
            Device::from_subsystem_sysname("net", "lo").unwrap(),
        ));
        let unit = ExecuteUnit::new(device);
        // test long substitution formatter
        // $kernel
        assert_eq!(unit.apply_format("$kernel", false).unwrap(), "lo");
        // $number
        assert_eq!(unit.apply_format("$number", false).unwrap(), "");
        // $devpath
        assert_eq!(
            unit.apply_format("$devpath", false).unwrap(),
            "/devices/virtual/net/lo"
        );
        // $id
        assert_eq!(unit.apply_format("$id", false).unwrap(), "");
        // $driver
        assert_eq!(unit.apply_format("$driver", false).unwrap(), "");
        // $attr{sysattr}
        assert_eq!(
            unit.apply_format("$attr{address}", false).unwrap(),
            "00:00:00:00:00:00"
        );
        // $env{key}
        assert_eq!(
            unit.apply_format("$env{DEVPATH}", false).unwrap(),
            "/devices/virtual/net/lo"
        );
        // $major
        assert_eq!(unit.apply_format("$major", false).unwrap(), "0");
        // $minor
        assert_eq!(unit.apply_format("$minor", false).unwrap(), "0");
        // $result
        assert_eq!(unit.apply_format("$result", false).unwrap(), "");
        // $result{index}
        assert_eq!(unit.apply_format("$result{1}", false).unwrap(), "");
        // $result{index+}
        assert_eq!(unit.apply_format("$result{1+}", false).unwrap(), "");
        // $parent
        assert_eq!(unit.apply_format("$parent", false).unwrap(), "");
        // $name
        assert_eq!(unit.apply_format("$name", false).unwrap(), "lo");
        // $links
        assert_eq!(unit.apply_format("$links", false).unwrap(), "");
        // $root
        assert_eq!(unit.apply_format("$root", false).unwrap(), "/dev");
        // $sys
        assert_eq!(unit.apply_format("$sys", false).unwrap(), "/sys");
        // $devnode
        assert_eq!(unit.apply_format("$devnode", false).unwrap(), "");

        // test short substitution formatter
        // %k
        assert_eq!(unit.apply_format("%k", false).unwrap(), "lo");
        // %n
        assert_eq!(unit.apply_format("%n", false).unwrap(), "");
        // %p
        assert_eq!(
            unit.apply_format("%p", false).unwrap(),
            "/devices/virtual/net/lo"
        );
        // %b
        assert_eq!(unit.apply_format("%b", false).unwrap(), "");
        // %d
        assert_eq!(unit.apply_format("%d", false).unwrap(), "");
        // %s{sysattr}
        assert_eq!(
            unit.apply_format("%s{address}", false).unwrap(),
            "00:00:00:00:00:00"
        );
        // %E{key}
        assert_eq!(
            unit.apply_format("%E{DEVPATH}", false).unwrap(),
            "/devices/virtual/net/lo"
        );
        // %M
        assert_eq!(unit.apply_format("%M", false).unwrap(), "0");
        // %m
        assert_eq!(unit.apply_format("%m", false).unwrap(), "0");
        // %c
        assert_eq!(unit.apply_format("%c", false).unwrap(), "");
        // %c{index}
        assert_eq!(unit.apply_format("%c{1}", false).unwrap(), "");
        // %c{index+}
        assert_eq!(unit.apply_format("%c{1+}", false).unwrap(), "");
        // %P
        assert_eq!(unit.apply_format("%P", false).unwrap(), "");
        // %D
        assert_eq!(unit.apply_format("%D", false).unwrap(), "lo");
        // %L
        assert_eq!(unit.apply_format("%L", false).unwrap(), "");
        // %r
        assert_eq!(unit.apply_format("%r", false).unwrap(), "/dev");
        // %S
        assert_eq!(unit.apply_format("%S", false).unwrap(), "/sys");
        // %N
        assert_eq!(unit.apply_format("%N", false).unwrap(), "");

        // $$
        assert_eq!(unit.apply_format("$$", false).unwrap(), "$");
        // %%
        assert_eq!(unit.apply_format("%%", false).unwrap(), "%");
    }

    #[test]
    #[ignore]
    fn test_apply_format_2() {
        let device = Rc::new(RefCell::new(
            Device::from_subsystem_sysname("block", "sda1").unwrap(),
        ));
        let unit = ExecuteUnit::new(device);
        assert_eq!(unit.apply_format("$number", false).unwrap(), "1");
        assert_eq!(unit.apply_format("$major", false).unwrap(), "8");
        assert_eq!(unit.apply_format("$minor", false).unwrap(), "1");
        assert_eq!(unit.apply_format("$driver", false).unwrap(), "");
        assert_eq!(unit.apply_format("$id", false).unwrap(), "");
        assert_eq!(unit.apply_format("$parent", false).unwrap(), "sda");
        assert_eq!(unit.apply_format("$devnode", false).unwrap(), "/dev/sda1");
    }
}
