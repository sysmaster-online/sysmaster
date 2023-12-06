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
    builtin::Netlink,
    error::*,
    log_dev,
    rules::{node::*, *},
    utils::commons::{replace_chars, resolve_subsystem_kernel, DEVMASTER_LEGAL_CHARS},
};
use device::{Device, DeviceAction};
use libc::mode_t;
use snafu::ResultExt;
use std::{
    cell::{Ref, RefCell},
    collections::HashMap,
    rc::Rc,
    time::SystemTime,
};

use crate::subst_format_map_err_ignore;
use futures::stream::TryStreamExt;
use nix::errno::Errno;
use nix::unistd::{Gid, Uid};
use rtnetlink::{new_connection, Handle};

/// the process unit on device uevent
#[allow(missing_docs, dead_code)]
pub struct ExecuteUnit {
    inner: RefCell<ExecuteUnitData>,

    seclabel_list: RefCell<HashMap<String, String>>,
    builtin_run_list: RefCell<Vec<String>>,
    program_run_list: RefCell<Vec<String>>,
}

struct ExecuteUnitData {
    device: Rc<Device>,
    parent: Option<Rc<Device>>,
    device_db_clone: Option<Rc<Device>>,
    name: String,
    program_result: String,
    mode: Option<mode_t>,
    uid: Option<Uid>,
    gid: Option<Gid>,
    _birth_sec: SystemTime,
    builtin_run: u32,
    /// set mask bit to 1 if the builtin failed or returned false
    builtin_ret: u32,
    escape_type: EscapeType,
    watch: bool,
    watch_final: bool,
    group_final: bool,
    owner_final: bool,
    mode_final: bool,
    name_final: bool,
    devlink_final: bool,
    run_final: bool,

    _rtnl: Option<Netlink>,
}

impl ExecuteUnitData {
    fn new(device: Rc<Device>) -> Self {
        ExecuteUnitData {
            device,
            parent: None,
            device_db_clone: None,
            name: String::default(),
            program_result: String::default(),
            mode: None,
            uid: None,
            gid: None,
            _birth_sec: SystemTime::now(),
            builtin_run: 0,
            builtin_ret: 0,
            escape_type: EscapeType::Unset,
            watch: false,
            watch_final: false,
            group_final: false,
            owner_final: false,
            mode_final: false,
            name_final: false,
            devlink_final: false,
            run_final: false,

            _rtnl: None,
        }
    }

    fn get_device(&self) -> Rc<Device> {
        self.device.clone()
    }

    fn get_device_db_clone(&self) -> Rc<Device> {
        debug_assert!(self.device_db_clone.is_some());

        self.device_db_clone.clone().unwrap()
    }

    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn set_name(&mut self, name: String) {
        self.name = name;
    }

    #[allow(dead_code)]
    fn get_builtin_run(&self) -> u32 {
        self.builtin_run
    }

    fn set_builtin_run(&mut self, builtin_run: u32) {
        self.builtin_run = builtin_run;
    }

    fn get_builtin_ret(&self) -> u32 {
        self.builtin_ret
    }

    fn set_builtin_ret(&mut self, builtin_ret: u32) {
        self.builtin_ret = builtin_ret;
    }

    fn get_program_result(&self) -> String {
        self.program_result.clone()
    }

    fn set_program_result(&mut self, result: String) {
        self.program_result = result;
    }

    fn get_escape_type(&self) -> EscapeType {
        self.escape_type
    }

    fn set_escape_type(&mut self, escape_type: EscapeType) {
        self.escape_type = escape_type;
    }

    fn is_watch_final(&self) -> bool {
        self.watch_final
    }

    fn set_watch_final(&mut self, watch_final: bool) {
        self.watch_final = watch_final;
    }

    fn is_watch(&self) -> bool {
        self.watch
    }

    fn set_watch(&mut self, watch: bool) {
        self.watch = watch;
    }

    fn clone_device_db(&mut self) -> Result<()> {
        self.device_db_clone = Some(Rc::new(
            self.device
                .clone_with_db()
                .context(DeviceSnafu)
                .log_dev_error(&self.device, "failed to clone db")?,
        ));

        Ok(())
    }

