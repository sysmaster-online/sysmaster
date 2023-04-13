// Copyright (c) 2022 Huawei Technologies Co.,Ltd. All rights reserved.
//
// sysMaster is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan
// PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY
// KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

use crate::rentry::ServiceRestart;

use super::base::PLUGIN_NAME;
use super::comm::ServiceUnitComm;
use super::config::ServiceConfig;
use super::mng::RunningData;
use super::mng::ServiceMng;
use super::rentry::{NotifyAccess, ServiceCommand, ServiceType};
use basic::logger;
use basic::special::{BASIC_TARGET, SHUTDOWN_TARGET, SYSINIT_TARGET};
use nix::sys::signal::Signal;
use nix::sys::socket::UnixCredentials;
use nix::sys::wait::WaitStatus;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::str::FromStr;
use sysmaster::error::*;
use sysmaster::rel::{ReStation, Reliability};
use sysmaster::unit::{
    SubUnit, UmIf, UnitActiveState, UnitBase, UnitDependencyMask, UnitMngUtil, UnitRelations,
};

use sysmaster::exec::ExecContext;

struct ServiceUnit {
    comm: Rc<ServiceUnitComm>,
    config: Rc<ServiceConfig>,
    mng: Rc<ServiceMng>,
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

    fn load(&self, paths: Vec<PathBuf>) -> Result<()> {
        self.config.load(paths, true)?;

        self.parse()?;

        self.service_add_extras()?;

        self.service_verify()
    }

    fn start(&self) -> Result<()> {
        log::debug!("begin to start the service unit.");
        let started = self.mng.start_check()?;
        if started {
            log::debug!("service already in starting, just return immediately");
            return Ok(());
        }

        self.mng.start_action();

        Ok(())
    }

    fn dump(&self) {
        todo!()
    }

    fn stop(&self, force: bool) -> Result<()> {
        log::debug!("begin to stop the service unit, force: {}.", force);
        if !force {
            self.mng.stop_check()?;
        }
        self.mng.stop_action();
        Ok(())
    }

    fn reload(&self) -> Result<()> {
        self.mng.reload_action();
        Ok(())
    }

    fn can_reload(&self) -> bool {
        self.config
            .get_exec_cmds(ServiceCommand::Reload)
            .map_or(false, |cmds| !cmds.is_empty())
    }

    fn kill(&self) {
        todo!()
    }

    fn release_resources(&self) {
        todo!()
    }

    fn sigchld_events(&self, wait_status: WaitStatus) {
        self.mng.sigchld_event(wait_status)
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
    ) -> Result<()> {
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

        let rt = Rc::new(RunningData::new(&comm));
        let _mng = Rc::new(ServiceMng::new(&comm, &config, &rt, &context));
        rt.attach_mng(_mng.clone());
        ServiceUnit {
            comm: Rc::clone(&comm),
            config: Rc::clone(&config),
            mng: Rc::clone(&_mng),
            exec_ctx: Rc::clone(&context),
        }
    }

    fn parse_kill_context(&self) -> Result<()> {
        self.config
            .kill_context()
            .set_kill_mode(self.config.config_data().borrow().Service.KillMode);

        let signal = Signal::from_str(&self.config.config_data().borrow().Service.KillSignal)?;

        self.config.kill_context().set_kill_signal(signal);
        Ok(())
    }

