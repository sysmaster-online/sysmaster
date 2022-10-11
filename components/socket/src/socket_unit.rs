//! SocketUnit is the entrance of the sub unit，implement the trait UnitObj,UnitMngUtil and UnitSubClass.
//! Trait UnitObj defines the behavior of the sub unit.
//! Trait UnitMngUtil is used to attach the Unitmanager to the sub unit.
//! Trait UnitSubClass implement the convert from sub unit to UnitObj.

use nix::{sys::signal::Signal, unistd::Pid};
use process1::manager::{
    ExecContext, Unit, UnitActionError, UnitActiveState, UnitManager, UnitMngUtil, UnitObj,
    UnitSubClass,
};
use std::{error::Error, path::PathBuf, rc::Rc};

use crate::{
    socket_comm::SocketComm, socket_config::SocketConfig, socket_load::SocketLoad,
    socket_mng::SocketMng, socket_port::SocketPorts,
};
use utils::logger;

// the structuer of the socket unit type
struct SocketUnit {
    comm: Rc<SocketComm>,
    config: Rc<SocketConfig>,
    mng: Rc<SocketMng>,
    ports: Rc<SocketPorts>,
    load: SocketLoad,
}

impl UnitObj for SocketUnit {
    fn load(&self, paths: Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
        log::debug!("socket begin to load conf file");
        self.config.load(paths)?;

        self.load.parse(self.config.config_data(), &self.mng)?;

        self.load.socket_add_extras(&self.mng);

        self.load.socket_verify()
    }

    // the function entrance to start the unit
    fn start(&self) -> Result<(), UnitActionError> {
        self.ports.attach(self.mng.clone());

        let starting = self.mng.start_check()?;
        if starting {
            log::debug!("socket already in start");
            return Ok(());
        }

        self.mng.start_action();

        Ok(())
    }

    fn stop(&self) -> Result<(), UnitActionError> {
        let stopping = self.mng.stop_check()?;
        if stopping {
            log::debug!("socket already in stop, return immediretly");
            return Ok(());
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
        self.ports.collect_fds()
    }

    fn attach_unit(&self, unit: Rc<Unit>) {
        self.comm.attach_unit(unit);
    }
}

// attach the UnitManager for weak reference
impl UnitMngUtil for SocketUnit {
    fn attach(&self, um: Rc<UnitManager>) {
        self.comm.attach_um(um);
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
        let _comm = Rc::new(SocketComm::new());
        let _config = Rc::new(SocketConfig::new());
        let ports = Rc::new(SocketPorts::new());
        let mng = Rc::new(SocketMng::new(&_comm, &_config, &ports, &context));
        SocketUnit {
            comm: Rc::clone(&_comm),
            config: Rc::clone(&_config),
            mng,
            ports: ports.clone(),
            load: SocketLoad::new(&_config, &_comm, &ports),
        }
    }
}

impl Default for SocketUnit {
    fn default() -> Self {
        SocketUnit::new()
    }
}

const LOG_LEVEL: u32 = 4;
const PLUGIN_NAME: &str = "SocketUnit";

use process1::declure_unitobj_plugin;

// define the method to create the instance of the unit
declure_unitobj_plugin!(SocketUnit, SocketUnit::default, PLUGIN_NAME, LOG_LEVEL);