    fn update_devnode(&mut self, seclabel_list: &HashMap<String, String>) -> Result<()> {
        if let Err(e) = self.device.get_devnum() {
            if e.is_errno(Errno::ENOENT) {
                return Ok(());
            }
            log_dev!(error, self.device, e);
            return Err(Error::Device { source: e });
        }

        if self.uid.is_none() {
            match self.device.get_devnode_uid() {
                Ok(uid) => self.uid = Some(uid),
                Err(e) => {
                    if !e.is_errno(Errno::ENOENT) {
                        return Err(Error::Device { source: e });
                    }
                }
            }
        }

        if self.gid.is_none() {
            match self.device.get_devnode_gid() {
                Ok(gid) => self.gid = Some(gid),
                Err(e) => {
                    if !e.is_errno(Errno::ENOENT) {
                        return Err(Error::Device { source: e });
                    }
                }
            }
        }

        if self.mode.is_none() {
            match self.device.get_devnode_mode() {
                Ok(mode) => self.mode = Some(mode),
                Err(e) => {
                    if !e.is_errno(Errno::ENOENT) {
                        return Err(Error::Device { source: e });
                    }
                }
            }
        }

        let apply_mac = self
            .device
            .get_action()
            .map(|action| action == DeviceAction::Add)
            .unwrap_or(false);

        super::node::node_apply_permissions(
            self.device.clone(),
            apply_mac,
            self.mode,
            self.uid,
            self.gid,
            seclabel_list,
        )?;

        update_node(self.device.clone(), self.device_db_clone.clone().unwrap())
    }

    fn rename_netif(&self) -> Result<bool> {
        let ifindex = match self.device.get_ifindex().context(DeviceSnafu) {
            Ok(ifindex) => ifindex,
            Err(e) => {
                if e.get_errno() == nix::Error::ENOENT {
                    return Ok(false);
                }

                return Err(e);
            }
        };

        let rt = tokio::runtime::Runtime::new().unwrap();

        if let Err(e) = rt.block_on(async {
            let (connection, handle, _) = new_connection().unwrap();
            tokio::spawn(connection);

            set_link_name(handle, ifindex, self.name.clone())
                .await
                .context(RtnetlinkSnafu)
        }) {
            if e.get_errno() == nix::Error::EBUSY {
                log_dev!(
                    info,
                    &self.device,
                    format!(
                        "Network interface '{}' is busy, cannot rename to '{}'",
                        self.device.get_sysname().context(DeviceSnafu)?,
                        self.name.clone(),
                    )
                );
                return Ok(false);
            }

            return Err(e);
        }

        log_dev!(
            info,
            &self.device,
            format!(
                "Network interface '{}' is renamed from '{}' to '{}'",
                self.device.get_ifindex().context(DeviceSnafu)?,
                self.device.get_sysname().context(DeviceSnafu)?,
                self.name.clone(),
            )
        );

        Ok(true)
    }

