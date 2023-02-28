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

use super::base::PLUGIN_NAME;
use super::comm::ServiceUmComm;
use basic::logger;
use std::rc::Rc;
use std::sync::Arc;
use sysmaster::rel::{ReStation, Reliability};
use sysmaster::unit::{UmIf, UnitManagerObj, UnitMngUtil};

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
    // nothing to customize
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

use sysmaster::declure_umobj_plugin;
declure_umobj_plugin!(ServiceManager, ServiceManager::default, PLUGIN_NAME);