    fn parse(&self) -> Result<()> {
        // if TimeoutSec is set, flush it's value to TimeoutStartSec and TimeoutStopSec
        self.config.flush_timeout();

        if let Some(envs) = self.config.environments() {
            for (key, value) in envs {
                self.exec_ctx.insert_env(key, value);
            }
        }

        self.exec_ctx.insert_envs_files(
            self.config
                .config_data()
                .borrow()
                .Service
                .EnvironmentFile
                .clone(),
        );

        if let Some(owner) = self.comm.owner() {
            if let Some(sockets) = self.config.sockets() {
                let um = self.comm.um();

                for socket in sockets {
                    if let Err(e) = um.unit_add_two_dependency(
                        owner.id(),
                        UnitRelations::UnitWants,
                        UnitRelations::UnitAfter,
                        &socket,
                        true,
                        UnitDependencyMask::File,
                    ) {
                        log::warn!(
                            "failed to add {:?} {:?} dependency of {}, error: {:?}",
                            UnitRelations::UnitWants,
                            UnitRelations::UnitAfter,
                            socket,
                            e
                        );
                    }

                    if let Err(e) = um.unit_add_dependency(
                        owner.id(),
                        UnitRelations::UnitTriggeredBy,
                        &socket,
                        true,
                        UnitDependencyMask::File,
                    ) {
                        log::warn!(
                            "failed to add {:?} dependency of {}, error: {:?}",
                            UnitRelations::UnitTriggeredBy,
                            socket,
                            e
                        );
                    }
                }
            }
        }

        self.parse_kill_context()?;

        Ok(())
    }

    fn service_add_extras(&self) -> Result<()> {
        if self.config.service_type() == ServiceType::Notify {
            self.config.set_notify_access(NotifyAccess::Main);
        }

        self.add_default_dependencies()?;

        Ok(())
    }

    fn service_verify(&self) -> Result<()> {
        if !self.config.config_data().borrow().Service.RemainAfterExit
            && self
                .config
                .get_exec_cmds(ServiceCommand::Start)
                .map_or(true, |cmds| cmds.is_empty())
        {
            return Err(Error::ConfigureError {
                msg: "No ExecStart command is configured and RemainAfterExit if false".to_string(),
            });
        }

        if self.config.service_type() != ServiceType::Oneshot
            && self
                .config
                .get_exec_cmds(ServiceCommand::Start)
                .map_or(true, |cmds| cmds.is_empty())
        {
            return Err(Error::ConfigureError {
                msg: "No ExecStart command is configured, service type is not oneshot".to_string(),
            });
        }

        if self.config.service_type() != ServiceType::Oneshot
            && self
                .config
                .get_exec_cmds(ServiceCommand::Start)
                .unwrap()
                .len()
                > 1
        {
            return Err(Error::ConfigureError {
                msg:
                    "More than Oneshot ExecStart command is configured, service type is not oneshot"
                        .to_string(),
            });
        }

        if self.config.service_type() == ServiceType::Oneshot
            && !matches!(
                self.config.config_data().borrow().Service.Restart,
                ServiceRestart::No
                    | ServiceRestart::OnFailure
                    | ServiceRestart::OnAbnormal
                    | ServiceRestart::OnWatchdog
                    | ServiceRestart::OnAbort
            )
        {
            return Err(Error::ConfigureError { msg:
                "When service type is onoshot, Restart= is not allowed set to Always or OnSuccess".to_string(),
        });
        }

        Ok(())
    }

    pub(self) fn add_default_dependencies(&self) -> Result<()> {
        let u = match self.comm.owner() {
            None => {
                return Ok(());
            }
            Some(v) => v,
        };

        if !u.default_dependencies() {
            return Ok(());
        }

        log::debug!("Adding default dependencies for service: {}", u.id());
        let um = self.comm.um();
        um.unit_add_two_dependency(
            u.id(),
            UnitRelations::UnitAfter,
            UnitRelations::UnitRequires,
            SYSINIT_TARGET,
            true,
            UnitDependencyMask::Default,
        )?;
        um.unit_add_dependency(
            u.id(),
            UnitRelations::UnitAfter,
            BASIC_TARGET,
            true,
            UnitDependencyMask::Default,
        )?;
        um.unit_add_two_dependency(
            u.id(),
            UnitRelations::UnitBefore,
            UnitRelations::UnitConflicts,
            SHUTDOWN_TARGET,
            true,
            UnitDependencyMask::Default,
        )?;

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
