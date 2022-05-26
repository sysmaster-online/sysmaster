use super::service_comm::ServiceComm;
use super::service_config::{ServiceConf, ServiceConfig};
use super::service_load::ServiceLoad;
use super::service_mng::ServiceMng;
use super::service_monitor::ServiceMonitor;
use log;
use nix::sys::signal::Signal;
use nix::unistd::Pid;
use process1::manager::{
    Unit, UnitActionError, UnitActiveState, UnitManager, UnitMngUtil, UnitObj, UnitSubClass,
};
use std::error::Error;
use std::rc::Rc;
use utils::config_parser::ConfigParse;
use utils::logger;

struct ServiceUnit {
    comm: Rc<ServiceComm>,
    config: Rc<ServiceConfig>,
    mng: ServiceMng,
    load: ServiceLoad,
    monitor: ServiceMonitor,
}

impl UnitObj for ServiceUnit {
    fn init(&self) {
        todo!()
    }

    fn done(&self) {
        todo!()
    }

    fn load(&self, conf_str: &str) -> Result<(), Box<dyn Error>> {
        let service_parser = ServiceConf::builder_parser();
        let service_conf = service_parser.conf_file_parse(conf_str);
        let ret = service_conf.map(|_conf| self.load.parse(_conf));
        if let Err(_e) = ret {
            return Err(Box::new(_e));
        }
        self.load.service_add_extras();

        return self.load.service_verify();
    }

    fn coldplug(&self) {
        todo!()
    }

    fn start(&self) -> Result<(), UnitActionError> {
        log::debug!("begin to start the service unit");
        self.mng.start_check()?;

        self.monitor.start_action();
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

    fn reload(&self) {
        self.mng.reload_action();
    }

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

    fn get_private_conf_section_name(&self) -> Option<&str> {
        Some("Service")
    }

    fn current_active_state(&self) -> UnitActiveState {
        self.mng.current_active_state()
    }

    fn attach_unit(&self, unit: Rc<Unit>) {
        self.comm.attach_unit(unit);
    }
}

impl UnitMngUtil for ServiceUnit {
    fn attach(&self, um: Rc<UnitManager>) {
        self.comm.attach_um(um);
    }
}

impl UnitSubClass for ServiceUnit {
    fn into_unitobj(self: Box<Self>) -> Box<dyn UnitObj> {
        Box::new(*self)
    }
}

impl ServiceUnit {
    fn new() -> ServiceUnit {
        let _comm = Rc::new(ServiceComm::new());
        let _config = Rc::new(ServiceConfig::new());
        ServiceUnit {
            comm: Rc::clone(&_comm),
            config: Rc::clone(&_config),
            mng: ServiceMng::new(&_comm, &_config),
            load: ServiceLoad::new(&_config),
            monitor: ServiceMonitor::new(&_config),
        }
    }
}

impl Default for ServiceUnit {
    fn default() -> Self {
        ServiceUnit::new()
    }
}

const LOG_LEVEL: u32 = 4;
const PLUGIN_NAME: &str = "ServiceUnit";
use process1::declure_unitobj_plugin;
declure_unitobj_plugin!(ServiceUnit, ServiceUnit::default, PLUGIN_NAME, LOG_LEVEL);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service_base::ServiceCommand;
    use process1::manager::UnitObj;
    use std::{fs::File, io::Read};

    #[test]
    fn test_service_parse() {
        let file_path = "../../libutils/examples/config.service";
        let mut file = File::open(file_path).unwrap();
        let mut buf = String::new();
        match file.read_to_string(&mut buf) {
            Ok(s) => s,
            Err(_e) => {
                return;
            }
        };

        let service = ServiceUnit::new();
        let _result = service.load(buf.as_str());
        assert_ne!(service.config.get_exec_cmds(ServiceCommand::Start).len(), 0);

        for command in &service.config.get_exec_cmds(ServiceCommand::Start) {
            println!("cmd: {}, args: {:?}", command.path(), command.argv());
        }
    }
}
