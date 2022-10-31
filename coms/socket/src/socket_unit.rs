//! SocketUnit is the entrance of the sub unit，implement the trait UnitObj,UnitMngUtil and UnitSubClass.
//! Trait UnitObj defines the behavior of the sub unit.
//! Trait UnitMngUtil is used to attach the Unitmanager to the sub unit.
//! Trait UnitSubClass implement the convert from sub unit to UnitObj.

use crate::{
    socket_base::{LOG_LEVEL, PLUGIN_NAME},
    socket_comm::SocketUnitComm,
    socket_config::SocketConfig,
    socket_load::SocketLoad,
    socket_mng::SocketMng,
};
use libsysmaster::manager::{
    ExecContext, Unit, UnitActionError, UnitActiveState, UnitManager, UnitMngUtil, UnitObj,
    UnitSubClass,
};
use libsysmaster::{ReStation, Reliability};
use libutils::logger;
use nix::{sys::signal::Signal, unistd::Pid};
use std::{error::Error, path::PathBuf, rc::Rc};

// the structuer of the socket unit type
struct SocketUnit {
    comm: Rc<SocketUnitComm>,
    config: Rc<SocketConfig>,
    mng: SocketMng,
    load: SocketLoad,
}

impl ReStation for SocketUnit {
    // input: do nothing

    // compensate: do nothing

    // data
    fn db_map(&self) {
        self.config.db_map();
        self.mng.db_map();
    }

    fn db_insert(&self) {
        self.config.db_insert();
        self.mng.db_insert();
    }

    // reload: entry-only
    fn entry_coldplug(&self) {
        // rebuild external connections, like: timer, ...
        self.mng.entry_coldplug();
    }

    fn entry_clear(&self) {
        // release external connection, like: timer, ...
        self.mng.entry_clear();
    }
}

impl UnitObj for SocketUnit {
    fn load(&self, paths: Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
        log::debug!("socket begin to load conf file");
        self.config.load(paths, true)?;

        let ret = self.load.socket_add_extras();
        if ret.is_err() {
            self.config.reset();
            return ret;
        }

        self.load.socket_verify()
    }

    // the function entrance to start the unit
    fn start(&self) -> Result<(), UnitActionError> {
        let starting = self.mng.start_check()?;
        if starting {
            log::debug!("socket already in start");
            return Ok(());
        }

        self.mng.start_action();

        Ok(())
    }

    fn stop(&self, force: bool) -> Result<(), UnitActionError> {
        if !force {
            let stopping = self.mng.stop_check()?;
            if stopping {
                log::debug!("socket already in stop, return immediretly");
                return Ok(());
            }
        }

        self.mng.stop_action();

        Ok(())
    }

    fn reload(&self) {}

    fn sigchld_events(&self, pid: Pid, code: i32, status: Signal) {
        self.mng.sigchld_event(pid, code, status)
    }

    fn current_active_state(&self) -> UnitActiveState {
        self.mng.current_active_state()
    }

    fn collect_fds(&self) -> Vec<i32> {
        self.mng.collect_fds()
    }

    fn attach_unit(&self, unit: Rc<Unit>) {
        self.comm.attach_unit(unit);
        self.db_insert();
    }
}

// attach the UnitManager for weak reference
impl UnitMngUtil for SocketUnit {
    fn attach_um(&self, um: Rc<UnitManager>) {
        self.comm.attach_um(um);
    }

    fn attach_reli(&self, reli: Rc<Reliability>) {
        self.comm.attach_reli(reli);
    }
}

impl UnitSubClass for SocketUnit {
    fn into_unitobj(self: Box<Self>) -> Box<dyn UnitObj> {
        Box::new(*self)
    }
}

impl SocketUnit {
    fn new() -> SocketUnit {
        let context = Rc::new(ExecContext::new());
        let _comm = Rc::new(SocketUnitComm::new());
        let _config = Rc::new(SocketConfig::new(&_comm));
        SocketUnit {
            comm: Rc::clone(&_comm),
            config: Rc::clone(&_config),
            mng: SocketMng::new(&_comm, &_config, &context),
            load: SocketLoad::new(&_config, &_comm),
        }
    }
}

impl Default for SocketUnit {
    fn default() -> Self {
        SocketUnit::new()
    }
}

// define the method to create the instance of the unit
use libsysmaster::declure_unitobj_plugin;
declure_unitobj_plugin!(SocketUnit, SocketUnit::default, PLUGIN_NAME, LOG_LEVEL);
