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

use super::comm::SocketUmComm;
use super::rentry::SocketReFrame;
use core::rel::{ReStation, Reliability};
use core::unit::{UmIf, UnitManagerObj, UnitMngUtil, UnitType};
use std::rc::Rc;
use std::sync::Arc;

struct SocketManager {
    comm: Arc<SocketUmComm>,
}

// the declaration "pub(self)" is for identification only.
impl SocketManager {
    pub(self) fn new() -> SocketManager {
        let _comm = SocketUmComm::get_instance();
        SocketManager {
            comm: Arc::clone(&_comm),
        }
    }
}

impl UnitManagerObj for SocketManager {
    fn private_section(&self, _unit_type: UnitType) -> String {
        "Socket".into()
    }

    fn can_transient(&self, _unit_type: UnitType) -> bool {
        true
    }
}

impl ReStation for SocketManager {
    // input: do nothing

    // compensate
    fn db_compensate_last(&self, _lframe: (u32, Option<u32>, Option<u32>), lunit: Option<&String>) {
        assert!(lunit.is_some());

        let frame = self.comm.rentry().last_frame();
        if frame.is_none() {
            // debug
            return;
        }

        let unit_id = lunit.unwrap();
        match frame.unwrap() {
            SocketReFrame::FdListen(spread) => self.rc_last_fdlisten(unit_id, spread),
        }
    }

    fn do_compensate_last(&self, _lframe: (u32, Option<u32>, Option<u32>), lunit: Option<&String>) {
        assert!(lunit.is_some());

        let frame = self.comm.rentry().last_frame();
        if frame.is_none() {
            // debug
            return;
        }

        let unit_id = lunit.unwrap();
        match frame.unwrap() {
            SocketReFrame::FdListen(spread) => self.dc_last_fdlisten(unit_id, spread),
        }
    }

    // no data

    // reload: no external connections, no entry
}

impl SocketManager {
    fn rc_last_fdlisten(&self, lunit: &str, spread: bool) {
        match spread {
            true => self.comm.um().rentry_trigger_merge(lunit, true), // merge to trigger
            false => {}                                               // do nothing, try again
        }
    }

    fn dc_last_fdlisten(&self, lunit: &str, spread: bool) {
        match spread {
            true => self.comm.um().trigger_unit(lunit), // re-run
            false => {}                                 // do nothing, try again
        }
    }
}

impl UnitMngUtil for SocketManager {
    fn attach_um(&self, um: Rc<dyn UmIf>) {
        self.comm.attach_um(um);
    }

    fn attach_reli(&self, reli: Rc<Reliability>) {
        self.comm.attach_reli(reli);
    }
}

impl Default for SocketManager {
    fn default() -> Self {
        SocketManager::new()
    }
}

use core::declare_umobj_plugin;
declare_umobj_plugin!(SocketManager, SocketManager::default);
