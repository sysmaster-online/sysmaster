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

use super::{
    FormatSubstitutionType, OperatorType, RuleFile, RuleLine, RuleToken, Rules, SubstituteType,
    TokenType::*,
};
use crate::{
    builtin::{BuiltinCommand, BuiltinManager, Netlink},
    error::{Error, Result},
    log_rule_token_debug, log_rule_token_error,
    rules::FORMAT_SUBST_TABLE,
    utils::{
        get_property_from_string, replace_chars, resolve_subsystem_kernel, spawn,
        sysattr_subdir_subst, DEVMASTER_LEGAL_CHARS,
    },
};
use device::{Device, DeviceAction};
use libc::mode_t;
use std::{
    borrow::BorrowMut,
    cell::RefCell,
    collections::HashMap,
    fs::OpenOptions,
    io::Read,
    os::unix::fs::PermissionsExt,
    sync::{Arc, Mutex, RwLock},
    time::{Duration, SystemTime},
};

use crate::device_trace;
use crate::{
    execute_err, execute_err_ignore_ENOENT, execute_none, subst_format_map_err_ignore,
    subst_format_map_none,
};
use nix::errno::Errno;

/// the process unit on device uevent
#[allow(missing_docs, dead_code)]
struct ExecuteUnit {
    device: Arc<Mutex<Device>>,
    parent: Option<Arc<Mutex<Device>>>,
    // device_db_clone: Option<Device>,
    name: String,
    program_result: String,
    // mode: mode_t,
    // uid: uid_t,
    // gid: gid_t,
    // seclabel_list: HashMap<String, String>,
    // run_list: HashMap<String, String>,
    // exec_delay_usec: useconds_t,
    birth_sec: SystemTime,
    rtnl: RefCell<Option<Netlink>>,
    builtin_run: u32,
    /// set mask bit to 1 if the builtin failed or returned false
    builtin_ret: u32,
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
            name: String::default(),
            program_result: String::default(),
            // mode: (),
            // uid: (),
            // gid: (),
            // seclabel_list: (),
            // run_list: (),
            // exec_delay_usec: (),
            birth_sec: SystemTime::now(),
            rtnl: RefCell::new(None),
            builtin_run: 0,
            builtin_ret: 0,
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

    /// apply runtime substitution on all formatters in the string
    pub fn apply_format(&self, src: &String, replace_whitespace: bool) -> Result<String> {
        let mut idx: usize = 0;
        let mut ret = String::new();
        while idx < src.len() {
            match Self::get_subst_type(src, &mut idx, false)? {
                Some((subst, attr)) => {
                    let v = self.subst_format(subst, attr).map_err(|e| {
                        log::debug!("failed to apply format: ({})", e);
                        e
                    })?;
                    if replace_whitespace {
                        ret += v.replace(' ', "_").as_str();
                    } else {
                        ret += v.as_str();
                    }
                }
                None => {
                    ret.push(src.chars().nth(idx).unwrap());
                    idx += 1;
                }
            }
        }

        Ok(ret)
    }

