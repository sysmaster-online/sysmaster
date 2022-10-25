//! mount unit is entry of mount type of unit，need impl
//! UnitObj,UnitMngUtil, UnitSubClass trait

use super::mount_base::{LOG_LEVEL, PLUGIN_NAME};
use super::mount_comm::MountUnitComm;
use super::mount_mng::MountMng;
use nix::{sys::signal::Signal, unistd::Pid};
use process1::manager::{UnitActiveState, UnitManager, UnitMngUtil, UnitObj, UnitSubClass};
use process1::{ReStation, Reliability};
use std::path::PathBuf;
use std::rc::Rc;
use utils::logger;

struct MountUnit {
    comm: Rc<MountUnitComm>,
    mng: Rc<MountMng>,
}

impl ReStation for MountUnit {
    // no input, no compensate

    // data
    fn db_map(&self) {
        self.mng.db_map();
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
    fn new() -> MountUnit {
        let _comm = Rc::new(MountUnitComm::new());
        MountUnit {
            comm: Rc::clone(&_comm),
            mng: Rc::new(MountMng::new(&_comm)),
        }
    }
}

impl UnitObj for MountUnit {
    fn load(&self, _paths: Vec<PathBuf>) -> utils::Result<(), Box<dyn std::error::Error>> {
        self.comm.unit().set_ignore_on_isolate(true);

        Ok(())
    }

    fn current_active_state(&self) -> UnitActiveState {
        self.mng.mount_state_to_unit_state()
    }

    fn attach_unit(&self, unit: Rc<process1::manager::Unit>) {
        self.comm.attach_unit(unit);
        self.db_insert();
    }

    fn init(&self) {}

    fn done(&self) {}

    fn dump(&self) {}

    fn start(&self) -> utils::Result<(), process1::manager::UnitActionError> {
        self.mng.enter_mounted(true);
        Ok(())
    }

    fn stop(&self, _force: bool) -> utils::Result<(), process1::manager::UnitActionError> {
        self.mng.enter_dead(true);
        Ok(())
    }

    fn reload(&self) {}

    fn kill(&self) {}

    fn release_resources(&self) {}

    fn sigchld_events(&self, _pid: Pid, _code: i32, _status: Signal) {}

    fn reset_failed(&self) {}
}

impl UnitSubClass for MountUnit {
    fn into_unitobj(self: Box<Self>) -> Box<dyn UnitObj> {
        Box::new(*self)
    }
}

impl UnitMngUtil for MountUnit {
    fn attach_um(&self, um: Rc<UnitManager>) {
        self.comm.attach_um(um);
    }

    fn attach_reli(&self, reli: Rc<Reliability>) {
        self.comm.attach_reli(reli);
    }
}

impl Default for MountUnit {
    fn default() -> Self {
        MountUnit::new()
    }
}

use process1::declure_unitobj_plugin;
declure_unitobj_plugin!(MountUnit, MountUnit::default, PLUGIN_NAME, LOG_LEVEL);
