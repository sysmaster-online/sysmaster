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

//! mount unit is entry of mount type of unit，need impl
//! UnitObj,UnitMngUtil, UnitSubClass trait

use super::comm::MountUnitComm;
use super::mng::MountMng;
use nix::sys::wait::WaitStatus;
use std::path::PathBuf;
use std::rc::Rc;
use sysmaster::error::*;
use sysmaster::rel::{ReStation, Reliability};
use sysmaster::unit::{SubUnit, UmIf, UnitActiveState, UnitBase, UnitMngUtil};

struct MountUnit {
    comm: Rc<MountUnitComm>,
    mng: Rc<MountMng>,
}

impl ReStation for MountUnit {
    // no input, no compensate

    // data
    fn db_map(&self, reload: bool) {
        self.mng.db_map(reload);
    }

    fn db_insert(&self) {
        self.mng.db_insert();
    }

    // reload: no external connections, entry-only
    fn entry_coldplug(&self) {
        // rebuild external connections, like: timer, ...
        // do nothing now
    }

    fn entry_clear(&self) {
        // release external connection, like: timer, ...
        // do nothing now
    }
}

impl MountUnit {
    fn new(_um: Rc<dyn UmIf>) -> MountUnit {
        let _comm = Rc::new(MountUnitComm::new());
        MountUnit {
            comm: Rc::clone(&_comm),
            mng: Rc::new(MountMng::new(&_comm)),
        }
    }
}

impl SubUnit for MountUnit {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn load(&self, _paths: Vec<PathBuf>) -> Result<()> {
        if let Some(u) = self.comm.owner() {
            u.set_ignore_on_isolate(true)
        }
        Ok(())
    }

    fn current_active_state(&self) -> UnitActiveState {
        self.mng.mount_state_to_unit_state()
    }

    fn get_subunit_state(&self) -> String {
        self.mng.get_state()
    }

    fn attach_unit(&self, unit: Rc<dyn UnitBase>) {
        self.comm.attach_unit(unit);
        self.db_insert();
    }

    fn init(&self) {}

    fn done(&self) {}

    fn dump(&self) {}

    fn start(&self) -> Result<()> {
        let started = self.mng.start_check()?;
        if started {
            log::debug!("mount already in starting, just return immediately");
            return Ok(());
        }

        self.mng.enter_mounted(true);

        Ok(())
    }

    fn stop(&self, _force: bool) -> Result<()> {
        self.mng.enter_dead(true);
        Ok(())
    }

    fn kill(&self) {}

    fn release_resources(&self) {}

    fn sigchld_events(&self, _wait_status: WaitStatus) {}

    fn reset_failed(&self) {}
}

impl UnitMngUtil for MountUnit {
    fn attach_um(&self, um: Rc<dyn UmIf>) {
        self.comm.attach_um(um);
    }

    fn attach_reli(&self, reli: Rc<Reliability>) {
        self.comm.attach_reli(reli);
    }
}

/*impl Default for MountUnit {
    fn default() -> Self {
        MountUnit::new()
    }
}*/

use sysmaster::declure_unitobj_plugin_with_param;
declure_unitobj_plugin_with_param!(MountUnit, MountUnit::new);
