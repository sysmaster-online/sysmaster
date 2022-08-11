//! SocketUnit类型socket类型的总入口，需要实现UnitObj,UnitMngUtil,以及UnitSubClass三个trait,
//! UnitObj是Unit的抽象，定义对process1提供的具体行为，
//! UnitMngUtil是为了关联subUnit和Manger，由于rust不支持继承和多态，因此需要采用这种方式来间接支持
//！ UnitSubClass为了实现SubUnit到UnitObj的转换，简介达成多态的目的

use nix::{sys::signal::Signal, unistd::Pid};
use process1::manager::{
    Unit, UnitActionError, UnitActiveState, UnitManager, UnitMngUtil, UnitObj, UnitSubClass,
};
use std::{error::Error, path::PathBuf, rc::Rc};

use crate::{
    socket_comm::SocketComm, socket_config::SocketConfig, socket_load::SocketLoad,
    socket_mng::SocketMng, socket_port::SocketPorts,
};
use utils::logger;

#[allow(dead_code)]
// the structuer of the socket unit type
struct SocketUnit {
    comm: Rc<SocketComm>,
    config: Rc<SocketConfig>,
    mng: Rc<SocketMng>,
    ports: Rc<SocketPorts>,
    load: SocketLoad,
}

impl UnitObj for SocketUnit {
    fn init(&self) {
        todo!()
    }

    fn done(&self) {
        todo!()
    }

    fn load(&self, paths: &Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
        log::debug!("socket beigin to load conf file");
        self.config.load(&paths)?;

        self.load.parse(self.config.config_data(), &self.mng)?;

        self.load.socket_add_extras(&self.mng);

        return self.load.socket_verify();
    }

    fn coldplug(&self) {
        todo!()
    }

    // the function entrance to start the unit
    fn start(&self) -> Result<(), UnitActionError> {
        self.ports.attach(self.mng.clone());
        self.mng.start_check()?;
        self.mng.start_action();

        Ok(())
    }

    fn dump(&self) {
        todo!()
    }

    fn stop(&self) -> Result<(), UnitActionError> {
        self.mng.stop_check()?;
        self.mng.stop_action();

        Ok(())
    }

    fn reload(&self) {}

    fn kill(&self) {
        todo!()
    }

    fn release_resources(&self) {
        todo!()
    }

    fn sigchld_events(&self, pid: Pid, code: i32, status: Signal) {
        self.mng.sigchld_event(pid, code, status)
    }

    fn reset_failed(&self) {
        todo!()
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
        let _comm = Rc::new(SocketComm::new());
        let _config = Rc::new(SocketConfig::new());
        let ports = Rc::new(SocketPorts::new());
        let mng = Rc::new(SocketMng::new(&_comm, &_config, &ports));
        SocketUnit {
            comm: Rc::clone(&_comm),
            config: Rc::clone(&_config),
            mng: mng.clone(),
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