    fn subst_format(
        &self,
        subst_type: FormatSubstitutionType,
        attribute: Option<String>,
    ) -> Result<String> {
        let mut device = self.device.lock().unwrap();
        match subst_type {
            FormatSubstitutionType::Devnode => subst_format_map_err_ignore!(
                device.get_devname(),
                "devnode",
                Errno::ENOENT,
                String::default()
            ),
            FormatSubstitutionType::Attr => {
                if attribute.is_none() {
                    return Err(Error::RulesExecuteError {
                        msg: "Attribute can not be empty for 'attr' formatter.".to_string(),
                        errno: Errno::EINVAL,
                    });
                }

                // try to read attribute value form path '[<SUBSYSTEM>/[SYSNAME]]<ATTRIBUTE>'
                let value =
                    if let Ok(v) = resolve_subsystem_kernel(&attribute.clone().unwrap(), true) {
                        v
                    } else if let Ok(v) = device.get_sysattr_value(attribute.clone().unwrap()) {
                        v
                    } else if self.parent.is_some() {
                        // try to get sysattr upwards
                        // we did not check whether self.parent is equal to self.device
                        // this perhaps will result in problems
                        if let Ok(v) = self
                            .parent
                            .clone()
                            .unwrap()
                            .as_ref()
                            .lock()
                            .unwrap()
                            .get_sysattr_value(attribute.clone().unwrap())
                        {
                            v
                        } else {
                            return Ok(String::default());
                        }
                    } else {
                        return Ok(String::default());
                    };

                let value = replace_chars(value.trim_end(), DEVMASTER_LEGAL_CHARS);

                Ok(value)
            }
            FormatSubstitutionType::Env => {
                if attribute.is_none() {
                    return Err(Error::RulesExecuteError {
                        msg: "Attribute can not be empty for 'env' formatter.".to_string(),
                        errno: Errno::EINVAL,
                    });
                }

                subst_format_map_err_ignore!(
                    device.get_property_value(attribute.unwrap()),
                    "env",
                    Errno::ENOENT,
                    String::default()
                )
            }
            FormatSubstitutionType::Kernel => {
                subst_format_map_none!(device.get_sysname(), "kernel", String::default())
            }
            FormatSubstitutionType::KernelNumber => subst_format_map_err_ignore!(
                device.get_sysnum(),
                "number",
                Errno::ENOENT,
                String::default()
            ),
            FormatSubstitutionType::Driver => {
                if self.parent.is_none() {
                    return Ok(String::default());
                }

                subst_format_map_err_ignore!(
                    self.parent.clone().unwrap().lock().unwrap().get_driver(),
                    "driver",
                    Errno::ENOENT,
                    String::default()
                )
            }
            FormatSubstitutionType::Devpath => {
                subst_format_map_none!(device.get_devpath(), "devpath", String::default())
            }
            FormatSubstitutionType::Id => {
                if self.parent.is_none() {
                    return Ok(String::default());
                }

                subst_format_map_none!(
                    self.parent.clone().unwrap().lock().unwrap().get_sysname(),
                    "id",
                    String::default()
                )
            }
            FormatSubstitutionType::Major | FormatSubstitutionType::Minor => {
                subst_format_map_err_ignore!(
                    device.get_devnum().map(|n| {
                        match subst_type {
                            FormatSubstitutionType::Major => nix::sys::stat::major(n).to_string(),
                            _ => nix::sys::stat::minor(n).to_string(),
                        }
                    }),
                    "major|minor",
                    Errno::ENOENT,
                    0.to_string()
                )
            }
            FormatSubstitutionType::Result => {
                if self.program_result.is_empty() {
                    return Ok(String::default());
                }

                let (index, plus) = match attribute {
                    Some(a) => {
                        if a.ends_with('+') {
                            let idx = match a[0..a.len() - 1].parse::<usize>() {
                                Ok(i) => i,
                                Err(_) => {
                                    return Err(Error::RulesExecuteError {
                                        msg: format!("invalid index {}", a),
                                        errno: Errno::EINVAL,
                                    })
                                }
                            };
                            (idx, true)
                        } else {
                            let idx = match a[0..a.len()].parse::<usize>() {
                                Ok(i) => i,
                                Err(_) => {
                                    return Err(Error::RulesExecuteError {
                                        msg: format!("invalid index {}", a),
                                        errno: Errno::EINVAL,
                                    })
                                }
                            };
                            (idx, false)
                        }
                    }
                    None => (0, true),
                };

                let result = self.program_result.trim();
                let mut ret = String::new();
                for (i, p) in result.split_whitespace().enumerate() {
                    if !plus {
                        if i == index {
                            return Ok(p.to_string());
                        }
                    } else if i >= index {
                        ret += p;
                        ret += " ";
                    }
                }
                let ret = ret.trim_end().to_string();
                if ret.is_empty() {
                    log::debug!("the {}th part of result string is not found.", index)
                }
                Ok(ret)
            }
            FormatSubstitutionType::Parent => {
                let parent = match device.get_parent() {
                    Ok(p) => p,
                    Err(e) => {
                        if e.get_errno() == Errno::ENOENT {
                            return Ok(String::default());
                        }

                        return Err(Error::RulesExecuteError {
                            msg: format!("failed to substitute formatter 'parent': ({})", e),
                            errno: e.get_errno(),
                        });
                    }
                };
                let devname = parent.lock().unwrap().get_devname();
                subst_format_map_err_ignore!(devname, "parent", Errno::ENOENT, String::default())
                    .map(|v| v.trim_start_matches("/dev/").to_string())
            }
            FormatSubstitutionType::Name => {
                if !self.name.is_empty() {
                    Ok(self.name.clone())
                } else if let Ok(devname) = device.get_devname() {
                    Ok(devname.trim_start_matches("/dev/").to_string())
                } else {
                    subst_format_map_none!(device.get_sysname(), "name", String::default())
                }
            }
            FormatSubstitutionType::Links => {
                let mut ret = String::new();
                for link in device.devlinks.iter() {
                    ret += link.trim_start_matches("/dev/");
                    ret += " ";
                }
                Ok(ret.trim_end().to_string())
            }
            FormatSubstitutionType::Root => Ok("/dev".to_string()),
            FormatSubstitutionType::Sys => Ok("/sys".to_string()),
            FormatSubstitutionType::Invalid => Err(Error::RulesExecuteError {
                msg: "invalid substitution formatter type.".to_string(),
                errno: Errno::EINVAL,
            }),
        }
    }

