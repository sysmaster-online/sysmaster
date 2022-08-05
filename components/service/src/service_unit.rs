use super::service_comm::ServiceComm;
use super::service_config::ServiceConfig;
use super::service_mng::ServiceMng;
use super::service_monitor::ServiceMonitor;
use log;
use nix::sys::signal::Signal;
use nix::unistd::Pid;
use process1::manager::{
    Unit, UnitActionError, UnitActiveState, UnitManager, UnitMngUtil, UnitObj, UnitSubClass,
};
use std::error::Error;
use std::path::PathBuf;
use std::rc::Rc;
use utils::logger;

struct ServiceUnit {
    comm: Rc<ServiceComm>,
    config: Rc<ServiceConfig>,
    mng: ServiceMng,
    monitor: ServiceMonitor,
}

impl UnitObj for ServiceUnit {
    fn init(&self) {
        todo!()
    }

    fn done(&self) {
        todo!()
    }

    fn load(&self, paths: &Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
        self.config.load(paths);
        // // self.load.service_add_extras();
        // self.config.parse_commands(&mut self.mng);

        return self.service_verify();
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
        let comm = Rc::new(ServiceComm::new());
        let config = Rc::new(ServiceConfig::default());
        ServiceUnit {
            comm: Rc::clone(&comm),
            config: Rc::clone(&config),
            mng: ServiceMng::new(&comm, &config),
            monitor: ServiceMonitor::new(&config),
        }
    }

    pub fn service_verify(&self) -> Result<(), Box<dyn Error>> {
        Ok(())
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
