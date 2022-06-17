use nix::{sys::signal::Signal, unistd::Pid};
use process1::manager::{
    Unit, UnitActionError, UnitActiveState, UnitManager, UnitMngUtil, UnitObj, UnitSubClass,
};
use std::{error::Error, rc::Rc};

use crate::{
    socket_comm::SocketComm,
    socket_config::{SocketConf, SocketConfig},
    socket_load::SocketLoad,
    socket_mng::SocketMng,
};
use utils::{config_parser::ConfigParse, logger};

#[allow(dead_code)]
struct SocketUnit {
    comm: Rc<SocketComm>,
    config: Rc<SocketConfig>,
    mng: SocketMng,
    load: SocketLoad,
}

impl UnitObj for SocketUnit {
    fn init(&self) {
        todo!()
    }

    fn done(&self) {
        todo!()
    }

    fn load(&self, conf_str: &str) -> Result<(), Box<dyn Error>> {
        let socket_parser = SocketConf::builder_parser();
        let socket_conf = socket_parser.conf_file_parse(conf_str);

        let ret = socket_conf.map(|conf| self.load.parse(conf));

        if let Err(e) = ret {
            return Err(Box::new(e));
        }

        self.load.socket_add_extras();

        return self.load.socket_verify();
    }

    fn coldplug(&self) {
        todo!()
    }

    fn start(&self) -> Result<(), UnitActionError> {
        log::debug!("begin to start the service unit");
        self.mng.start_check()?;

        self.mng.start_action();

        Ok(())
    }

    fn dump(&self) {
        todo!()
    }

    fn stop(&self) -> Result<(), UnitActionError> {
        Ok(())
    }

    fn reload(&self) {}

    fn kill(&self) {
        todo!()
    }

    fn release_resources(&self) {
        todo!()
    }

    fn sigchld_events(&self, _pid: Pid, _code: i32, _status: Signal) {}

    fn reset_failed(&self) {
        todo!()
    }

    fn current_active_state(&self) -> UnitActiveState {
        self.mng.current_active_state()
    }

    fn attach_unit(&self, unit: Rc<Unit>) {
        self.comm.attach_unit(unit);
    }
}

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
        SocketUnit {
            comm: Rc::clone(&_comm),
            config: Rc::clone(&_config),
            mng: SocketMng::new(&_comm, &_config),
            load: SocketLoad::new(&_config, &_comm),
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

declure_unitobj_plugin!(SocketUnit, SocketUnit::default, PLUGIN_NAME, LOG_LEVEL);