    fn get_subst_type(
        s: &String,
        idx: &mut usize,
        strict: bool,
    ) -> Result<Option<(FormatSubstitutionType, Option<String>)>> {
        if *idx >= s.len() {
            return Err(Error::RulesExecuteError {
                msg: "the idx is greater than the string length".to_string(),
                errno: Errno::EINVAL,
            });
        }

        let mut subst = FormatSubstitutionType::Invalid;
        let mut attr: Option<String> = None;
        let mut idx_b = *idx;

        if s.chars().nth(idx_b) == Some('$') {
            idx_b += 1;
            if s.chars().nth(idx_b) == Some('$') {
                *idx = idx_b;
                return Ok(None);
            }

            if let Some(sub) = s.get(idx_b..) {
                for ent in FORMAT_SUBST_TABLE.iter() {
                    if sub.starts_with(ent.0) {
                        subst = ent.2;
                        idx_b += ent.0.len();
                        break;
                    }
                }
            }
        } else if s.chars().nth(idx_b) == Some('%') {
            idx_b += 1;
            if s.chars().nth(idx_b) == Some('%') {
                *idx = idx_b;
                return Ok(None);
            }

            if let Some(sub) = s.get(idx_b..) {
                for ent in FORMAT_SUBST_TABLE.iter() {
                    if sub.starts_with(ent.1) {
                        subst = ent.2;
                        idx_b += 1;
                        break;
                    }
                }
            }
        } else {
            return Ok(None);
        }

        if subst == FormatSubstitutionType::Invalid {
            if strict {
                return Err(Error::RulesExecuteError {
                    msg: "single $ or % symbol is invalid.".to_string(),
                    errno: Errno::EINVAL,
                });
            } else {
                return Ok(None);
            }
        }

        if s.chars().nth(idx_b) == Some('{') {
            let left = idx_b + 1;
            let right = if let Some(sub) = s.get(left..) {
                match sub.find('}') {
                    Some(i) => left + i,
                    None => {
                        return Err(Error::RulesExecuteError {
                            msg: "unclosed brackets.".to_string(),
                            errno: Errno::EINVAL,
                        })
                    }
                }
            } else {
                return Err(Error::RulesExecuteError {
                    msg: "unclosed brackets.".to_string(),
                    errno: Errno::EINVAL,
                });
            };

            attr = Some(s.get(left..right).unwrap().to_string());
            idx_b = right + 1;
        }

        *idx = idx_b;
        Ok(Some((subst, attr)))
    }
}

