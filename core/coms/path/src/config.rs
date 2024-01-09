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

//! path_config mod load the conf file list and convert it to structure which is defined in this mod.
//!
#![allow(non_snake_case)]
use crate::{comm::PathUnitComm, rentry::SectionPath};
use core::error::*;
use core::rel::ReStation;
use std::{cell::RefCell, path::PathBuf, rc::Rc};
use unit_parser::prelude::UnitConfig;

#[derive(UnitConfig, Default)]
#[allow(non_snake_case)]
pub(super) struct PathConfigData {
    pub Path: SectionPath,
}

impl PathConfigData {
    pub(self) fn new(Path: SectionPath) -> PathConfigData {
        PathConfigData { Path }
    }

    pub(self) fn set_property(&mut self, key: &str, value: &str) -> Result<(), core::error::Error> {
        self.Path.set_property(key, value)
    }
}

pub(super) struct PathConfig {
    // associated objects
    comm: Rc<PathUnitComm>,

    // owned objects
    data: Rc<RefCell<PathConfigData>>,
}

impl ReStation for PathConfig {
    // no input, no compensate

    // data
    fn db_map(&self, reload: bool) {
        if reload {
            return;
        }

        if let Some(data) = self.comm.rentry_conf_get() {
            // PathConfigData
            self.data.replace(PathConfigData::new(data));
        }
    }

    fn db_insert(&self) {
        self.comm.rentry_conf_insert(&self.data.borrow().Path);
    }

    // reload: no external connections, no entry
}

impl PathConfig {
    pub(super) fn new(commr: &Rc<PathUnitComm>) -> Self {
        PathConfig {
            comm: Rc::clone(commr),
            data: Rc::new(RefCell::new(PathConfigData::default())),
        }
    }

    pub(super) fn load(&self, paths: Vec<PathBuf>, name: &str, update: bool) -> Result<()> {
        let data = match PathConfigData::load_config(paths, name) {
            Ok(v) => v,
            Err(e) => {
                log::error!("Invalid Configuration: {}", e);
                return Err(Error::ConfigureError {
                    msg: format!("Invalid Configuration: {}", e),
                });
            }
        };

        // record original configuration
        *self.data.borrow_mut() = data;

        if update {
            self.db_update();
        }

        Ok(())
    }

    pub(super) fn config_data(&self) -> Rc<RefCell<PathConfigData>> {
        self.data.clone()
    }

    pub(super) fn set_property(&self, key: &str, value: &str) -> Result<()> {
        let ret = self.data.borrow_mut().set_property(key, value);
        self.db_update();
        ret
    }
}
