use super::service_base::PLUGIN_NAME;
use super::service_comm::ServiceUnitComm;
use super::service_config::ServiceConfig;
use super::service_mng::RunningData;
use super::service_mng::ServiceMng;
use super::service_monitor::ServiceMonitor;
use super::service_rentry::{NotifyAccess, ServiceCommand, ServiceType};
use libutils::error::Error as ServiceError;
use libutils::logger;
use libutils::special::{BASIC_TARGET, SHUTDOWN_TARGET, SYSINIT_TARGET};
use nix::sys::signal::Signal;
use nix::sys::socket::UnixCredentials;
use nix::unistd::Pid;
use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use std::rc::Rc;
use sysmaster::error::UnitActionError;
use sysmaster::rel::{ReStation, Reliability};
use sysmaster::unit::{
    ExecContext, SubUnit, UmIf, UnitActiveState, UnitBase, UnitDependencyMask, UnitMngUtil,
    UnitRelations,
};

struct ServiceUnit {
    comm: Rc<ServiceUnitComm>,
    config: Rc<ServiceConfig>,
    mng: Rc<ServiceMng>,
    monitor: ServiceMonitor,
    exec_ctx: Rc<ExecContext>,
}

impl ReStation for ServiceUnit {
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

impl SubUnit for ServiceUnit {
    fn init(&self) {
        todo!()
    }

    fn done(&self) {
        todo!()
    }

    fn load(&self, paths: Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
        self.config.load(paths, true)?;

        self.parse()?;

        self.service_add_extras()?;

        self.service_verify()
    }

    fn start(&self) -> Result<(), UnitActionError> {
        log::debug!("begin to start the service unit.");
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

    fn stop(&self, force: bool) -> Result<(), UnitActionError> {
        log::debug!("begin to stop the service unit, force: {}.", force);
        if !force {
            self.mng.stop_check()?;
        }
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

    fn get_subunit_state(&self) -> String {
        self.mng.get_state()
    }

    fn attach_unit(&self, unit: Rc<dyn UnitBase>) {
        self.comm.attach_unit(unit);
        self.db_insert();
    }

    fn notify_message(
        &self,
        ucred: &UnixCredentials,
        messages: &HashMap<&str, &str>,
        fds: Vec<i32>,
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
    fn attach_um(&self, um: Rc<dyn UmIf>) {
        self.comm.attach_um(um);
    }

    fn attach_reli(&self, reli: Rc<Reliability>) {
        self.comm.attach_reli(reli);
    }
}

impl ServiceUnit {
    fn new(_um: Rc<dyn UmIf>) -> ServiceUnit {
        let comm = Rc::new(ServiceUnitComm::new());
        let config = Rc::new(ServiceConfig::new(&comm));
        let context = Rc::new(ExecContext::new());

        let rt = Rc::new(RunningData::new());
        let _mng = Rc::new(ServiceMng::new(&comm, &config, &rt, &context));
        rt.attach_mng(_mng.clone());
        ServiceUnit {
            comm: Rc::clone(&comm),
            config: Rc::clone(&config),
            mng: Rc::clone(&_mng),
            monitor: ServiceMonitor::new(&config),
            exec_ctx: Rc::clone(&context),
        }
    }

    fn parse_kill_context(&self) {
        self.config
            .kill_context()
            .set_kill_mode(self.config.config_data().borrow().Service.KillMode);
    }

    fn parse(&self) -> Result<(), Box<dyn Error>> {
        if let Some(envs) = self.config.environments() {
            for env in envs {
                let content: Vec<&str> = env.split('=').map(|s| s.trim()).collect();
                if content.len() != 2 {
                    continue;
                }

                self.exec_ctx
                    .insert_env(content[0].to_string(), content[1].to_string());
            }
        }
        if let Some(owner) = self.comm.owner() {
            if let Some(sockets) = self.config.sockets() {
                for socket in sockets {
                    let ret = owner.insert_two_deps(
                        UnitRelations::UnitWants,
                        UnitRelations::UnitAfter,
                        socket.to_string(),
                    );
                    if ret.is_ok() {
                        owner.insert_dep(UnitRelations::UnitTriggeredBy, socket.clone());
                    }
                }
            }
        }

        self.parse_kill_context();

        Ok(())
    }

    fn service_add_extras(&self) -> Result<(), Box<dyn Error>> {
        if self.config.service_type() == ServiceType::Notify {
            self.config.set_notify_access(NotifyAccess::Main);
        }

        self.add_default_dependencies().map_err(|_e| {
            Box::new(ServiceError::Other {
                msg: "add default dependency error",
            })
        })?;

        Ok(())
    }

    fn service_verify(&self) -> Result<(), Box<dyn Error>> {
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

    pub(self) fn add_default_dependencies(&self) -> Result<(), UnitActionError> {
        if let Some(u) = self.comm.owner() {
            log::debug!("add default dependencies for service [{}]", u.id());
            if !u.default_dependencies() {
                return Ok(());
            }

            let um = self.comm.um();

            um.unit_add_two_dependency(
                u.id(),
                UnitRelations::UnitAfter,
                UnitRelations::UnitRequires,
                SYSINIT_TARGET,
                true,
                UnitDependencyMask::UnitDependencyDefault,
            )?;

            um.unit_add_dependency(
                u.id(),
                UnitRelations::UnitAfter,
                BASIC_TARGET,
                true,
                UnitDependencyMask::UnitDependencyDefault,
            )?;

            um.unit_add_two_dependency(
                u.id(),
                UnitRelations::UnitBefore,
                UnitRelations::UnitConflicts,
                SHUTDOWN_TARGET,
                true,
                UnitDependencyMask::UnitDependencyDefault,
            )?;
        }

        Ok(())
    }
}

/*impl Default for ServiceUnit {
    fn default() -> Self {
        ServiceUnit::new()
    }
}*/

use sysmaster::declure_unitobj_plugin_with_param;
declure_unitobj_plugin_with_param!(ServiceUnit, ServiceUnit::new, PLUGIN_NAME);