/// manage processing units
pub struct ExecuteManager {
    rules: Arc<RwLock<Rules>>,
    builtin_mgr: BuiltinManager,

    current_rule_file: Option<Arc<RwLock<RuleFile>>>,
    current_rule_line: Option<Arc<RwLock<RuleLine>>>,
    current_rule_token: Option<Arc<RwLock<RuleToken>>>,

    current_unit: Option<ExecuteUnit>,

    properties: HashMap<String, String>,

    unit_spawn_timeout_usec: u64,
}

impl ExecuteManager {
    /// create a execute manager object
    pub fn new(rules: Arc<RwLock<Rules>>) -> ExecuteManager {
        let builtin_mgr = BuiltinManager::new();

        builtin_mgr.init();

        ExecuteManager {
            rules,
            builtin_mgr,
            current_rule_file: None,
            current_rule_line: None,
            current_rule_token: None,
            current_unit: None,
            properties: HashMap::new(),
            unit_spawn_timeout_usec: 3,
        }
    }

    /// process a device object
    pub fn process_device(&mut self, device: Arc<Mutex<Device>>) -> Result<()> {
        log::debug!(
            "{}",
            device_trace!("Start processing device", device.as_ref().lock().unwrap(),)
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

        // only apply rule token on parent device once
        // that means if some a parent device matches the token, do not match any parent tokens in the following
        let mut parents_done = false;

        for token in RuleToken::iter(self.current_rule_token.clone()) {
            self.current_rule_token = Some(token.clone());

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
                    // if current rule token does not match, abort applying the rest tokens in this line
                    log_rule_token_debug!(token.as_ref().read().unwrap(), "fails to match.");

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
                log_rule_token_debug!(token.as_ref().read().unwrap(), "fails to match.");

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

        let token = self
            .current_rule_token
            .as_ref()
            .unwrap()
            .as_ref()
            .read()
            .unwrap();

        log_rule_token_debug!(token, "applying token");

        match token_type {
            MatchAction => {
                let action = execute_err!(
                    token,
                    device.as_ref().lock().unwrap().borrow_mut().get_action()
                )?;

                Ok(token.pattern_match(&action.to_string()))
            }
            MatchDevpath => {
                let devpath = execute_none!(
                    token,
                    device.as_ref().lock().unwrap().borrow_mut().get_devpath(),
                    "DEVPATH"
                )?;

                Ok(token.pattern_match(&devpath))
            }
            MatchKernel | MatchParentsKernel => {
                let sysname = execute_none!(
                    token,
                    device.as_ref().lock().unwrap().borrow_mut().get_sysname(),
                    "SYSNAME"
                )?;

                Ok(token.pattern_match(&sysname))
            }
            MatchDevlink => {
                for devlink in device.as_ref().lock().unwrap().borrow_mut().devlinks.iter() {
                    if token.pattern_match(devlink) ^ (token.op == OperatorType::Nomatch) {
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
                let value = match device
                    .as_ref()
                    .lock()
                    .unwrap()
                    .borrow_mut()
                    .get_property_value(token.attr.clone().unwrap())
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
                for tag in device
                    .as_ref()
                    .lock()
                    .unwrap()
                    .borrow_mut()
                    .current_tags
                    .iter()
                {
                    if token.pattern_match(tag) ^ (token.op == OperatorType::Nomatch) {
                        return Ok(token.op == OperatorType::Match);
                    }
                }

                Ok(token.op == OperatorType::Nomatch)
            }
            MatchSubsystem | MatchParentsSubsystem => {
                let subsystem = execute_err_ignore_ENOENT!(
                    token,
                    device.as_ref().lock().unwrap().borrow_mut().get_subsystem()
                )?;

                Ok(token.pattern_match(&subsystem))
            }
            MatchDriver | MatchParentsDriver => {
                let driver = execute_err_ignore_ENOENT!(
                    token,
                    device.as_ref().lock().unwrap().borrow_mut().get_driver()
                )?;

                Ok(token.pattern_match(&driver))
            }
            MatchAttr | MatchParentsAttr => {
                token.attr_match(device, self.current_unit.as_ref().unwrap())
            }
            MatchTest => {
                let mut val = match self
                    .current_unit
                    .as_ref()
                    .unwrap()
                    .apply_format(&token.value, false)
                {
                    Ok(v) => v,
                    Err(e) => {
                        log::debug!("failed to apply formatter: ({})", e);
                        return Ok(token.op == OperatorType::Nomatch);
                    }
                };

                // if the value is not an absolute path, try to find it under sysfs
                if !val.starts_with('/') {
                    val = match resolve_subsystem_kernel(&val, false) {
                        Ok(v) => v,
                        Err(_) => {
                            // only throw out error when getting the syspath of device
                            let syspath = execute_none!(
                                token,
                                device.as_ref().lock().unwrap().get_syspath(),
                                "SYSPATH"
                            )
                            .map_err(|e| {
                                log_rule_token_debug!(token, "failed to apply token.");
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
                            return Ok(token.op == OperatorType::Nomatch);
                        }

                        if let Some(attr) = &token.attr {
                            let mode = mode_t::from_str_radix(attr, 8).unwrap_or_else(|_| {
                                log::debug!("failed to parse mode: {}", attr);
                                0
                            });

                            let metadata = match std::fs::metadata(path) {
                                Ok(m) => m,
                                Err(_) => {
                                    return Ok(token.op == OperatorType::Nomatch);
                                }
                            };

                            let permissions = metadata.permissions().mode();

                            Ok((mode & permissions > 0) ^ (token.op == OperatorType::Nomatch))
                        } else {
                            Ok(token.op == OperatorType::Match)
                        }
                    }
                    Err(e) => {
                        if e.get_errno() == nix::errno::Errno::ENOENT {
                            return Ok(token.op == OperatorType::Nomatch);
                        }

                        Err(Error::RulesExecuteError {
                            msg: format!("Apply '{}' error: {}", token.content, e),
                            errno: e.get_errno(),
                        })
                    }
                }
            }
            MatchImportFile => {
                let file_name = match self
                    .current_unit
                    .as_ref()
                    .unwrap()
                    .apply_format(&token.value, false)
                {
                    Ok(v) => v,
                    Err(e) => {
                        log::debug!("failed to apply formatter: ({})", e);
                        return Ok(false);
                    }
                };

                log_rule_token_debug!(
                    token,
                    format!("Importing properties from file '{}'", file_name)
                );

                let mut file = match OpenOptions::new().read(true).open(&file_name) {
                    Ok(f) => f,
                    Err(e) => {
                        if e.kind() != std::io::ErrorKind::NotFound {
                            log_rule_token_error!(
                                token,
                                format!("failed to open '{}'.", file_name)
                            );
                            return Err(Error::RulesExecuteError {
                                msg: e.to_string(),
                                errno: Errno::from_i32(e.raw_os_error().unwrap_or_default()),
                            });
                        }

                        return Ok(token.op == OperatorType::Nomatch);
                    }
                };

                let mut content = String::new();
                if let Err(e) = file.read_to_string(&mut content) {
                    log_rule_token_debug!(token, format!("failed to read '{}': {}", file_name, e));
                    return Ok(token.op == OperatorType::Nomatch);
                }

                for line in content.split('\n') {
                    match get_property_from_string(line) {
                        Ok((key, value)) => {
                            execute_err!(
                                token,
                                device.as_ref().lock().unwrap().add_property(key, value)
                            )?;
                        }
                        Err(e) => {
                            log_rule_token_debug!(token, e);
                        }
                    }
                }

                Ok(token.op == OperatorType::Match)
            }
            MatchImportProgram => {
                let cmd = match self
                    .current_unit
                    .as_ref()
                    .unwrap()
                    .apply_format(&token.value, false)
                {
                    Ok(v) => v,
                    Err(e) => {
                        log::debug!("failed to apply formatter: ({})", e);
                        return Ok(false);
                    }
                };

                log_rule_token_debug!(
                    token,
                    format!("Importing properties from output of cmd '{}'", cmd)
                );

                let result = match spawn(&cmd, Duration::from_secs(self.unit_spawn_timeout_usec)) {
                    Ok(s) => {
                        if s.1 < 0 {
                            log_rule_token_debug!(
                                token,
                                format!("command returned {}, ignoring.", s.1)
                            );
                            return Ok(token.op == OperatorType::Nomatch);
                        }
                        s.0
                    }
                    Err(e) => {
                        log_rule_token_debug!(token, format!("failed execute command: ({})", e));
                        return Ok(token.op == OperatorType::Nomatch);
                    }
                };

                for line in result.split('\n') {
                    let line = replace_chars(line.trim_end(), DEVMASTER_LEGAL_CHARS);
                    match get_property_from_string(&line) {
                        Ok((key, value)) => {
                            execute_err!(
                                token,
                                device.as_ref().lock().unwrap().add_property(key, value)
                            )?;
                        }
                        Err(e) => {
                            log_rule_token_debug!(token, e);
                        }
                    }
                }

                Ok(token.op == OperatorType::Match)
            }
            MatchImportBuiltin => {
                let builtin = match token.value.parse::<BuiltinCommand>() {
                    Ok(cmd) => cmd,
                    Err(_) => {
                        log_rule_token_error!(token, "invalid builtin command.");
                        return Ok(false);
                    }
                };

                let mask = 0b1 << builtin as u32;
                let already_run = self.current_unit.as_ref().unwrap().builtin_run;
                let run_result = self.current_unit.as_ref().unwrap().builtin_ret;

                if self.builtin_mgr.run_once(builtin) {
                    if already_run & mask != 0 {
                        log_rule_token_debug!(
                            token,
                            format!(
                                "builtin '{}' can only run once and has run before.",
                                builtin
                            )
                        );
                        return Ok((token.op == OperatorType::Match) ^ (run_result & mask > 0));
                    }

                    self.current_unit.as_mut().unwrap().builtin_run = already_run | mask;
                }

                let cmd = match self
                    .current_unit
                    .as_ref()
                    .unwrap()
                    .apply_format(&token.value, false)
                {
                    Ok(v) => v,
                    Err(e) => {
                        log_rule_token_error!(token, format!("failed to apply formatter: ({})", e));
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

                log_rule_token_debug!(
                    token,
                    format!("Importing properties from builtin cmd '{}'", cmd)
                );

                match self.builtin_mgr.run(
                    self.current_unit.as_ref().unwrap().device.clone(),
                    &mut self.current_unit.as_mut().unwrap().rtnl,
                    builtin,
                    argv.len() as i32,
                    argv,
                    false,
                ) {
                    Ok(ret) => {
                        // if builtin command returned false, set the mask bit to 1
                        self.current_unit.as_mut().unwrap().builtin_ret =
                            run_result | ((!ret as u32) << builtin as u32);
                        Ok((token.op == OperatorType::Nomatch) ^ ret)
                    }
                    Err(e) => {
                        log_rule_token_error!(token, format!("failed to run builtin ({})", e));
                        Ok(token.op == OperatorType::Nomatch)
                    }
                }
            }
            MatchProgram => {
                let cmd = match self
                    .current_unit
                    .as_ref()
                    .unwrap()
                    .apply_format(&token.value, false)
                {
                    Ok(v) => v,
                    Err(e) => {
                        log_rule_token_error!(token, format!("failed to apply formatter: ({})", e));
                        return Ok(false);
                    }
                };

                let result = match spawn(&cmd, Duration::from_secs(self.unit_spawn_timeout_usec)) {
                    Ok(s) => {
                        if s.1 != 0 {
                            return Ok(token.op == OperatorType::Nomatch);
                        }
                        s.0
                    }
                    Err(e) => {
                        log_rule_token_debug!(token, format!("failed to apply token: ({})", e));
                        return Ok(false);
                    }
                };

                let result = replace_chars(result.trim_end(), DEVMASTER_LEGAL_CHARS);

                log::debug!(
                    "\x1b[34mCapture stdout from command '{}': '{}'\x1b[0m",
                    cmd,
                    &result
                );

                self.current_unit.as_mut().unwrap().program_result = result;

                Ok(token.op == OperatorType::Match)
            }
            AssignDevlink => {
                todo!()
            }
            _ => {
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
}

impl RuleToken {
    fn pattern_match(&self, s: &str) -> bool {
        let mut value_match = false;
        for regex in self.value_regex.iter() {
            if regex.is_match(s) {
                value_match = true;
                break;
            }
        }

        (self.op == OperatorType::Nomatch) ^ value_match
    }

    fn attr_match(&self, device: Arc<Mutex<Device>>, unit: &ExecuteUnit) -> Result<bool> {
        let attr = self.attr.clone().unwrap_or_default();

        let val = match self.attr_subst_type {
            SubstituteType::Plain => {
                if let Ok(v) = device
                    .as_ref()
                    .lock()
                    .unwrap()
                    .borrow_mut()
                    .get_sysattr_value(attr)
                    .map_err(|e| Error::RulesExecuteError {
                        msg: format!("failed to match sysattr: ({})", e),
                        errno: e.get_errno(),
                    })
                {
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
                if let Ok(v) = device
                    .as_ref()
                    .lock()
                    .unwrap()
                    .borrow_mut()
                    .get_sysattr_value(attr_name)
                    .map_err(|e| Error::RulesExecuteError {
                        msg: format!("failed to match sysattr: ({})", e),
                        errno: e.get_errno(),
                    })
                {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn test_subst_format() {
        let device = Arc::new(Mutex::new(
            Device::from_path("/dev/sda1".to_string()).unwrap(),
        ));
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

        let device = Arc::new(Mutex::new(
            Device::from_subsystem_sysname("net".to_string(), "lo".to_string()).unwrap(),
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
        let device = Arc::new(Mutex::new(
            Device::from_subsystem_sysname("net".to_string(), "lo".to_string()).unwrap(),
        ));
        let unit = ExecuteUnit::new(device);
        // test long substitution formatter
        // $kernel
        assert_eq!(
            unit.apply_format(&"$kernel".to_string(), false).unwrap(),
            "lo"
        );
        // $number
        assert_eq!(
            unit.apply_format(&"$number".to_string(), false).unwrap(),
            ""
        );
        // $devpath
        assert_eq!(
            unit.apply_format(&"$devpath".to_string(), false).unwrap(),
            "/devices/virtual/net/lo"
        );
        // $id
        assert_eq!(unit.apply_format(&"$id".to_string(), false).unwrap(), "");
        // $driver
        assert_eq!(
            unit.apply_format(&"$driver".to_string(), false).unwrap(),
            ""
        );
        // $attr{sysattr}
        assert_eq!(
            unit.apply_format(&"$attr{address}".to_string(), false)
                .unwrap(),
            "00:00:00:00:00:00"
        );
        // $env{key}
        assert_eq!(
            unit.apply_format(&"$env{DEVPATH}".to_string(), false)
                .unwrap(),
            "/devices/virtual/net/lo"
        );
        // $major
        assert_eq!(
            unit.apply_format(&"$major".to_string(), false).unwrap(),
            "0"
        );
        // $minor
        assert_eq!(
            unit.apply_format(&"$minor".to_string(), false).unwrap(),
            "0"
        );
        // $result
        assert_eq!(
            unit.apply_format(&"$result".to_string(), false).unwrap(),
            ""
        );
        // $result{index}
        assert_eq!(
            unit.apply_format(&"$result{1}".to_string(), false).unwrap(),
            ""
        );
        // $result{index+}
        assert_eq!(
            unit.apply_format(&"$result{1+}".to_string(), false)
                .unwrap(),
            ""
        );
        // $parent
        assert_eq!(
            unit.apply_format(&"$parent".to_string(), false).unwrap(),
            ""
        );
        // $name
        assert_eq!(
            unit.apply_format(&"$name".to_string(), false).unwrap(),
            "lo"
        );
        // $links
        assert_eq!(unit.apply_format(&"$links".to_string(), false).unwrap(), "");
        // $root
        assert_eq!(
            unit.apply_format(&"$root".to_string(), false).unwrap(),
            "/dev"
        );
        // $sys
        assert_eq!(
            unit.apply_format(&"$sys".to_string(), false).unwrap(),
            "/sys"
        );
        // $devnode
        assert_eq!(
            unit.apply_format(&"$devnode".to_string(), false).unwrap(),
            ""
        );

        // test short substitution formatter
        // %k
        assert_eq!(unit.apply_format(&"%k".to_string(), false).unwrap(), "lo");
        // %n
        assert_eq!(unit.apply_format(&"%n".to_string(), false).unwrap(), "");
        // %p
        assert_eq!(
            unit.apply_format(&"%p".to_string(), false).unwrap(),
            "/devices/virtual/net/lo"
        );
        // %b
        assert_eq!(unit.apply_format(&"%b".to_string(), false).unwrap(), "");
        // %d
        assert_eq!(unit.apply_format(&"%d".to_string(), false).unwrap(), "");
        // %s{sysattr}
        assert_eq!(
            unit.apply_format(&"%s{address}".to_string(), false)
                .unwrap(),
            "00:00:00:00:00:00"
        );
        // %E{key}
        assert_eq!(
            unit.apply_format(&"%E{DEVPATH}".to_string(), false)
                .unwrap(),
            "/devices/virtual/net/lo"
        );
        // %M
        assert_eq!(unit.apply_format(&"%M".to_string(), false).unwrap(), "0");
        // %m
        assert_eq!(unit.apply_format(&"%m".to_string(), false).unwrap(), "0");
        // %c
        assert_eq!(unit.apply_format(&"%c".to_string(), false).unwrap(), "");
        // %c{index}
        assert_eq!(unit.apply_format(&"%c{1}".to_string(), false).unwrap(), "");
        // %c{index+}
        assert_eq!(unit.apply_format(&"%c{1+}".to_string(), false).unwrap(), "");
        // %P
        assert_eq!(unit.apply_format(&"%P".to_string(), false).unwrap(), "");
        // %D
        assert_eq!(unit.apply_format(&"%D".to_string(), false).unwrap(), "lo");
        // %L
        assert_eq!(unit.apply_format(&"%L".to_string(), false).unwrap(), "");
        // %r
        assert_eq!(unit.apply_format(&"%r".to_string(), false).unwrap(), "/dev");
        // %S
        assert_eq!(unit.apply_format(&"%S".to_string(), false).unwrap(), "/sys");
        // %N
        assert_eq!(unit.apply_format(&"%N".to_string(), false).unwrap(), "");

        // $$
        assert_eq!(unit.apply_format(&"$$".to_string(), false).unwrap(), "$");
        // %%
        assert_eq!(unit.apply_format(&"%%".to_string(), false).unwrap(), "%");
    }

    #[test]
    #[ignore]
    fn test_apply_format_2() {
        let device = Arc::new(Mutex::new(
            Device::from_subsystem_sysname("block".to_string(), "sda1".to_string()).unwrap(),
        ));
        let unit = ExecuteUnit::new(device);
        assert_eq!(
            unit.apply_format(&"$number".to_string(), false).unwrap(),
            "1"
        );
        assert_eq!(
            unit.apply_format(&"$major".to_string(), false).unwrap(),
            "8"
        );
        assert_eq!(
            unit.apply_format(&"$minor".to_string(), false).unwrap(),
            "1"
        );
        assert_eq!(
            unit.apply_format(&"$driver".to_string(), false).unwrap(),
            ""
        );
        assert_eq!(unit.apply_format(&"$id".to_string(), false).unwrap(), "");
        assert_eq!(
            unit.apply_format(&"$parent".to_string(), false).unwrap(),
            "sda"
        );
        assert_eq!(
            unit.apply_format(&"$devnode".to_string(), false).unwrap(),
            "/dev/sda1"
        );
    }
}
