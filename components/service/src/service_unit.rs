use crate::service_base::{NotifyAccess, ServiceCommand, ServiceType};
use crate::service_mng::RunningData;

use super::service_comm::ServiceComm;
use super::service_config::ServiceConfig;
use super::service_mng::ServiceMng;
use super::service_monitor::ServiceMonitor;
use log;
use nix::sys::signal::Signal;
use nix::sys::socket::UnixCredentials;
use nix::unistd::Pid;
use process1::manager::{
    ExecContext, Unit, UnitActionError, UnitActiveState, UnitManager, UnitMngUtil, UnitObj,
    UnitRelations, UnitSubClass,
};

use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use std::rc::Rc;

use utils::error::Error as ServiceError;
use utils::logger;

struct ServiceUnit {
    comm: Rc<ServiceComm>,
    config: Rc<ServiceConfig>,
    mng: Rc<ServiceMng>,
    monitor: ServiceMonitor,
    exec_ctx: Rc<ExecContext>,
}

impl UnitObj for ServiceUnit {
    fn init(&self) {
        todo!()
    }

    fn done(&self) {
        todo!()
    }

    fn load(&self, paths: &Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
        self.config.load(paths)?;

        self.parse()?;

        self.service_add_extras()?;

        return self.service_verify();
    }

    fn coldplug(&self) {
        todo!()
    }

    fn start(&self) -> Result<(), UnitActionError> {
        log::debug!("begin to start the service unit");
        let started = self.mng.start_check()?;
        if started {
            log::debug!("service already in starting, just return immediately");
            return Ok(());
        }

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

    fn notify_message(
        &self,
        ucred: &UnixCredentials,
        messages: &HashMap<&str, &str>,
        fds: &Vec<i32>,
    ) -> Result<(), ServiceError> {
        log::debug!(
            "begin to start service notify message, ucred: {:?}, pids: {:?}, messages: {:?}",
            ucred,
            fds,
            messages
        );
        self.mng.notify_message(ucred, messages, fds)
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
        let config = Rc::new(ServiceConfig::new());
        let context = Rc::new(ExecContext::new());

        let rt = Rc::new(RunningData::new());
        let mng = Rc::new(ServiceMng::new(&comm, &config, &rt, &context));
        rt.attach_mng(mng.clone());
        ServiceUnit {
            comm: Rc::clone(&comm),
            config: Rc::clone(&config),
            mng: mng.clone(),
            monitor: ServiceMonitor::new(&config),
            exec_ctx: context.clone(),
        }
    }

    pub(super) fn parse(&self) -> Result<(), Box<dyn Error>> {
        match self.config.environments() {
            Some(envs) => {
                for env in envs {
                    let content: Vec<&str> = env.split("=").map(|s| s.trim()).collect();
                    if content.len() != 2 {
                        continue;
                    }

                    self.exec_ctx
                        .insert_env(content[0].to_string(), content[1].to_string());
                }
            }
            None => {}
        }

        match self.config.sockets() {
            Some(sockets) => {
                for socket in sockets {
                    self.comm.unit().insert_two_deps(
                        UnitRelations::UnitWants,
                        UnitRelations::UnitAfter,
                        socket.to_string(),
                    );

                    self.comm
                        .unit()
                        .insert_dep(UnitRelations::UnitTriggeredBy, socket.clone());
                }
            }
            None => {}
        }

        Ok(())
    }

    pub fn service_add_extras(&self) -> Result<(), Box<dyn Error>> {
        if self.config.service_type() == ServiceType::Notify {
            self.config.set_notify_access(NotifyAccess::Main);
        }

        Ok(())
    }

    pub fn service_verify(&self) -> Result<(), Box<dyn Error>> {
        if !self.config.config_data().borrow().Service.RemainAfterExit
            && self
                .config
                .config_data()
                .borrow()
                .Service
                .ExecStart
                .is_none()
        {
            return Err(Box::new(ServiceError::Other {
                msg: "No ExecStart command is configured and RemainAfterExit if false",
            }));
        }

        if self.config.service_type() != ServiceType::Oneshot
            && self.config.get_exec_cmds(ServiceCommand::Start).is_none()
        {
            return Err(Box::new(ServiceError::Other {
                msg: "No ExecStart command is configured, service type is not oneshot",
            }));
        }

        if self.config.service_type() != ServiceType::Oneshot
            && self
                .config
                .get_exec_cmds(ServiceCommand::Start)
                .unwrap()
                .len()
                > 1
        {
            return Err(Box::new(ServiceError::Other {
                msg:
                    "More than Oneshot ExecStart command is configured, service type is not oneshot",
            }));
        }

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
