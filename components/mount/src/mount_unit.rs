//! mount unit is entry of mount type of unit，need impl
//! UnitObj,UnitMngUtil, UnitSubClass trait

use nix::{sys::signal::Signal, unistd::Pid};
use std::path::PathBuf;
use std::rc::Rc;

use super::mount_comm::MountComm;
use super::mount_mng::MountMng;
use process1::manager::{UnitActiveState, UnitMngUtil, UnitObj, UnitSubClass};
use utils::logger;

struct MountUnit {
    comm: Rc<MountComm>,
    mng: Rc<MountMng>,
}

impl MountUnit {
    fn new() -> MountUnit {
        let _comm = Rc::new(MountComm::new());
        MountUnit {
            comm: Rc::clone(&_comm),
            mng: Rc::new(MountMng::new(&_comm)),
        }
    }
}

impl UnitObj for MountUnit {
    fn load(&self, _paths: Vec<PathBuf>) -> utils::Result<(), Box<dyn std::error::Error>> {
        if self.comm.unit().is_some() {
            self.comm.unit().unwrap().set_ignore_on_isolate(true);
        }

        Ok(())
    }

    fn current_active_state(&self) -> UnitActiveState {
        self.mng.mount_state_to_unit_state()
    }

    fn attach_unit(&self, unit: Rc<process1::manager::Unit>) {
        self.comm.attach_unit(unit);
    }

    fn init(&self) {}

    fn done(&self) {}

    fn coldplug(&self) {}

    fn dump(&self) {}

    fn start(&self) -> utils::Result<(), process1::manager::UnitActionError> {
        self.mng.enter_mounted();
        Ok(())
    }

    fn stop(&self) -> utils::Result<(), process1::manager::UnitActionError> {
        self.mng.enter_dead();
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
    fn attach(&self, _um: Rc<process1::manager::UnitManager>) {}
}

impl Default for MountUnit {
    fn default() -> Self {
        MountUnit::new()
    }
}

const LOG_LEVEL: u32 = 4;
const PLUGIN_NAME: &str = "MountUnit";
use process1::declure_unitobj_plugin;
declure_unitobj_plugin!(MountUnit, MountUnit::default, PLUGIN_NAME, LOG_LEVEL);

#[cfg(test)]
mod tests {
    #[test]
    fn test() {}
}
