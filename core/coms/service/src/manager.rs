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

#[cfg(feature = "plugin")]
use crate::base::PLUGIN_NAME;
#[cfg(feature = "plugin")]
use constants::LOG_FILE_PATH;

use super::comm::ServiceUmComm;
use core::rel::{ReStation, Reliability};
use core::unit::{UmIf, UnitManagerObj, UnitMngUtil, UnitType};
use std::rc::Rc;
use std::sync::Arc;

struct ServiceManager {
    comm: Arc<ServiceUmComm>,
}

// the declaration "pub(self)" is for identification only.
impl ServiceManager {
    pub(self) fn new() -> ServiceManager {
        let _comm = ServiceUmComm::get_instance();
        ServiceManager {
            comm: Arc::clone(&_comm),
        }
    }
}

impl UnitManagerObj for ServiceManager {
    fn private_section(&self, _unit_type: UnitType) -> String {
        "Service".into()
    }

    fn can_transient(&self, _unit_type: UnitType) -> bool {
        true
    }
}

impl ReStation for ServiceManager {
    // no input, no compensate

    // no data

    // reload: no external connections, no entry
}

impl UnitMngUtil for ServiceManager {
    fn attach_um(&self, um: Rc<dyn UmIf>) {
        self.comm.attach_um(um);
    }

    fn attach_reli(&self, reli: Rc<Reliability>) {
        self.comm.attach_reli(reli);
    }
}

impl Default for ServiceManager {
    fn default() -> Self {
        ServiceManager::new()
    }
}

use core::declare_umobj_plugin;
declare_umobj_plugin!(ServiceManager, ServiceManager::default);
