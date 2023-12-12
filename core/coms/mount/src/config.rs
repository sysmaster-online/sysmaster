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
//
#![allow(non_snake_case)]
use core::unit::KillContext;
use core::{error::Result, rel::ReStation};
use std::{cell::RefCell, path::PathBuf, rc::Rc, str::FromStr};

use nix::sys::signal::Signal;
use unit_parser::prelude::UnitConfig;

use crate::{comm::MountUnitComm, rentry::SectionMount};

#[derive(UnitConfig, Default)]
pub(super) struct MountConfigData {
    pub Mount: SectionMount,
}

impl MountConfigData {
    pub(self) fn new(Mount: SectionMount) -> MountConfigData {
        MountConfigData { Mount }
    }
}

#[allow(unused)]
pub(super) struct MountConfig {
    // associated objects
    comm: Rc<MountUnitComm>,

    // owned objects
    data: Rc<RefCell<MountConfigData>>,
    kill_context: Rc<KillContext>,
    mount_parameters: RefCell<MountParameters>,
    mount_parameters_from_mountinfo: RefCell<MountParameters>,
}

impl ReStation for MountConfig {
    // no input, no compensate

    // data
    fn db_map(&self, reload: bool) {
        if reload {
            return;
        }
        if let Some(conf) = self.comm.rentry_conf_get() {
            self.data.replace(MountConfigData::new(conf));
        }
    }

    fn db_insert(&self) {
        self.comm.rentry_conf_insert(&self.data.borrow().Mount);
    }

    // reload: no external connections, no entry
}

#[derive(Clone)]
pub(super) struct MountParameters {
    pub what: String,
    pub options: String,
    pub fstype: String,
}

impl MountParameters {
    fn empty() -> Self {
        Self {
            what: String::new(),
            options: String::new(),
            fstype: String::new(),
        }
    }
}

impl MountConfig {
    pub(super) fn new(commr: &Rc<MountUnitComm>) -> Self {
        MountConfig {
            comm: Rc::clone(commr),
            data: Rc::new(RefCell::new(MountConfigData::default())),
            kill_context: Rc::new(KillContext::default()),
            mount_parameters: RefCell::new(MountParameters::empty()),
            mount_parameters_from_mountinfo: RefCell::new(MountParameters::empty()),
        }
    }

    pub(super) fn load(&self, paths: Vec<PathBuf>, name: &str, _update: bool) {
        log::debug!("Loading {} config from: {:?}", name, paths);
        let mount_config = match MountConfigData::load_config(paths, name) {
            Ok(v) => v,
            Err(e) => {
                log::error!("Invalid Configuration: {}", e);
                return;
            }
        };
        *self.data.borrow_mut() = mount_config;
        if let Err(e) = self.parse_kill_context() {
            log::error!("Failed to parse KillContext for {}: {}", name, e);
        }
        self.set_mount_parameters(MountParameters {
            what: self.mount_what(),
            options: self.mount_options(),
            fstype: self.mount_type(),
        })

        // Todo: reli
        // if update {
        //     self.db_update();
        // }
    }

    pub(super) fn config_data(&self) -> Rc<RefCell<MountConfigData>> {
        self.data.clone()
    }

    pub(super) fn mount_where(&self) -> String {
        self.data.borrow().Mount.Where.clone()
    }

    pub(super) fn set_mount_where(&self, mount_where: &str) {
        (*self.data.borrow_mut()).Mount.Group = mount_where.to_string();
    }

    pub(super) fn mount_what(&self) -> String {
        self.data.borrow().Mount.What.clone()
    }

    pub(super) fn mount_type(&self) -> String {
        self.data.borrow().Mount.Type.clone()
    }

    pub(super) fn mount_options(&self) -> String {
        self.data.borrow().Mount.Options.clone()
    }

    pub(super) fn directory_mode(&self) -> u32 {
        self.data.borrow().Mount.DirectoryMode
    }

    #[allow(unused)]
    pub(super) fn force_unmount(&self) -> bool {
        self.data.borrow().Mount.ForceUnmount
    }

    pub(super) fn mount_parameters(&self) -> MountParameters {
        (*self.mount_parameters.borrow()).clone()
    }

    pub(super) fn set_mount_parameters(&self, mount_parameters: MountParameters) {
        *self.mount_parameters.borrow_mut() = mount_parameters
    }

    pub(super) fn mount_parameters_from_mountinfo(&self) -> MountParameters {
        (*self.mount_parameters_from_mountinfo.borrow()).clone()
    }

    pub(super) fn set_mount_parameters_from_mountinfo(&self, mount_parameters: MountParameters) {
        *self.mount_parameters_from_mountinfo.borrow_mut() = mount_parameters
    }

    /// update the mount parameters. return true if parameters are updated, return false if
    /// parameters are not changed
    pub(super) fn updated_mount_parameters_from_mountinfo(
        &self,
        what: &str,
        options: &str,
        fstype: &str,
    ) -> bool {
        let mut parameter_changed = false;
        let mut mount_parameters = self.mount_parameters_from_mountinfo();
        if !mount_parameters.what.eq(what) {
            mount_parameters.what = what.to_string();
            parameter_changed = true;
        }
        if !mount_parameters.options.eq(options) {
            mount_parameters.options = options.to_string();
            parameter_changed = true;
        }
        if !mount_parameters.fstype.eq(fstype) {
            mount_parameters.fstype = fstype.to_string();
            parameter_changed = true;
        }
        self.set_mount_parameters_from_mountinfo(mount_parameters);
        parameter_changed
    }

    pub(super) fn kill_context(&self) -> Rc<KillContext> {
        self.kill_context.clone()
    }

    pub(super) fn parse_kill_context(&self) -> Result<()> {
        self.kill_context
            .set_kill_mode(self.config_data().borrow().Mount.KillMode);

        let signal = Signal::from_str(&self.config_data().borrow().Mount.KillSignal)?;
        self.kill_context.set_kill_signal(signal);
        Ok(())
    }
}

pub(super) fn mount_is_bind(mount_parameters: &MountParameters) -> bool {
    // This is a simplified version.
    for v in mount_parameters.options.split(',') {
        let option = v.trim();
        if option == "bind" || option == "rbind" {
            return true;
        }
    }
    if mount_parameters.options.contains("bind|rbind") {
        return true;
    }
    if mount_parameters.fstype == "bind" || mount_parameters.fstype == "rbind" {
        return true;
    }
    false
}
