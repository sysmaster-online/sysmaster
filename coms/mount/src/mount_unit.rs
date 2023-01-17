//! mount unit is entry of mount type of unit，need impl
//! UnitObj,UnitMngUtil, UnitSubClass trait

use super::mount_base::PLUGIN_NAME;
use super::mount_comm::MountUnitComm;
use super::mount_mng::MountMng;
use libutils::logger;
use nix::{sys::signal::Signal, unistd::Pid};
use std::path::PathBuf;
use std::rc::Rc;
use sysmaster::reliability::{ReStation, Reliability};
use sysmaster::unit::{SubUnit, UmIf, UnitActionError, UnitActiveState, UnitBase, UnitMngUtil};

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
    fn new(_um: Rc<dyn UmIf>) -> MountUnit {
        let _comm = Rc::new(MountUnitComm::new());
        MountUnit {
            comm: Rc::clone(&_comm),
            mng: Rc::new(MountMng::new(&_comm)),
        }
    }
}

impl SubUnit for MountUnit {
    fn load(&self, _paths: Vec<PathBuf>) -> libutils::Result<(), Box<dyn std::error::Error>> {
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

    fn start(&self) -> libutils::Result<(), UnitActionError> {
        let started = self.mng.start_check()?;
        if started {
            log::debug!("mount already in starting, just return immediately");
            return Ok(());
        }

        self.mng.enter_mounted(true);

        Ok(())
    }

    fn stop(&self, _force: bool) -> libutils::Result<(), UnitActionError> {
        self.mng.enter_dead(true);
        Ok(())
    }

    fn reload(&self) {}

    fn kill(&self) {}

    fn release_resources(&self) {}

    fn sigchld_events(&self, _pid: Pid, _code: i32, _status: Signal) {}

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
declure_unitobj_plugin_with_param!(MountUnit, MountUnit::new, PLUGIN_NAME);