    /// apply runtime substitution on all formatters in the string
    fn apply_format(&self, src: &str, replace_whitespace: bool) -> Result<String> {
        let mut idx: usize = 0;
        let mut ret = String::new();
        while idx < src.len() {
            match get_subst_type(src, &mut idx, false)? {
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
        match subst_type {
            FormatSubstitutionType::Devnode => subst_format_map_err_ignore!(
                self.device.get_devname(),
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
                let attr = attribute.unwrap();

                // try to read attribute value form path '[<SUBSYSTEM>/[SYSNAME]]<ATTRIBUTE>'
                let value = if let Ok(v) = resolve_subsystem_kernel(&attr, true) {
                    v
                } else if let Ok(v) = self.device.get_sysattr_value(&attr) {
                    v
                } else if self.parent.is_some() {
                    // try to get sysattr upwards
                    // we did not check whether self.parent is equal to self.device
                    // this perhaps will result in problems
                    if let Ok(v) = self.parent.clone().unwrap().get_sysattr_value(&attr) {
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
                    self.device.get_property_value(&attribute.unwrap()),
                    "env",
                    Errno::ENOENT,
                    String::default()
                )
            }
            FormatSubstitutionType::Kernel => Ok(self.device.get_sysname().unwrap_or_else(|_| {
                log::debug!("formatter 'kernel' got empty value.");
                "".to_string()
            })),
            FormatSubstitutionType::KernelNumber => subst_format_map_err_ignore!(
                self.device.get_sysnum(),
                "number",
                Errno::ENOENT,
                String::default()
            ),
            FormatSubstitutionType::Driver => {
                if self.parent.is_none() {
                    return Ok(String::default());
                }

                subst_format_map_err_ignore!(
                    self.parent.as_ref().unwrap().get_driver(),
                    "driver",
                    Errno::ENOENT,
                    String::default()
                )
            }
            FormatSubstitutionType::Devpath => Ok(self.device.get_devpath().unwrap_or_else(|_| {
                log::debug!("formatter 'devpath' got empty value.");
                "".to_string()
            })),
            FormatSubstitutionType::Id => {
                if self.parent.is_none() {
                    return Ok(String::default());
                }

                Ok(self
                    .parent
                    .as_ref()
                    .unwrap()
                    .get_sysname()
                    .unwrap_or_else(|_| {
                        log::debug!("formatter 'id' got empty value.");
                        "".to_string()
                    }))
            }
            FormatSubstitutionType::Major | FormatSubstitutionType::Minor => {
                subst_format_map_err_ignore!(
                    self.device.get_devnum().map(|n| {
                        match subst_type {
                            FormatSubstitutionType::Major => nix::sys::stat::major(n).to_string(),
                            _ => nix::sys::stat::minor(n).to_string(),
                        }
                    }),
                    "major|minor",
                    Errno::ENOENT,
                    "0".to_string()
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
                let parent = match self.device.get_parent() {
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
                let devname = parent.get_devname();
                subst_format_map_err_ignore!(devname, "parent", Errno::ENOENT, String::default())
                    .map(|v| v.trim_start_matches("/dev/").to_string())
            }
            FormatSubstitutionType::Name => {
                if !self.name.is_empty() {
                    Ok(self.name.clone())
                } else if let Ok(devname) = self.device.get_devname() {
                    Ok(devname.trim_start_matches("/dev/").to_string())
                } else {
                    Ok(self.device.get_sysname().unwrap_or_else(|_| {
                        log::debug!("formatter 'name' got empty value.");
                        "".to_string()
                    }))
                }
            }
            FormatSubstitutionType::Links => {
                let mut ret = String::new();
                for link in &self.device.devlink_iter() {
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

    fn is_group_final(&self) -> bool {
        self.group_final
    }

    fn set_group_final(&mut self, group_final: bool) {
        self.group_final = group_final;
    }

    fn is_owner_final(&self) -> bool {
        self.owner_final
    }

    fn set_owner_final(&mut self, owner_final: bool) {
        self.owner_final = owner_final;
    }

    fn is_mode_final(&self) -> bool {
        self.mode_final
    }

    fn set_mode_final(&mut self, mode_final: bool) {
        self.mode_final = mode_final;
    }

    fn is_name_final(&self) -> bool {
        self.name_final
    }

    fn set_name_final(&mut self, name_final: bool) {
        self.name_final = name_final;
    }

    fn is_devlink_final(&self) -> bool {
        self.devlink_final
    }

    fn set_devlink_final(&mut self, devlink_final: bool) {
        self.devlink_final = devlink_final;
    }

    fn is_run_final(&self) -> bool {
        self.run_final
    }

    fn set_run_final(&mut self, run_final: bool) {
        self.run_final = run_final;
    }

    fn get_uid(&self) -> Option<nix::unistd::Uid> {
        self.uid
    }

    fn set_uid(&mut self, uid: Option<nix::unistd::Uid>) {
        self.uid = uid;
    }

    fn get_gid(&self) -> Option<nix::unistd::Gid> {
        self.gid
    }

    fn set_gid(&mut self, gid: Option<nix::unistd::Gid>) {
        self.gid = gid;
    }

    fn get_mode(&self) -> Option<libc::mode_t> {
        self.mode
    }

    fn set_mode(&mut self, mode: Option<libc::mode_t>) {
        self.mode = mode;
    }

    fn set_parent(&mut self, parent: Option<Rc<Device>>) {
        self.parent = parent;
    }

    fn get_parent(&self) -> Option<Rc<Device>> {
        self.parent.clone()
    }
}

impl ExecuteUnit {
    /// create a execute unit based on device object
    pub fn new(device: Rc<Device>) -> Self {
        ExecuteUnit {
            seclabel_list: RefCell::new(HashMap::new()),
            builtin_run_list: RefCell::new(vec![]),
            program_run_list: RefCell::new(vec![]),
            inner: RefCell::new(ExecuteUnitData::new(device)),
        }
    }

    /// apply runtime substitution on all formatters in the string
    pub(crate) fn apply_format(&self, src: &str, replace_whitespace: bool) -> Result<String> {
        self.inner.borrow().apply_format(src, replace_whitespace)
    }

    #[allow(dead_code)]
    pub(crate) fn subst_format(
        &self,
        subst_type: FormatSubstitutionType,
        attribute: Option<String>,
    ) -> Result<String> {
        self.inner.borrow().subst_format(subst_type, attribute)
    }

    pub(crate) fn update_devnode(&self) -> Result<()> {
        self.inner
            .borrow_mut()
            .update_devnode(&self.seclabel_list.borrow())
    }

    pub(crate) fn get_device(&self) -> Rc<Device> {
        self.inner.borrow().get_device()
    }

    pub(crate) fn get_device_db_clone(&self) -> Rc<Device> {
        self.inner.borrow().get_device_db_clone()
    }

    pub(crate) fn get_name(&self) -> String {
        self.inner.borrow().get_name()
    }

    pub(crate) fn set_name(&self, name: String) {
        self.inner.borrow_mut().set_name(name)
    }

    pub(crate) fn get_builtin_run(&self) -> u32 {
        self.inner.borrow().get_builtin_ret()
    }

    pub(crate) fn set_builtin_run(&self, builtin_run: u32) {
        self.inner.borrow_mut().set_builtin_run(builtin_run)
    }

    pub(crate) fn get_builtin_ret(&self) -> u32 {
        self.inner.borrow().get_builtin_ret()
    }

    pub(crate) fn set_builtin_ret(&self, builtin_ret: u32) {
        self.inner.borrow_mut().set_builtin_ret(builtin_ret)
    }

    pub(crate) fn get_program_result(&self) -> String {
        self.inner.borrow().get_program_result()
    }

    pub(crate) fn set_program_result(&self, result: String) {
        self.inner.borrow_mut().set_program_result(result)
    }

    pub(crate) fn get_escape_type(&self) -> EscapeType {
        self.inner.borrow().get_escape_type()
    }

    pub(crate) fn set_escape_type(&self, escape_type: EscapeType) {
        self.inner.borrow_mut().set_escape_type(escape_type)
    }

    pub(crate) fn get_watch_final(&self) -> bool {
        self.inner.borrow().is_watch_final()
    }

    pub(crate) fn set_watch_final(&self, watch_final: bool) {
        self.inner.borrow_mut().set_watch_final(watch_final)
    }

    pub(crate) fn clone_device_db(&self) -> Result<()> {
        self.inner.borrow_mut().clone_device_db()
    }

    pub(crate) fn rename_netif(&self) -> Result<bool> {
        self.inner.borrow().rename_netif()
    }

    pub(crate) fn is_watch(&self) -> bool {
        self.inner.borrow().is_watch()
    }

    pub(crate) fn set_watch(&self, watch: bool) {
        self.inner.borrow_mut().set_watch(watch)
    }

    pub(crate) fn is_group_final(&self) -> bool {
        self.inner.borrow().is_group_final()
    }

    pub(crate) fn set_group_final(&self, group_final: bool) {
        self.inner.borrow_mut().set_group_final(group_final)
    }

    pub(crate) fn is_owner_final(&self) -> bool {
        self.inner.borrow().is_owner_final()
    }

    pub(crate) fn set_owner_final(&self, owner_final: bool) {
        self.inner.borrow_mut().set_owner_final(owner_final)
    }

    pub(crate) fn is_mode_final(&self) -> bool {
        self.inner.borrow().is_mode_final()
    }

    pub(crate) fn set_mode_final(&self, mode_final: bool) {
        self.inner.borrow_mut().set_mode_final(mode_final)
    }

    pub(crate) fn is_name_final(&self) -> bool {
        self.inner.borrow().is_name_final()
    }

    pub(crate) fn set_name_final(&self, name_final: bool) {
        self.inner.borrow_mut().set_name_final(name_final)
    }

    pub(crate) fn is_devlink_final(&self) -> bool {
        self.inner.borrow().is_devlink_final()
    }

    pub(crate) fn set_devlink_final(&self, devlink_final: bool) {
        self.inner.borrow_mut().set_devlink_final(devlink_final)
    }

    pub(crate) fn is_run_final(&self) -> bool {
        self.inner.borrow().is_run_final()
    }

    pub(crate) fn set_run_final(&self, run_final: bool) {
        self.inner.borrow_mut().set_run_final(run_final)
    }

    #[allow(dead_code)]
    pub(crate) fn get_uid(&self) -> Option<nix::unistd::Uid> {
        self.inner.borrow().get_uid()
    }

    pub(crate) fn set_uid(&self, uid: Option<nix::unistd::Uid>) {
        self.inner.borrow_mut().set_uid(uid)
    }

    #[allow(dead_code)]
    pub(crate) fn get_gid(&self) -> Option<nix::unistd::Gid> {
        self.inner.borrow().get_gid()
    }

    pub(crate) fn set_gid(&self, gid: Option<nix::unistd::Gid>) {
        self.inner.borrow_mut().set_gid(gid)
    }

    #[allow(dead_code)]
    pub(crate) fn get_mode(&self) -> Option<libc::mode_t> {
        self.inner.borrow().get_mode()
    }

    pub(crate) fn set_mode(&self, mode: Option<libc::mode_t>) {
        self.inner.borrow_mut().set_mode(mode)
    }

    pub(crate) fn builtin_run_list_clear(&self) {
        self.builtin_run_list.borrow_mut().clear();
    }

    pub(crate) fn program_run_list_clear(&self) {
        self.program_run_list.borrow_mut().clear();
    }

    pub(crate) fn builtin_run_list_push(&self, builtin: String) {
        self.builtin_run_list.borrow_mut().push(builtin);
    }

    pub(crate) fn program_run_list_push(&self, program: String) {
        self.program_run_list.borrow_mut().push(program);
    }

    #[allow(dead_code)]
    pub(crate) fn seclabel_list_insert(&self, key: String, value: String) -> Option<String> {
        self.seclabel_list.borrow_mut().insert(key, value)
    }

    #[allow(dead_code)]
    pub(crate) fn seclabel_list_clear(&self) {
        self.seclabel_list.borrow_mut().clear()
    }

    pub(crate) fn set_parent(&self, parent: Option<Rc<Device>>) {
        self.inner.borrow_mut().set_parent(parent)
    }

    pub(crate) fn get_parent(&self) -> Option<Rc<Device>> {
        self.inner.borrow().get_parent()
    }
}

pub(crate) struct VecRefWrapper<'a, T: 'a> {
    r: Ref<'a, Vec<T>>,
}

impl<'a, 'b: 'a, T: 'a> IntoIterator for &'b VecRefWrapper<'a, T> {
    type IntoIter = std::slice::Iter<'a, T>;
    type Item = &'a T;

    fn into_iter(self) -> std::slice::Iter<'a, T> {
        self.r.iter()
    }
}

/// iterator wrapper of hash map in refcell
pub(crate) struct HashMapRefWrapper<'a, K: 'a, V: 'a> {
    r: Ref<'a, HashMap<K, V>>,
}

impl<'a, 'b: 'a, K: 'a, V: 'a> IntoIterator for &'b HashMapRefWrapper<'a, K, V> {
    type IntoIter = std::collections::hash_map::Iter<'a, K, V>;
    type Item = (&'a K, &'a V);

    fn into_iter(self) -> std::collections::hash_map::Iter<'a, K, V> {
        self.r.iter()
    }
}

impl ExecuteUnit {
    pub(crate) fn builtin_run_list_iter(&self) -> VecRefWrapper<String> {
        VecRefWrapper {
            r: self.builtin_run_list.borrow(),
        }
    }

    pub(crate) fn program_run_list_iter(&self) -> VecRefWrapper<String> {
        VecRefWrapper {
            r: self.program_run_list.borrow(),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn seclabel_list_iter(&self) -> HashMapRefWrapper<String, String> {
        HashMapRefWrapper {
            r: self.seclabel_list.borrow(),
        }
    }
}

async fn set_link_name(handle: Handle, ifindex: u32, name: String) -> Result<(), rtnetlink::Error> {
    let mut links = handle.link().get().match_index(ifindex).execute();
    if let Some(link) = links.try_next().await? {
        handle
            .link()
            .set(link.header.index)
            .name(name)
            .execute()
            .await?
    } else {
        log::error!("no link link {} found", name);
    }
    Ok(())
}

fn get_subst_type(
    s: &str,
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::rules::rules_load::tests::create_tmp_file;
    use device::utils::*;
    use nix::sys::stat::{major, minor};
    use std::fs::remove_dir_all;
    use std::path::Path;
    use std::rc::Rc;

    #[test]
    fn test_update_devnode() {
        if let Err(e) = LoopDev::inner_process(
            "/tmp/test_update_devnode_tmpfile",
            1024 * 1024 * 10,
            |dev| {
                let dev = Rc::new(dev.shallow_clone().unwrap());
                let id = dev.get_device_id().unwrap();
                let devnum = dev.get_devnum().unwrap();
                let major_minor = format!("{}:{}", major(devnum), minor(devnum));

                create_tmp_file("/tmp/test_update_devnode/data", &id, "", true);

                dev.sealed.replace(true);
                dev.set_base_path("/tmp/test_update_devnode");
                dev.set_devgid("1").unwrap();
                dev.set_devuid("1").unwrap();
                dev.set_devmode("666").unwrap();
                dev.add_devlink("test_update_devnode/bbb").unwrap();

                let mut unit = ExecuteUnitData::new(dev.clone());
                unit.clone_device_db().unwrap();

                unit.update_devnode(&HashMap::new()).unwrap();
                /* Record the devlink in db */
                dev.update_db().unwrap();

                let p = Path::new("/dev/test_update_devnode/bbb");
                let p_block_s = format!("/dev/block/{}", major_minor);
                let prior_p_s = format!("/run/devmaster/links/test_update_devnode\\x2fbbb/{}", id);
                let prior_p = Path::new(&prior_p_s);
                assert!(p.exists());
                assert!(Path::new(&p_block_s).exists());
                assert!(p
                    .canonicalize()
                    .unwrap()
                    .ends_with(&dev.get_sysname().unwrap()));
                let _ = prior_p.symlink_metadata().unwrap(); // Test symlink exists.

                let new_dev = Rc::new(Device::from_device_id(&id).unwrap());
                new_dev.sealed.replace(true);
                new_dev.set_base_path("/tmp/test_update_devnode");
                new_dev.set_devgid("0").unwrap();
                new_dev.set_devuid("0").unwrap();
                new_dev.set_devmode("600").unwrap();

                let mut unit = ExecuteUnitData::new(new_dev);
                /* See the devlink in db, but it is absent in the current device object.
                 *
                 * Then update_devnode method will remove the devlink.
                 */
                unit.clone_device_db().unwrap();
                unit.update_devnode(&HashMap::new()).unwrap();

                assert!(!Path::new("/dev/test_update_devnode/bbb").exists());
                assert!(Path::new(&p_block_s).exists());
                let _ = prior_p.symlink_metadata().unwrap_err(); // Test symlink does not exists.

                remove_dir_all("/tmp/test_update_devnode").unwrap();

                /* Non-block devices do not have device nodes, thus update_devnode method will do nothing. */
                let lo = Rc::new(Device::from_subsystem_sysname("net", "lo").unwrap());

                let mut unit = ExecuteUnitData::new(lo);
                unit.update_devnode(&HashMap::new()).unwrap();

                /* Cover error paths when uid, gid or mode is not set. */
                let dev = Rc::new(Device::from_device_id(&id).unwrap());
                dev.sealed.replace(true);
                dev.add_devlink("test_update_devnode/xxx").unwrap();

                let mut unit = ExecuteUnitData::new(dev.clone());
                unit.clone_device_db().unwrap();
                unit.update_devnode(&HashMap::new()).unwrap();

                let p = Path::new("/dev/test_update_devnode/xxx");
                let prior_p_s = format!("/run/devmaster/links/test_update_devnode\\x2fxxx/{}", id);
                assert!(p.exists());
                assert!(Path::new(&p_block_s).exists());
                assert!(p
                    .canonicalize()
                    .unwrap()
                    .ends_with(&dev.get_sysname().unwrap()));
                let _ = Path::new(&prior_p_s).symlink_metadata().unwrap(); // Test symlink exists.

                cleanup_node(dev).unwrap();

                assert!(!p.exists());
                assert!(!Path::new(&p_block_s).exists());
                let _ = Path::new(&prior_p_s).symlink_metadata().unwrap_err(); // Test symlink exists.

                Ok(())
            },
        ) {
            assert!(e.is_errno(nix::Error::EACCES) || e.is_errno(nix::Error::EBUSY));
        }
    }

    #[test]
    fn test_subst_format() {
        if let Err(e) =
            LoopDev::inner_process("/tmp/test_subst_format_tmpfile", 1024 * 1024 * 10, |dev| {
                let dev = Rc::new(dev.shallow_clone().unwrap());
                let mut unit = ExecuteUnitData::new(dev.clone());
                let devnum = dev.get_devnum().unwrap();
                let major_minor = format!("{}:{}", major(devnum), minor(devnum));
                let sysname = dev.get_sysname().unwrap();

                assert_eq!(
                    unit.subst_format(FormatSubstitutionType::Attr, Some("dev".to_string()))
                        .unwrap(),
                    major_minor
                );

                assert_eq!(
                    unit.subst_format(
                        FormatSubstitutionType::Attr,
                        Some(format!("[block/{}]dev", sysname))
                    )
                    .unwrap(),
                    major_minor
                );

                unit.set_parent(Some(Rc::new(
                    Device::from_subsystem_sysname("net", "lo").unwrap(),
                )));

                /* Get the sysattr of parent device set in unit. */
                assert_eq!(
                    unit.subst_format(FormatSubstitutionType::Attr, Some("ifindex".to_string()))
                        .unwrap(),
                    "1".to_string()
                );

                /* Invalid sysattr will be replaced with empty string. */
                assert!(unit
                    .subst_format(
                        FormatSubstitutionType::Attr,
                        Some("asdfasdfads".to_string())
                    )
                    .unwrap()
                    .is_empty());

                dev.add_property("hello", "world").unwrap();

                assert_eq!(
                    unit.subst_format(FormatSubstitutionType::Env, Some("hello".to_string()))
                        .unwrap(),
                    "world".to_string()
                );

                assert!(unit
                    .subst_format(FormatSubstitutionType::Env, Some("asdfgasd".to_string()))
                    .unwrap()
                    .is_empty());

                assert!(unit
                    .subst_format(FormatSubstitutionType::Driver, None)
                    .unwrap()
                    .is_empty());

                assert_eq!(
                    unit.subst_format(FormatSubstitutionType::Id, None).unwrap(),
                    "lo".to_string()
                );

                let major = unit
                    .subst_format(FormatSubstitutionType::Major, None)
                    .unwrap();
                let minor = unit
                    .subst_format(FormatSubstitutionType::Minor, None)
                    .unwrap();
                assert_eq!(format!("{}:{}", major, minor), major_minor);

                unit.program_result = "hello world test".to_string();
                assert_eq!(
                    unit.subst_format(FormatSubstitutionType::Result, None)
                        .unwrap(),
                    "hello world test".to_string()
                );
                assert_eq!(
                    unit.subst_format(FormatSubstitutionType::Result, Some("0".to_string()))
                        .unwrap(),
                    "hello".to_string()
                );
                assert_eq!(
                    unit.subst_format(FormatSubstitutionType::Result, Some("1+".to_string()))
                        .unwrap(),
                    "world test".to_string()
                );
                unit.subst_format(FormatSubstitutionType::Result, Some("x".to_string()))
                    .unwrap_err();
                unit.subst_format(FormatSubstitutionType::Result, Some("x+".to_string()))
                    .unwrap_err();

                assert!(unit
                    .subst_format(FormatSubstitutionType::Result, Some("3+".to_string()))
                    .unwrap()
                    .is_empty());

                assert_eq!(
                    unit.subst_format(FormatSubstitutionType::Name, None)
                        .unwrap(),
                    sysname
                );

                unit.name = "test".to_string();
                assert_eq!(
                    unit.subst_format(FormatSubstitutionType::Name, None)
                        .unwrap(),
                    "test".to_string(),
                );

                dev.add_devlink("test").unwrap();
                dev.sealed.replace(true);
                assert_eq!(
                    unit.subst_format(FormatSubstitutionType::Links, None)
                        .unwrap(),
                    "test".to_string(),
                );

                unit.subst_format(FormatSubstitutionType::Invalid, None)
                    .unwrap_err();

                Ok(())
            })
        {
            assert!(e.is_errno(nix::Error::EACCES) || e.is_errno(nix::Error::EBUSY));
        }

        let dev = Rc::new(Device::from_subsystem_sysname("net", "lo").unwrap());
        let unit = ExecuteUnitData::new(dev);
        assert_eq!(
            unit.subst_format(FormatSubstitutionType::Name, None)
                .unwrap(),
            "lo".to_string()
        );
    }
}
