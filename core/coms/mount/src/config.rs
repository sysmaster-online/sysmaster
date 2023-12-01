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

use std::{cell::RefCell, path::PathBuf, rc::Rc};

use unit_parser::prelude::UnitConfig;

use crate::{comm::MountUnitComm, rentry::SectionMount};

#[derive(UnitConfig, Default)]
#[allow(non_snake_case)]
pub(super) struct MountConfigData {
    pub Mount: SectionMount,
}

#[allow(unused)]
pub(super) struct MountConfig {
    // associated objects
    comm: Rc<MountUnitComm>,

    // owned objects
    data: Rc<RefCell<MountConfigData>>,
}

pub(super) struct MountParameters {
    pub what: String,
    pub options: String,
    pub fstype: String,
}

impl MountConfig {
    pub(super) fn new(commr: &Rc<MountUnitComm>) -> Self {
        MountConfig {
            comm: Rc::clone(commr),
            data: Rc::new(RefCell::new(MountConfigData::default())),
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
        MountParameters {
            what: self.mount_what(),
            options: self.mount_options(),
            fstype: self.mount_type(),
        }
    }

    pub(super) fn update_mount_parameters(&self, what: &str, options: &str, fstype: &str) {
        (*self.data.borrow_mut()).Mount.What = what.to_string();
        (*self.data.borrow_mut()).Mount.Options = options.to_string();
        (*self.data.borrow_mut()).Mount.Type = fstype.to_string();
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
