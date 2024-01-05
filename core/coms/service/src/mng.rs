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

use super::comm::ServiceUnitComm;
use super::config::ServiceConfig;
use super::pid::ServicePid;
use super::rentry::{
    NotifyState, ServiceCommand, ServiceRestart, ServiceResult, ServiceState, ServiceType,
};
use super::spawn::ServiceSpawn;
use crate::monitor::ServiceMonitor;
use crate::rentry::{ExitStatus, NotifyAccess};
use basic::{do_entry_log, IN_SET};
use basic::{fs, process};
use core::error::*;
use core::exec::{ExecCommand, ExecContext, ExecFlag, ExecFlags, PreserveMode};
use core::rel::ReStation;
use core::unit::{KillOperation, UnitActiveState, UnitNotifyFlags};
use core::unit::{PathSpec, PathType};
use event::{EventState, EventType, Events, Source};
use log::Level;
use nix::libc;
use nix::sys::signal::Signal;
use nix::sys::socket::UnixCredentials;
use nix::sys::wait::WaitStatus;
use nix::unistd::Pid;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::os::unix::prelude::RawFd;
use std::path::PathBuf;
use std::rc::{Rc, Weak};

pub(super) struct ServiceMng {
    // associated objects
    comm: Rc<ServiceUnitComm>,
    config: Rc<ServiceConfig>,

    // owned objects
    pid: Rc<ServicePid>,
    spawn: ServiceSpawn,
    state: RefCell<ServiceState>,
    result: RefCell<ServiceResult>,
    reload_result: RefCell<ServiceResult>,

    main_command: RefCell<VecDeque<ExecCommand>>,
    control_cmd_type: RefCell<Option<ServiceCommand>>,
    control_command: RefCell<VecDeque<ExecCommand>>,
    rd: Rc<RunningData>,
    monitor: RefCell<ServiceMonitor>,
    current_main_command: RefCell<ExecCommand>,
    current_control_command: RefCell<ExecCommand>,
}

impl ReStation for ServiceMng {
    // input: do nothing

    // compensate: do nothing

    // data
    fn db_map(&self, _reload: bool) {
        if let Some((
            state,
            result,
            m_pid,
            c_pid,
            main_cmd_len,
            control_cmd_type,
            control_cmd_len,
            notify_state,
            forbid_restart,
            reset_restart,
            restarts,
            exit_status,
            monitor,
        )) = self.comm.rentry_mng_get()
        {
            *self.state.borrow_mut() = state;
            *self.result.borrow_mut() = result;
            self.pid.update_main(m_pid);
            self.pid.update_control(c_pid);
            self.main_command_update(main_cmd_len);
            self.control_command_update(control_cmd_type, control_cmd_len);
            self.rd.set_notify_state(notify_state);
            self.rd.set_forbid_restart(forbid_restart);
            self.rd.set_reset_restart(reset_restart);
            self.rd.set_restarts(restarts);
            self.rd.set_wait_status(WaitStatus::from(exit_status));
            *self.monitor.borrow_mut() = monitor;
        }
    }

    fn db_insert(&self) {
        let exit_status = ExitStatus::from(self.rd.wait_status());
        self.comm.rentry_mng_insert(
            self.state(),
            self.result(),
            self.pid.main(),
            self.pid.control(),
            self.main_command.borrow().len(),
            *self.control_cmd_type.borrow(),
            self.control_command.borrow().len(),
            self.rd.notify_state(),
            self.rd.forbid_restart(),
            self.rd.reset_restart(),
            self.rd.restarts(),
            exit_status,
            *self.monitor.borrow(),
        );
    }

    // reload: no external connections
    fn entry_coldplug(&self) {
        self.rd.enable_timer(self.coldplug_timeout()).unwrap();
        self.restart_watchdog();
    }

    fn entry_clear(&self) {
        // pid_file is a transient file that can be directly closed
        self.unwatch_pid_file();

        self.stop_watchdog();

        let events = self.comm.um().events();
        events.del_source(self.rd.timer()).unwrap();
    }
}

impl ServiceMng {
    pub(super) fn new(
        commr: &Rc<ServiceUnitComm>,
        configr: &Rc<ServiceConfig>,
        rd: &Rc<RunningData>,
        exec_ctx: &Rc<ExecContext>,
    ) -> ServiceMng {
        let _pid = Rc::new(ServicePid::new(commr));

        ServiceMng {
            comm: Rc::clone(commr),
            config: Rc::clone(configr),
            pid: Rc::clone(&_pid),
            spawn: ServiceSpawn::new(commr, &_pid, configr, exec_ctx, rd),
            state: RefCell::new(ServiceState::Dead),
            result: RefCell::new(ServiceResult::Success),
            reload_result: RefCell::new(ServiceResult::Success),

            main_command: RefCell::new(VecDeque::new()),
            control_cmd_type: RefCell::new(None),
            control_command: RefCell::new(VecDeque::new()),
            rd: rd.clone(),
            monitor: RefCell::new(ServiceMonitor::new()),
            current_main_command: RefCell::new(ExecCommand::empty()),
            current_control_command: RefCell::new(ExecCommand::empty()),
        }
    }

    pub(super) fn start_check(&self) -> Result<bool> {
        if IN_SET!(
            self.state(),
            ServiceState::Stop,
            ServiceState::StopWatchdog,
            ServiceState::StopSigterm,
            ServiceState::StopSigkill,
            ServiceState::StopPost,
            ServiceState::FinalWatchdog,
            ServiceState::FinalSigterm,
            ServiceState::FinalSigkill,
            ServiceState::Cleaning
        ) {
            return Err(Error::UnitActionEAgain);
        }

        // service is in starting
        if IN_SET!(
            self.state(),
            ServiceState::Condition,
            ServiceState::StartPre,
            ServiceState::Start,
            ServiceState::StartPost
        ) {
            return Ok(true);
        }
        let ret = self.comm.owner().map(|u| u.test_start_limit());
        if ret.is_none() || !ret.unwrap() {
            self.enter_dead(ServiceResult::FailureStartLimitHit, false);
            return Err(Error::UnitActionECanceled);
        }

        Ok(false)
    }

    pub(super) fn start_action(&self) {
        if self.rd.reset_restart() {
            self.rd.clear_restarts();
            self.rd.set_reset_restart(false);
        }
        self.set_result(ServiceResult::Success);
        self.rd.set_forbid_restart(false);
        self.enter_contion();
        self.db_update();
    }

    pub(super) fn stop_check(&self) -> Result<()> {
        if IN_SET!(
            self.state(),
            ServiceState::Stop,
            ServiceState::StopSigterm,
            ServiceState::StopSigkill,
            ServiceState::StopPost,
            ServiceState::FinalSigterm,
            ServiceState::FinalSigkill
        ) {
            return Ok(());
        }

        Ok(())
    }

    pub(super) fn stop_action(&self) {
        self.rd.set_forbid_restart(true);

        /* logic same as service_stop() in systemd */
        if vec![
            ServiceState::Stop,
            ServiceState::StopSigterm,
            ServiceState::StopSigkill,
            ServiceState::StopPost,
            ServiceState::FinalWatchdog,
            ServiceState::FinalSigterm,
            ServiceState::FinalWatchdog,
        ]
        .contains(&self.state())
        {
            return;
        }

        if self.state() == ServiceState::AutoRestart {
            self.set_state(ServiceState::Dead);
            return;
        }

        if vec![
            ServiceState::Condition,
            ServiceState::StartPre,
            ServiceState::Start,
            ServiceState::StartPost,
            ServiceState::Reload,
            ServiceState::StopWatchdog,
        ]
        .contains(&self.state())
        {
            self.enter_signal(ServiceState::StopSigterm, ServiceResult::Success);
            self.db_update();
            return;
        }

        if self.state() == ServiceState::Cleaning {
            self.enter_signal(ServiceState::FinalSigkill, ServiceResult::Success);
            return;
        }

        self.enter_stop(ServiceResult::Success);
        self.db_update();
    }

    pub(super) fn reload_action(&self) {
        self.enter_reload();
        self.db_update();
    }

    pub(super) fn current_active_state(&self) -> UnitActiveState {
        service_state_to_unit_state(self.config.service_type(), self.state())
    }

    fn enter_contion(&self) {
        self.log(Level::Debug, "enter running service condition command");

        self.control_command_fill(ServiceCommand::Condition);
        match self.control_command_pop() {
            Some(cmd) => {
                *self.current_control_command.borrow_mut() = cmd.clone();
                match self.spawn.start_service(
                    &cmd,
                    self.config.config_data().borrow().Service.TimeoutStartSec,
                    ExecFlags::CONTROL,
                ) {
                    Ok(pid) => self.pid.set_control(pid),
                    Err(_e) => {
                        self.enter_dead(ServiceResult::FailureResources, true);
                        return;
                    }
                }
                self.set_state(ServiceState::Condition);
            }
            None => {
                self.enter_prestart();
            }
        }
    }

    fn enter_prestart(&self) {
        self.log(Level::Debug, "enter running service prestart command");
        self.pid.unwatch_control();
        self.control_command_fill(ServiceCommand::StartPre);
        match self.control_command_pop() {
            Some(cmd) => {
                *self.current_control_command.borrow_mut() = cmd.clone();
                match self.spawn.start_service(
                    &cmd,
                    self.config.config_data().borrow().Service.TimeoutStartSec,
                    ExecFlags::CONTROL,
                ) {
                    Ok(pid) => self.pid.set_control(pid),
                    Err(_e) => {
                        self.enter_dead(ServiceResult::FailureResources, true);
                        return;
                    }
                }
                self.set_state(ServiceState::StartPre);
            }
            None => self.enter_start(),
        }
    }

    fn enter_start(&self) {
        self.log(Level::Debug, "enter running service start command");

        self.control_command.borrow_mut().clear();
        self.pid.unwatch_control();
        self.pid.unwatch_main();
        self.main_command_fill();

        let service_type = self.config.service_type();

        let cmd = if service_type == ServiceType::Forking {
            self.control_command_fill(ServiceCommand::Start);
            self.control_command_pop()
        } else {
            self.main_command_fill();
            self.main_command_pop()
        };

        if cmd.is_none() {
            if self.config.service_type() != ServiceType::Oneshot {
                log::error!("no start command is configured and service type is not oneshot");
                self.enter_signal(ServiceState::StopSigterm, ServiceResult::FailureResources);
                return;
            }
            self.set_state(ServiceState::Start);
            self.enter_start_post();
            return;
        }
        let cmd = cmd.unwrap();

        // for Simple and Idle service type, disable the timer.
        let time_out = match service_type {
            ServiceType::Simple | ServiceType::Idle => u64::MAX,
            _ => self.config.config_data().borrow().Service.TimeoutStartSec,
        };

        let ret = self.spawn.start_service(
            &cmd,
            time_out,
            ExecFlags::PASS_FDS | ExecFlags::SOFT_WATCHDOG,
        );

        if ret.is_err() {
            log::error!(
                "failed to start service: unit Name{}",
                self.comm.get_owner_id()
            );
            self.enter_signal(ServiceState::StopSigterm, ServiceResult::FailureResources);
            return;
        }
        *self.current_main_command.borrow_mut() = cmd;
        let pid = ret.unwrap();
        log::debug!(
            "service type is: {:?}, forking pid is: {}",
            service_type,
            pid
        );

        match service_type {
            ServiceType::Simple => {
                let _ = self.pid.set_main(pid);
                self.enter_start_post();
            }
            ServiceType::Forking => {
                // for forking service type, we consider the process startup complete when the process exit;
                log::debug!("in forking type, set pid {} to control pid", pid);
                self.pid.set_control(pid);
                self.set_state(ServiceState::Start);
            }
            ServiceType::Oneshot | ServiceType::Notify => {
                let _ = self.pid.set_main(pid);
                self.set_state(ServiceState::Start);
            }

            ServiceType::Idle => todo!(),
            ServiceType::Exec => todo!(),
            _ => {}
        }
    }

    fn enter_start_post(&self) {
        self.log(Level::Debug, "enter running service startpost command");

        self.pid.unwatch_control();

        self.restart_watchdog();

        self.control_command_fill(ServiceCommand::StartPost);
        match self.control_command_pop() {
            Some(cmd) => {
                *self.current_control_command.borrow_mut() = cmd.clone();
                match self.spawn.start_service(
                    &cmd,
                    self.config.config_data().borrow().Service.TimeoutStartSec,
                    ExecFlags::CONTROL,
                ) {
                    Ok(pid) => self.pid.set_control(pid),
                    Err(_e) => {
                        log::error!(
                            "Failed to run start post service, unit name:{}",
                            self.comm.get_owner_id()
                        );
                        return;
                    }
                }
                self.set_state(ServiceState::StartPost);
            }
            None => self.enter_running(ServiceResult::Success),
        }
    }

    fn enter_running(&self, sr: ServiceResult) {
        self.pid.unwatch_control();
        if self.result() == ServiceResult::Success {
            self.set_result(sr);
        }

        if self.result() != ServiceResult::Success {
            self.enter_signal(ServiceState::StopSigterm, sr);
        } else if self.service_alive() {
            if self.rd.notify_state() == NotifyState::Stopping {
                self.enter_stop_by_notify();
            } else {
                self.set_state(ServiceState::Running);
                log::info!("Started {}", self.comm.get_owner_id());
                // for running service, the default timeout is runtime_max_usec, the default value is U64::MAX for not enable timer
                if let Err(e) = self.rd.enable_timer(u64::MAX) {
                    self.log(
                        Level::Warn,
                        &format!("enter running enable timer error: {}", e),
                    );
                }
            }
        } else if self.config.config_data().borrow().Service.RemainAfterExit {
            self.set_state(ServiceState::Exited);
        } else {
            self.enter_stop(sr);
        }
    }

    fn enter_stop(&self, res: ServiceResult) {
        self.log(
            Level::Debug,
            &format!("enter running stop command, service result: {:?}", res),
        );

        if self.result() == ServiceResult::Success {
            self.set_result(res);
        }

        self.control_command_fill(ServiceCommand::Stop);
        let cmd = match self.control_command_pop() {
            None => {
                self.enter_signal(ServiceState::StopSigterm, ServiceResult::Success);
                return;
            }
            Some(v) => v,
        };
        *self.current_control_command.borrow_mut() = cmd.clone();

        let time_out = self.config.config_data().borrow().Service.TimeoutStopSec;
        match self.spawn.start_service(&cmd, time_out, ExecFlags::CONTROL) {
            Ok(pid) => self.pid.set_control(pid),
            Err(_e) => {
                log::error!("Failed to run ExecStop of {}", self.comm.get_owner_id());
                self.enter_signal(ServiceState::StopSigterm, ServiceResult::FailureResources);
                return;
            }
        }
        self.set_state(ServiceState::Stop);
    }

    fn enter_stop_by_notify(&self) {
        // todo tidy pids

        if let Err(e) = self
            .rd
            .enable_timer(self.config.config_data().borrow().Service.TimeoutStopSec)
        {
            self.log(
                Level::Warn,
                &format!("action notify stop enable timer error: {}", e),
            );
        }

        self.set_state(ServiceState::StopSigterm);
    }

    fn enter_stop_post(&self, res: ServiceResult) {
        self.log(
            Level::Debug,
            &format!("running stop post, service result: {:?}", res),
        );
        if self.result() == ServiceResult::Success {
            self.set_result(res);
        }

        self.control_command_fill(ServiceCommand::StopPost);
        let cmd = match self.control_command_pop() {
            None => {
                self.enter_signal(ServiceState::FinalSigterm, ServiceResult::Success);
                return;
            }
            Some(v) => v,
        };
        *self.current_control_command.borrow_mut() = cmd.clone();

        let time_out = self.config.config_data().borrow().Service.TimeoutStopSec;
        match self.spawn.start_service(&cmd, time_out, ExecFlags::CONTROL) {
            Ok(pid) => self.pid.set_control(pid),
            Err(_e) => {
                self.enter_signal(ServiceState::FinalSigterm, ServiceResult::FailureResources);
                log::error!("Failed to run ExecStopPost of {}", self.comm.get_owner_id());
                return;
            }
        }
        self.set_state(ServiceState::StopPost);
    }

    fn enter_dead(&self, res: ServiceResult, force_restart: bool) {
        self.log(
            Level::Debug,
            &format!(
                "Running into dead state, res: {:?}, current res: {:?}, restart: {}",
                res,
                self.result(),
                force_restart
            ),
        );
        log::info!("Stopped {}", self.comm.get_owner_id());
        let mut restart = force_restart;

        if self.comm.owner().is_none() {
            return;
        }

        if self
            .comm
            .um()
            .has_stop_job(&self.comm.owner().unwrap().id())
        {
            restart = false;
        }

        if self.result() == ServiceResult::Success {
            self.set_result(res);
        }

        let state = if self.result() == ServiceResult::Success {
            ServiceState::Dead
        } else {
            ServiceState::Failed
        };

        if !restart {
            log::debug!("not allowded restart");
        } else {
            restart = self.shall_restart();
            if restart {
                self.rd.set_will_auto_restart(true)
            }
        }

        self.set_state(state);
        if restart {
            self.rd.set_will_auto_restart(false);
            if let Err(e) = self
                .rd
                .enable_timer(self.config.config_data().borrow().Service.RestartSec)
            {
                self.log(
                    Level::Warn,
                    &format!("auto restart start timer error: {}", e),
                );
                self.enter_dead(ServiceResult::FailureResources, false);
                return;
            }
            self.set_state(ServiceState::AutoRestart);
        } else {
            self.rd.set_reset_restart(true);
        }

        self.rd.set_forbid_restart(false);

        let preserve_mode = self
            .config
            .config_data()
            .borrow()
            .Service
            .RuntimeDirectoryPreserve;
        if preserve_mode == PreserveMode::No
            || preserve_mode == PreserveMode::Restart && !self.rd.will_restart()
        {
            let runtime_directory = self.spawn.runtime_directory();
            let _ = self.comm.um().unit_destroy_runtime_data(runtime_directory);
        }

        if let Some(p) = self.config.pid_file() {
            do_entry_log!(nix::unistd::unlink, p, "unlink");
        }
    }

    fn enter_reload(&self) {
        log::debug!("running service reload command");
        self.control_command.borrow_mut().clear();
        self.pid.unwatch_control();
        self.control_command_fill(ServiceCommand::Reload);
        self.set_reload_result(ServiceResult::Success);

        match self.control_command_pop() {
            Some(cmd) => {
                *self.current_control_command.borrow_mut() = cmd.clone();
                match self.spawn.start_service(
                    &cmd,
                    self.config.config_data().borrow().Service.TimeoutStartSec,
                    ExecFlags::CONTROL,
                ) {
                    Ok(pid) => self.pid.set_control(pid),
                    Err(_e) => {
                        log::error!("failed to start service: {}", self.comm.get_owner_id());
                        self.set_reload_result(ServiceResult::FailureResources);
                        self.enter_running(ServiceResult::Success);
                        return;
                    }
                }
                self.set_state(ServiceState::Reload);
            }
            None => self.enter_running(ServiceResult::Success),
        }
    }

    fn enter_restart(&self) {
        if self
            .comm
            .um()
            .has_stop_job(&self.comm.owner().unwrap().id())
        {
            log::info!("there is stop in pending, not restart");
            return;
        }

        if let Err(e) = self
            .comm
            .um()
            .restart_unit(&self.comm.get_owner_id(), false)
        {
            log::debug!(
                "failed to restart unit:{}, errno: {:?}",
                &self.comm.get_owner_id(),
                e
            );
            self.enter_dead(ServiceResult::FailureResources, false);
            return;
        }

        self.rd.add_restarts();
        self.rd.set_reset_restart(false);
        log::info!(
            "restart unit {}; restart times: {}",
            &self.comm.get_owner_id(),
            self.rd.restarts()
        );
    }

    fn enter_signal(&self, state: ServiceState, res: ServiceResult) {
        self.log(
            Level::Debug,
            &format!(
                "Sending signal of state: {:?}, service result: {:?}",
                state, res
            ),
        );

        if self.result() == ServiceResult::Success {
            self.set_result(res);
        }

        let unit = self.comm.owner().expect("unit is not attached");

        let op = state.to_kill_operation();
        self.comm
            .um()
            .child_watch_all_pids(&self.comm.get_owner_id());

        let ret = unit.kill_context(
            self.config.kill_context(),
            self.pid.main(),
            self.pid.control(),
            op,
            self.pid.main_pid_alien(),
        );

        if ret.is_err() {
            if matches!(
                state,
                ServiceState::StopWatchdog | ServiceState::StopSigterm | ServiceState::StopSigkill
            ) {
                return self.enter_stop_post(ServiceResult::FailureResources);
            } else {
                return self.enter_dead(ServiceResult::FailureResources, true);
            }
        }

        if ret.unwrap() {
            if let Err(e) = self
                .rd
                .enable_timer(self.config.config_data().borrow().Service.TimeoutStopSec)
            {
                self.log(
                    Level::Error,
                    &format!("in enter signal start timer error: {}", e),
                );

                if matches!(
                    state,
                    ServiceState::StopWatchdog
                        | ServiceState::StopSigterm
                        | ServiceState::StopSigkill
                ) {
                    return self.enter_stop_post(ServiceResult::FailureResources);
                } else {
                    return self.enter_dead(ServiceResult::FailureResources, true);
                }
            }
            self.set_state(state);
        } else if matches!(
            state,
            ServiceState::StopWatchdog | ServiceState::StopSigterm | ServiceState::StopSigkill
        ) {
            self.enter_stop_post(ServiceResult::Success);
        } else if matches!(
            state,
            ServiceState::FinalWatchdog | ServiceState::FinalSigterm
        ) {
            self.enter_signal(ServiceState::FinalSigkill, ServiceResult::Success);
        } else {
            self.enter_dead(ServiceResult::Success, true);
        }
    }

    fn set_state(&self, state: ServiceState) {
        let original_state = self.state();
        *self.state.borrow_mut() = state;

        self.log(
            Level::Debug,
            &format!(
                "unit: {}, original state: {:?}, change to: {:?}",
                self.comm.get_owner_id(),
                original_state,
                state
            ),
        );

        if !matches!(
            self.state(),
            ServiceState::Condition
                | ServiceState::StartPre
                | ServiceState::Start
                | ServiceState::StartPost
                | ServiceState::Running
                | ServiceState::Reload
                | ServiceState::Stop
                | ServiceState::StopWatchdog
                | ServiceState::StopSigterm
                | ServiceState::StopSigkill
                | ServiceState::StopPost
                | ServiceState::FinalWatchdog
                | ServiceState::FinalSigterm
                | ServiceState::FinalSigkill
                | ServiceState::AutoRestart
                | ServiceState::Cleaning
        ) {
            self.rd.delete_timer();
        }

        // TODO
        // check the new state
        if !vec![
            ServiceState::Start,
            ServiceState::StartPost,
            ServiceState::Running,
            ServiceState::Reload,
            ServiceState::Stop,
            ServiceState::StopWatchdog,
            ServiceState::StopSigterm,
            ServiceState::StopSigkill,
            ServiceState::StopPost,
            ServiceState::FinalWatchdog,
            ServiceState::FinalSigterm,
            ServiceState::FinalSigkill,
        ]
        .contains(&state)
        {
            self.pid.unwatch_main();
            self.main_command.borrow_mut().clear();
        }

        if !vec![
            ServiceState::Condition,
            ServiceState::StartPre,
            ServiceState::Start,
            ServiceState::StartPost,
            ServiceState::Reload,
            ServiceState::Stop,
            ServiceState::StopWatchdog,
            ServiceState::StopSigterm,
            ServiceState::StopSigkill,
            ServiceState::StopPost,
            ServiceState::FinalWatchdog,
            ServiceState::FinalSigterm,
            ServiceState::FinalSigkill,
            ServiceState::Cleaning,
        ]
        .contains(&state)
        {
            self.pid.unwatch_control();
            self.control_command.borrow_mut().clear();
            self.set_cmd_type(None);
        }

        if vec![
            ServiceState::Dead,
            ServiceState::Failed,
            ServiceState::AutoRestart,
        ]
        .contains(&state)
        {
            self.pid.child_unwatch_all_pids();
        }

        // todo!()
        // trigger the unit the dependency trigger_by
        self.comm
            .um()
            .trigger_notify(&self.comm.owner().unwrap().id());

        let os = service_state_to_unit_state(self.config.service_type(), original_state);
        let ns = service_state_to_unit_state(self.config.service_type(), state);
        if let Some(u) = self.comm.owner() {
            let mut flags = UnitNotifyFlags::EMPTY;

            if self.rd.will_auto_restart() {
                flags |= UnitNotifyFlags::WILL_AUTO_RESTART;
            }

            if self.reload_result() != ServiceResult::Success {
                flags |= UnitNotifyFlags::RELOAD_FAILURE;
            }
            u.notify(os, ns, flags)
        }
    }

    fn set_cmd_type(&self, cmd_type: Option<ServiceCommand>) {
        *self.control_cmd_type.borrow_mut() = cmd_type;
    }

    fn service_alive(&self) -> bool {
        if let Ok(v) = self.pid.main_alive() {
            return v;
        }

        self.cgroup_good()
    }

    fn run_next_control(&self) {
        self.log(Level::Debug, "runring next control command");
        let time_out = match self.state() {
            ServiceState::Condition
            | ServiceState::StartPre
            | ServiceState::Start
            | ServiceState::StartPost
            | ServiceState::Running
            | ServiceState::Reload => self.config.config_data().borrow().Service.TimeoutStartSec,
            _ => self.config.config_data().borrow().Service.TimeoutStopSec,
        };

        if let Some(cmd) = self.control_command_pop() {
            *self.current_control_command.borrow_mut() = cmd.clone();
            match self.spawn.start_service(&cmd, time_out, ExecFlags::CONTROL) {
                Ok(pid) => self.pid.set_control(pid),
                Err(_e) => {
                    log::error!("failed to start service: {}", self.comm.get_owner_id());
                    if matches!(
                        self.state(),
                        ServiceState::Condition
                            | ServiceState::StartPre
                            | ServiceState::StartPost
                            | ServiceState::Stop
                    ) {
                        self.enter_signal(
                            ServiceState::StopSigterm,
                            ServiceResult::FailureResources,
                        );
                    } else if matches!(self.state(), ServiceState::StopPost) {
                        self.enter_dead(ServiceResult::FailureResources, true);
                    } else if matches!(self.state(), ServiceState::Reload) {
                        self.set_reload_result(ServiceResult::FailureResources);
                        self.enter_running(ServiceResult::Success);
                    } else {
                        self.enter_stop(ServiceResult::FailureResources);
                    }
                }
            }
        }
    }

    fn run_next_main(&self) {
        if let Some(cmd) = self.main_command_pop() {
            match self.spawn.start_service(
                &cmd,
                self.config.config_data().borrow().Service.TimeoutStartSec,
                ExecFlags::PASS_FDS | ExecFlags::SOFT_WATCHDOG,
            ) {
                Ok(pid) => {
                    let _ = self.pid.set_main(pid);
                }
                Err(_e) => {
                    log::error!("failed to run main command: {}", self.comm.get_owner_id());
                }
            }
        }
    }

    pub fn get_state(&self) -> String {
        let state = *self.state.borrow();
        state.to_string()
    }

    pub(super) fn state(&self) -> ServiceState {
        *self.state.borrow()
    }

    fn set_result(&self, result: ServiceResult) {
        *self.result.borrow_mut() = result;
    }

    fn result(&self) -> ServiceResult {
        *self.result.borrow()
    }

    fn set_reload_result(&self, result: ServiceResult) {
        *self.reload_result.borrow_mut() = result;
    }

    fn reload_result(&self) -> ServiceResult {
        *self.reload_result.borrow()
    }

    fn main_command_fill(&self) {
        let cmd_type = ServiceCommand::Start;
        if let Some(cmds) = self.config.get_exec_cmds(cmd_type) {
            *self.main_command.borrow_mut() = cmds
        }
    }

    fn main_command_pop(&self) -> Option<ExecCommand> {
        self.main_command.borrow_mut().pop_front()
    }

    fn main_command_update(&self, len: usize) {
        self.main_command.borrow_mut().clear();
        self.main_command_fill();
        let max = self.main_command.borrow().len();
        for _i in len..max {
            self.main_command_pop();
        }
    }

    fn control_command_fill(&self, cmd_type: ServiceCommand) {
        if let Some(cmds) = self.config.get_exec_cmds(cmd_type) {
            *self.control_command.borrow_mut() = cmds;
            if !self.control_command.borrow().is_empty() {
                self.set_cmd_type(Some(cmd_type));
            }
        }
    }

    fn control_command_pop(&self) -> Option<ExecCommand> {
        self.control_command.borrow_mut().pop_front()
    }

    fn control_command_update(&self, cmd_type: Option<ServiceCommand>, len: usize) {
        if let Some(c_type) = cmd_type {
            self.control_command.borrow_mut().clear();
            self.control_command_fill(c_type);
            let max = self.control_command.borrow().len();
            for _i in len..max {
                self.control_command_pop();
            }
        } else {
            assert_eq!(len, 0);
        }
    }

    fn load_pid_file(&self) -> Result<bool> {
        let pid_file = match self.config.pid_file() {
            Some(v) => v,
            None => {
                return Err(Error::Other {
                    msg: "pid file is not configured".to_string(),
                })
            }
        };

        let pid_file_path = pid_file.as_path();
        if !pid_file_path.exists() || !pid_file_path.is_file() {
            return Err(Error::NotFound {
                what: "pid file is not a file or not exist".to_string(),
            });
        }

        let pid = match fs::read_first_line(pid_file_path) {
            Ok(line) => line.trim().parse::<i32>(),
            Err(e) => {
                return Err(Error::Parse {
                    source: Box::new(e),
                })
            }
        };

        if pid.is_err() {
            log::debug!(
                "failed to parse pid from pid_file {:?}, err: {:?}",
                pid_file_path,
                pid
            );
            return Err(Error::Other {
                msg: "parsed the pid from pid file failed".to_string(),
            });
        }

        let pid = Pid::from_raw(pid.unwrap());
        if self.pid.main().is_some() && self.pid.main().unwrap() == pid {
            return Ok(false);
        }

        self.valid_main_pid(pid)?;

        self.pid.unwatch_main();
        self.pid.set_main(pid).map_err(|_e| Error::Other {
            msg: "invalid main pid".to_string(),
        })?;

        self.comm
            .um()
            .child_watch_pid(&self.comm.get_owner_id(), pid);

        Ok(true)
    }

    fn valid_main_pid(&self, pid: Pid) -> Result<bool> {
        if pid == nix::unistd::getpid() {
            return Err(Error::Other {
                msg: "main pid is the sysmaster's pid".to_string(),
            });
        }

        if self.pid.control().is_some() && self.pid.control().unwrap() == pid {
            return Err(Error::Other {
                msg: "main pid is the control process".to_string(),
            });
        }

        if !process::alive(pid) {
            return Err(Error::Other {
                msg: "main pid is not alive".to_string(),
            });
        }
        if self
            .comm
            .um()
            .same_unit_with_pid(&self.comm.get_owner_id(), pid)
        {
            return Ok(true);
        }

        Ok(false)
    }

    fn demand_pid_file(&self) -> Result<()> {
        let pid_file_inotify =
            PathInotify::new(self.config.pid_file().unwrap(), PathType::Modified);

        self.rd.attach_inotify(Rc::new(pid_file_inotify));

        self.watch_pid_file()
    }

    fn watch_pid_file(&self) -> Result<()> {
        let pid_file_inotify = self.rd.path_inotify();
        log::debug!("watch pid file: {}", pid_file_inotify);
        match pid_file_inotify.watch() {
            Ok(_) => {
                let events = self.comm.um().events();
                let source = Rc::clone(&pid_file_inotify);
                events.add_source(source).unwrap();
                let source = Rc::clone(&pid_file_inotify);
                events.set_enabled(source, EventState::On).unwrap();

                if let Err(e) = self.retry_pid_file() {
                    log::warn!("retry load pid file error: {}, Ignore and Continue", e);
                }
                Ok(())
            }

            Err(e) => {
                log::debug!(
                    "failed to add watch for pid file {}, err: {}",
                    pid_file_inotify,
                    e
                );
                self.unwatch_pid_file();

                Err(e)
            }
        }
    }

    fn unwatch_pid_file(&self) {
        self.log(
            Level::Debug,
            &format!("unwatch pid file {}", self.rd.path_inotify()),
        );
        let events = self.comm.um().events();
        events.del_source(self.rd.path_inotify()).unwrap();
        self.rd.path_inotify().unwatch();
    }

    fn retry_pid_file(&self) -> Result<bool> {
        self.log(
            Level::Debug,
            &format!("retry loading pid file: {}", self.rd.path_inotify()),
        );
        self.load_pid_file()?;

        self.unwatch_pid_file();
        self.enter_running(ServiceResult::Success);

        Ok(true)
    }

    fn cgroup_good(&self) -> bool {
        if let Some(Ok(v)) = self
            .comm
            .owner()
            .map(|u| cgroup::cg_is_empty_recursive(&u.cg_path()))
        {
            return !v;
        }

        false
    }

    fn guess_main_pid(&self) {
        if self.pid.main().is_some() {
            return;
        }

        if let Some(u) = self.comm.owner() {
            if let Ok(pid) = u.guess_main_pid() {
                if let Err(e) = self.pid.set_main(pid) {
                    log::error!("set main pid error: {}", e);
                    return;
                }
                self.comm
                    .um()
                    .child_watch_pid(&self.comm.get_owner_id(), pid);
            }
        }
    }

    fn shall_restart(&self) -> bool {
        if self.rd.forbid_restart() {
            return false;
        }

        if self
            .config
            .config_data()
            .borrow()
            .Service
            .RestartPreventExitStatus
            .exit_status_enabled(self.rd.wait_status())
        {
            return false;
        }

        match self.config.config_data().borrow().Service.Restart {
            ServiceRestart::No => false,
            ServiceRestart::OnSuccess => self.result() == ServiceResult::Success,
            ServiceRestart::OnFailure => !matches!(
                self.result(),
                ServiceResult::Success | ServiceResult::SkipCondition
            ),
            ServiceRestart::OnWatchdog => self.result() == ServiceResult::FailureWatchdog,
            ServiceRestart::OnAbnormal => !matches!(
                self.result(),
                ServiceResult::Success
                    | ServiceResult::FailureExitCode
                    | ServiceResult::SkipCondition
            ),
            ServiceRestart::OnAbort => {
                matches!(
                    self.result(),
                    ServiceResult::FailureSignal | ServiceResult::FailureCoreDump
                )
            }
            ServiceRestart::Always => self.result() != ServiceResult::SkipCondition,
        }
    }

    fn restart_watchdog(&self) {
        self.monitor
            .borrow_mut()
            .set_original_watchdog(self.config.config_data().borrow().Service.WatchdogSec);
        let watchdog_usec = self.monitor.borrow().watchdog_usec();
        if watchdog_usec == 0 || watchdog_usec == u64::MAX {
            self.stop_watchdog();
            return;
        }

        log::debug!("service create watchdog timer: {}", watchdog_usec);
        if self.rd.armd_watchdog() {
            let events = self.comm.um().events();
            let watchdog = self.rd.watchdog();
            events.del_source(watchdog.clone()).unwrap();

            watchdog.set_time(watchdog_usec);
            events.add_source(watchdog.clone()).unwrap();
            events.set_enabled(watchdog, EventState::OneShot).unwrap();
            return;
        }

        let watchdog = Rc::new(ServiceMonitorData::new(watchdog_usec));
        self.rd.attach_watchdog(watchdog.clone());

        let events = self.comm.um().events();
        events.add_source(watchdog.clone()).unwrap();
        events.set_enabled(watchdog, EventState::OneShot).unwrap();
    }

    fn force_watchdog(&self) {
        //todo!("check the global service_watchdogs was enabled")

        self.enter_signal(ServiceState::StopWatchdog, ServiceResult::FailureWatchdog);
    }

    fn override_watchdog_usec(&self, usec: u64) {
        self.monitor.borrow_mut().override_watchdog_usec(usec);
        self.restart_watchdog()
    }

    fn stop_watchdog(&self) {
        if self.rd.armd_watchdog() {
            let events = self.comm.um().events();
            events.del_source(self.rd.watchdog()).unwrap();
        }
    }

    fn kill_control_process(&self) {
        if let Some(pid) = self.pid.control() {
            if let Err(e) = process::kill_and_cont(pid, Signal::SIGKILL) {
                self.log(
                    Level::Warn,
                    &format!("failed to kill control process {}, error: {}", pid, e),
                )
            }
        }
    }

    fn log(&self, level: Level, msg: &str) {
        self.comm.log(level, msg);
    }

    pub(self) fn coldplug_timeout(&self) -> u64 {
        match self.state() {
            ServiceState::Condition
            | ServiceState::StartPre
            | ServiceState::Start
            | ServiceState::StartPost
            | ServiceState::Reload => self.config.config_data().borrow().Service.TimeoutStartSec,

            ServiceState::Running => 0, // todo => TimeoutMaxSec,

            ServiceState::Stop
            | ServiceState::StopSigterm
            | ServiceState::StopSigkill
            | ServiceState::StopPost
            | ServiceState::FinalSigterm
            | ServiceState::FinalSigkill => {
                self.config.config_data().borrow().Service.TimeoutStopSec
            }

            ServiceState::StopWatchdog | ServiceState::FinalWatchdog => {
                // todo => TimeoutAbortSec ? TimeoutAbortSec : TimeoutStopSec,
                self.config.config_data().borrow().Service.TimeoutStopSec
            }

            ServiceState::AutoRestart => self.config.config_data().borrow().Service.RestartSec,

            ServiceState::Cleaning => todo!(), // TimeoutCleanSec,

            _ => u64::MAX,
        }
    }

    pub fn reset_failed(&self) {
        if self.state() == ServiceState::Failed {
            self.set_state(ServiceState::Dead);
        }
        self.set_result(ServiceResult::Success);
        self.set_reload_result(ServiceResult::Success);
        let unit = match self.comm.owner() {
            None => {
                log::warn!("Failed to get the unit, thus we can't reset the StartLimit");
                return;
            }
            Some(v) => v,
        };
        unit.reset_start_limit();
    }

    pub(super) fn set_socket_fd(&self, fd: i32) {
        self.spawn.set_socket_fd(fd)
    }

    pub(super) fn release_socket_fd(&self, fd: i32) {
        self.spawn.release_socket_fd(fd)
    }
}

impl ServiceMng {
    pub(super) fn sigchld_event(&self, wait_status: WaitStatus) {
        self.do_sigchld_event(wait_status);
        self.db_update();
    }

    fn sigchld_result(&self, wait_status: WaitStatus) -> ServiceResult {
        match wait_status {
            WaitStatus::Exited(_, status) => {
                if status == 0 {
                    ServiceResult::Success
                } else {
                    ServiceResult::FailureExitCode
                }
            }
            WaitStatus::Signaled(pid, sig, core_dump) => {
                // long running service for not oneshot service, or service running in main pid, or current running service is Start.
                // the following signals always use to indicate normal exit.
                let is_daemon = self.config.service_type() != ServiceType::Oneshot
                    || self.pid.control() != Some(pid)
                    || *self.control_cmd_type.borrow() == Some(ServiceCommand::Start);
                if core_dump {
                    ServiceResult::FailureCoreDump
                } else if is_daemon
                    && matches!(
                        sig,
                        Signal::SIGHUP | Signal::SIGINT | Signal::SIGTERM | Signal::SIGPIPE
                    )
                {
                    ServiceResult::Success
                } else {
                    ServiceResult::FailureSignal
                }
            }
            _ => unreachable!(),
        }
    }

    fn do_sigchld_event(&self, wait_status: WaitStatus) {
        log::debug!("ServiceUnit sigchld exit wait status: {:?}", wait_status);
        log::debug!(
            "main_pid: {:?}, control_pid: {:?}, state: {:?}",
            self.pid.main(),
            self.pid.control(),
            self.state()
        );

        // none has been filter after waitpid, unwrap is safe here
        let pid = wait_status.pid().unwrap();
        let mut res = self.sigchld_result(wait_status);

        if self.pid.main() == Some(pid) {
            // for main pid updated by the process before its exited, updated the main pid.
            if let Ok(v) = self.load_pid_file() {
                if v {
                    return;
                }
            }

            self.pid.reset_main();
            self.rd.set_wait_status(wait_status);
            let exec_flag = self.current_main_command.borrow().get_exec_flag();

            if exec_flag.contains(ExecFlag::EXEC_COMMAND_IGNORE_FAILURE) {
                self.set_result(ServiceResult::Success);
                res = ServiceResult::Success;
            }

            if self.result() == ServiceResult::Success {
                self.set_result(res);
            }

            if !self.main_command.borrow().is_empty()
                && res == ServiceResult::Success
                && self.config.service_type() == ServiceType::Oneshot
            {
                self.run_next_main();
            } else {
                self.main_command.borrow_mut().clear();

                match self.state() {
                    ServiceState::Start if self.config.service_type() == ServiceType::Oneshot => {
                        if res == ServiceResult::Success {
                            self.enter_start_post();
                        } else {
                            self.enter_signal(ServiceState::StopSigterm, res);
                        }
                    }
                    ServiceState::Start if self.config.service_type() == ServiceType::Notify => {
                        if res != ServiceResult::Success {
                            self.enter_signal(ServiceState::StopSigterm, res);
                        } else {
                            self.enter_signal(
                                ServiceState::StopSigterm,
                                ServiceResult::FailureProtocol,
                            );
                        }
                    }
                    ServiceState::Start => {
                        self.enter_running(res);
                    }

                    ServiceState::Running => self.enter_running(res),

                    ServiceState::StartPost | ServiceState::Reload | ServiceState::Stop => {
                        if !self.pid.control_pid_avail() {
                            self.enter_stop(res);
                        }
                    }

                    ServiceState::StopWatchdog
                    | ServiceState::StopSigkill
                    | ServiceState::StopSigterm => {
                        if !self.pid.control_pid_avail() {
                            self.enter_stop_post(res);
                        }
                    }
                    ServiceState::StopPost => {
                        if !self.pid.control_pid_avail() {
                            self.enter_signal(ServiceState::FinalSigterm, res);
                        }
                    }
                    ServiceState::FinalSigterm
                    | ServiceState::FinalSigkill
                    | ServiceState::FinalWatchdog => {
                        if !self.pid.control_pid_avail() {
                            self.enter_dead(res, true);
                        }
                    }
                    _ => {
                        unreachable!(
                            "{}",
                            format!(
                                "current state is: {}, main pid exit at wrong state",
                                self.state()
                            )
                        );
                    }
                }
            }
        } else if self.pid.control() == Some(pid) {
            self.pid.reset_control();

            if self
                .current_control_command
                .borrow()
                .get_exec_flag()
                .contains(ExecFlag::EXEC_COMMAND_IGNORE_FAILURE)
            {
                res = ServiceResult::Success;
            }

            if !self.control_command.borrow().is_empty() && res == ServiceResult::Success {
                self.run_next_control();
                return;
            }

            self.control_command.borrow_mut().clear();
            self.set_cmd_type(None);
            match self.state() {
                ServiceState::Condition => {
                    if res == ServiceResult::Success {
                        self.enter_prestart();
                    } else {
                        self.enter_signal(ServiceState::StopSigterm, res);
                    }
                }
                ServiceState::StartPre => {
                    if res == ServiceResult::Success {
                        self.enter_start();
                    } else {
                        self.enter_signal(ServiceState::StopSigterm, res);
                    }
                }
                ServiceState::Start => {
                    if self.config.service_type() != ServiceType::Forking {
                        return;
                    }
                    // only forking type will be in Start state with the control pid exit.
                    log::debug!("in sigchild, forking type control pid exit");
                    if res != ServiceResult::Success {
                        self.enter_signal(ServiceState::StopSigterm, res);
                        return;
                    }

                    if self.config.pid_file().is_some() {
                        // will load the pid_file after the forking pid exist.
                        let start_post_exist = if self
                            .config
                            .get_exec_cmds(ServiceCommand::StartPost)
                            .is_some()
                        {
                            !self
                                .config
                                .get_exec_cmds(ServiceCommand::StartPost)
                                .unwrap()
                                .is_empty()
                        } else {
                            false
                        };

                        let loaded = self.load_pid_file();
                        log::debug!("service in Start state, load pid file result: {:?}", loaded);
                        if loaded.is_err() && !start_post_exist {
                            match self.demand_pid_file() {
                                Ok(_) => {
                                    if !self.cgroup_good() {
                                        self.enter_signal(
                                            ServiceState::StopSigterm,
                                            ServiceResult::FailureProtocol,
                                        );
                                    }
                                }
                                Err(_e) => {
                                    log::error!("demand pid file failed: {:?}", _e);
                                    self.enter_signal(
                                        ServiceState::StopSigterm,
                                        ServiceResult::FailureProtocol,
                                    );
                                }
                            }
                            return;
                        }

                        self.enter_start_post();
                    } else {
                        self.guess_main_pid();
                    }

                    self.enter_start_post();
                }
                ServiceState::StartPost => {
                    if res != ServiceResult::Success {
                        self.enter_signal(ServiceState::StopSigterm, res);
                    }

                    if self.config.pid_file().is_some() {
                        let loaded = self.load_pid_file();
                        if loaded.is_err() {
                            match self.demand_pid_file() {
                                Ok(_) => {
                                    if !self.cgroup_good() {
                                        self.enter_stop(ServiceResult::FailureProtocol);
                                    }
                                }
                                Err(_) => {
                                    self.enter_signal(
                                        ServiceState::StopSigterm,
                                        ServiceResult::FailureProtocol,
                                    );
                                }
                            }
                            return;
                        }
                    } else {
                        self.guess_main_pid();
                    }
                    self.enter_running(ServiceResult::Success);
                }
                ServiceState::Reload => {
                    self.set_reload_result(res);
                    self.enter_running(res);
                }
                ServiceState::Stop => {
                    self.enter_signal(ServiceState::StopSigterm, res);
                }
                ServiceState::StopSigterm
                | ServiceState::StopSigkill
                | ServiceState::StopWatchdog => {
                    if !self.pid.main_pid_avail() {
                        self.enter_stop_post(res);
                    }
                }
                ServiceState::StopPost => {
                    if !self.pid.main_pid_avail() {
                        self.enter_signal(ServiceState::FinalSigterm, res);
                    }
                }
                ServiceState::FinalSigterm
                | ServiceState::FinalSigkill
                | ServiceState::FinalWatchdog => {
                    if !self.pid.main_pid_avail() {
                        self.enter_dead(res, true);
                    }
                }
                _ => {
                    unreachable!(
                        "{}",
                        format!(
                            "current state is: {}, control process exit at wrong time",
                            self.state()
                        )
                    )
                }
            }
        }
    }
}

impl ServiceMng {
    fn notify_message_authorized(&self, pid: Pid) -> bool {
        let notify_access = match self.config.config_data().borrow().Service.NotifyAccess {
            None => NotifyAccess::None,
            Some(v) => v,
        };

        if notify_access == NotifyAccess::None {
            log::warn!(
                "Got notification message from {:?}, but NotifyAccess is configured to none.",
                pid
            );
            return false;
        }

        let main_pid = match self.pid.main() {
            None => {
                log::warn!("Couldn't determine main pid.");
                Pid::from_raw(0)
            }
            Some(v) => v,
        };

        if notify_access == NotifyAccess::Main && pid != main_pid {
            if main_pid.as_raw() == 0 {
                log::warn!(
                    "Got notification message from {:?}, but main pid is currently not known.",
                    pid
                );
            } else {
                log::warn!("Got notification message from {:?}, but only message from main {:?} is accept.", pid, main_pid);
            }
            return false;
        }

        let control_pid = match self.pid.control() {
            None => {
                log::warn!("Couldn't determine control pid.");
                Pid::from_raw(0)
            }
            Some(v) => v,
        };

        if notify_access == NotifyAccess::Exec && pid != main_pid && pid != control_pid {
            if main_pid.as_raw() != 0 && control_pid.as_raw() != 0 {
                log::warn!("Got notification message from {:?}, but only message from main {:?} or control {:?} is accept.", pid, main_pid, control_pid);
            } else if main_pid.as_raw() != 0 {
                log::warn!(
                    "Got notification message from {:?}, but message is not from main {:?}.",
                    pid,
                    main_pid
                );
            } else if control_pid.as_raw() != 0 {
                log::warn!(
                    "Got notification message from {:?}, but message is not from control {:?}.",
                    pid,
                    control_pid
                );
            } else {
                log::warn!("Got notification message from {:?}, but main pid and control pid are currently not known.", pid);
            }
            return false;
        }

        true
    }

    pub(super) fn notify_message(
        &self,
        ucred: &UnixCredentials,
        messages: &HashMap<&str, &str>,
        _fds: Vec<i32>,
    ) -> Result<()> {
        let ret = self.do_notify_message(ucred, messages, _fds);
        self.db_update();
        ret
    }

    fn do_notify_message(
        &self,
        ucred: &UnixCredentials,
        messages: &HashMap<&str, &str>,
        _fds: Vec<i32>,
    ) -> Result<()> {
        if !self.notify_message_authorized(Pid::from_raw(ucred.pid())) {
            return Ok(());
        }

        if let Some(&pidr) = messages.get("MAINPID") {
            if IN_SET!(
                self.state(),
                ServiceState::Start,
                ServiceState::StartPost,
                ServiceState::Running
            ) {
                let pid = pidr.parse::<i32>()?;
                let main_pid = Pid::from_raw(pid);
                if Some(main_pid) != self.pid.main() {
                    let valid = self.valid_main_pid(main_pid)?;

                    if ucred.pid() == 0 || valid {
                        let _ = self.pid.set_main(main_pid);
                        self.comm
                            .um()
                            .child_watch_pid(&self.comm.get_owner_id(), main_pid);
                    }
                }
            }
        };

        for (&key, &value) in messages {
            if key == "READY" && value == "1" {
                log::debug!("service plugin get READY=1");
                self.rd.set_notify_state(NotifyState::Ready);
                if self.config.service_type() == ServiceType::Notify
                    && self.state() == ServiceState::Start
                {
                    self.enter_start_post();
                }
            }

            if key == "STOPPING" && value == "1" {
                self.rd.set_notify_state(NotifyState::Stopping);
                if self.state() == ServiceState::Running {
                    self.enter_stop_by_notify();
                }
            }

            if key == "ERRNO" {
                let err = value.parse::<i32>();
                if err.is_err() {
                    log::warn!("parse ERRNO failed in received messages");
                    continue;
                }

                self.rd.set_errno(err.unwrap());
            }

            if key == "WATCHDOG" {
                if value == "1" {
                    self.restart_watchdog();
                } else if value == "trigger" {
                    self.force_watchdog();
                } else {
                    log::warn!(
                        "{} send WATCHDOG= field is invalid, ignoring.",
                        self.comm.owner().unwrap().id()
                    );
                }
            }
            if key == "WATCHDOG_USEC" {
                let watchdog_override_usec = value.parse::<u64>();
                if let Ok(v) = watchdog_override_usec {
                    self.override_watchdog_usec(v);
                    continue;
                }
                log::warn!("failed to parse notify message of WATCGDOG_USEC item");
            }
        }

        Ok(())
    }
}

impl ServiceState {
    fn to_unit_active_state(self) -> UnitActiveState {
        match self {
            ServiceState::Dead => UnitActiveState::InActive,
            ServiceState::Condition
            | ServiceState::StartPre
            | ServiceState::Start
            | ServiceState::StartPost
            | ServiceState::AutoRestart => UnitActiveState::Activating,
            ServiceState::Running | ServiceState::Exited => UnitActiveState::Active,
            ServiceState::Reload => UnitActiveState::Reloading,
            ServiceState::Stop
            | ServiceState::StopWatchdog
            | ServiceState::StopPost
            | ServiceState::StopSigterm
            | ServiceState::StopSigkill
            | ServiceState::FinalSigterm
            | ServiceState::FinalSigkill
            | ServiceState::FinalWatchdog => UnitActiveState::DeActivating,
            ServiceState::Failed => UnitActiveState::Failed,
            ServiceState::Cleaning => UnitActiveState::Maintenance,
        }
    }

    fn to_unit_active_state_idle(self) -> UnitActiveState {
        match self {
            ServiceState::Dead => UnitActiveState::InActive,
            ServiceState::Condition
            | ServiceState::StartPre
            | ServiceState::Start
            | ServiceState::StartPost
            | ServiceState::Running
            | ServiceState::Exited => UnitActiveState::Active,
            ServiceState::Reload => UnitActiveState::Reloading,
            ServiceState::Stop
            | ServiceState::StopWatchdog
            | ServiceState::StopPost
            | ServiceState::StopSigterm
            | ServiceState::StopSigkill
            | ServiceState::FinalSigterm
            | ServiceState::FinalSigkill
            | ServiceState::FinalWatchdog => UnitActiveState::DeActivating,
            ServiceState::Failed => UnitActiveState::Failed,
            ServiceState::Cleaning => UnitActiveState::Maintenance,
            ServiceState::AutoRestart => UnitActiveState::Activating,
        }
    }

    fn to_kill_operation(self) -> KillOperation {
        match self {
            ServiceState::StopWatchdog => KillOperation::KillWatchdog,
            ServiceState::StopSigterm | ServiceState::FinalSigterm => KillOperation::KillTerminate,
            ServiceState::StopSigkill | ServiceState::FinalSigkill => KillOperation::KillKill,
            _ => KillOperation::KillInvalid,
        }
    }
}

fn service_state_to_unit_state(service_type: ServiceType, state: ServiceState) -> UnitActiveState {
    if service_type == ServiceType::Idle {
        return state.to_unit_active_state_idle();
    }

    state.to_unit_active_state()
}

pub(super) struct RunningData {
    comm: Rc<ServiceUnitComm>,
    mng: RefCell<Weak<ServiceMng>>,
    data: RefCell<Rtdata>,
}

impl RunningData {
    pub(super) fn new(commr: &Rc<ServiceUnitComm>) -> Self {
        RunningData {
            comm: Rc::clone(commr),
            mng: RefCell::new(Weak::new()),
            data: RefCell::new(Rtdata::new()),
        }
    }

    pub(super) fn attach_mng(&self, mng: Rc<ServiceMng>) {
        *self.mng.borrow_mut() = Rc::downgrade(&mng);
    }

    pub(self) fn attach_inotify(&self, path_inotify: Rc<PathInotify>) {
        path_inotify.attach(self.mng.borrow_mut().clone());
        self.data.borrow_mut().attach_inotify(path_inotify);
    }

    pub(self) fn path_inotify(&self) -> Rc<PathInotify> {
        self.data.borrow().path_inotify()
    }

    pub(self) fn set_errno(&self, errno: i32) {
        self.data.borrow_mut().set_errno(errno);
    }

    pub(self) fn set_notify_state(&self, notify_state: NotifyState) {
        self.data.borrow_mut().set_notify_state(notify_state);
    }

    pub(self) fn notify_state(&self) -> NotifyState {
        self.data.borrow().notify_state()
    }

    pub(self) fn set_forbid_restart(&self, forbid_restart: bool) {
        self.data.borrow_mut().set_forbid_restart(forbid_restart);
    }

    pub(self) fn forbid_restart(&self) -> bool {
        self.data.borrow().forbid_restart()
    }

    pub(self) fn set_reset_restart(&self, reset: bool) {
        self.data.borrow_mut().set_reset_restart(reset);
    }

    pub(self) fn reset_restart(&self) -> bool {
        self.data.borrow().reset_restart()
    }

    pub(self) fn set_will_auto_restart(&self, will_auto_restart: bool) {
        self.data
            .borrow_mut()
            .set_will_auto_restart(will_auto_restart);
    }

    pub(self) fn will_auto_restart(&self) -> bool {
        self.data.borrow().will_auto_restart()
    }

    pub(self) fn will_restart(&self) -> bool {
        if self.data.borrow().will_auto_restart() {
            return true;
        }
        if self.mng.borrow().upgrade().unwrap().state() == ServiceState::AutoRestart {
            return true;
        }
        let u = match self.comm.owner() {
            None => return false,
            Some(v) => v,
        };
        self.comm.um().has_start_job(&u.id())
    }

    pub(self) fn attach_timer(&self, timer: Rc<ServiceTimer>) {
        timer.attach_mng(self.mng.borrow_mut().clone());
        self.data.borrow_mut().attach_timer(timer)
    }

    pub(self) fn timer(&self) -> Rc<ServiceTimer> {
        self.data.borrow().timer()
    }

    pub(self) fn armd_timer(&self) -> bool {
        self.data.borrow().armd_timer()
    }

    pub(super) fn enable_timer(&self, usec: u64) -> Result<i32> {
        let events = self.comm.um().events();
        /* usec == 0 is allowed here, see: https://gitee.com/openeuler/sysmaster/pulls/518 */
        if usec == u64::MAX {
            log::debug!("Timer is configured to u64::Max, won't enable.");
            // which means not enable the service timer, so delete the previous timer
            if self.armd_timer() {
                let timer = self.timer();
                events.del_source(timer)?;
            }
            return Ok(0);
        }
        log::debug!("Enable a timer: {}us", usec);
        if self.armd_timer() {
            let timer = self.timer();
            events.del_source(timer.clone())?;

            timer.set_time(usec);
            events.add_source(timer.clone())?;
            events.set_enabled(timer, EventState::OneShot)?;
            return Ok(0);
        }

        let timer = Rc::new(ServiceTimer::new(usec));
        self.attach_timer(timer.clone());

        events.add_source(timer.clone())?;
        events.set_enabled(timer, EventState::OneShot)?;

        Ok(0)
    }

    pub(self) fn delete_timer(&self) {
        if !self.armd_timer() {
            return;
        }

        let events = self.comm.um().events();
        let timer = self.timer();
        events.set_enabled(timer.clone(), EventState::Off).unwrap();
        events.del_source(timer).unwrap();
    }

    pub(super) fn add_restarts(&self) {
        self.data.borrow_mut().add_restarts();
    }

    pub(super) fn clear_restarts(&self) {
        self.data.borrow_mut().clear_restarts();
    }

    pub(super) fn set_restarts(&self, restarts: u32) {
        self.data.borrow_mut().set_restarts(restarts);
    }

    pub(self) fn restarts(&self) -> u32 {
        self.data.borrow().restarts()
    }

    pub(super) fn set_wait_status(&self, wait_status: WaitStatus) {
        self.data.borrow_mut().set_wait_status(wait_status);
    }

    pub(self) fn wait_status(&self) -> WaitStatus {
        self.data.borrow().wait_status()
    }

    pub(self) fn attach_watchdog(&self, watchdog: Rc<ServiceMonitorData>) {
        watchdog.attach_mng(self.mng.borrow_mut().clone());
        self.data.borrow_mut().attach_watchdog(watchdog);
    }

    pub(self) fn watchdog(&self) -> Rc<ServiceMonitorData> {
        self.data.borrow().watchdog()
    }

    pub(self) fn armd_watchdog(&self) -> bool {
        self.data.borrow().armd_watchdog()
    }
}

struct Rtdata {
    errno: i32,
    notify_state: NotifyState,
    path_inotify: Option<Rc<PathInotify>>,

    forbid_restart: bool,
    reset_restarts: bool,
    will_auto_restart: bool,
    restarts: u32,
    timer: Option<Rc<ServiceTimer>>,

    exec_status: WaitStatus,

    watchdog: Option<Rc<ServiceMonitorData>>,
}

impl Rtdata {
    pub(self) fn new() -> Self {
        Rtdata {
            errno: 0,
            notify_state: NotifyState::Unknown,
            path_inotify: None,

            forbid_restart: false,
            reset_restarts: false,
            will_auto_restart: false,
            restarts: 0,
            timer: None,
            exec_status: WaitStatus::StillAlive,
            watchdog: None,
        }
    }

    pub(self) fn set_notify_state(&mut self, notify_state: NotifyState) {
        self.notify_state = notify_state;
    }

    pub(self) fn notify_state(&self) -> NotifyState {
        self.notify_state
    }

    pub(self) fn set_errno(&mut self, errno: i32) {
        self.errno = errno;
    }

    #[allow(dead_code)]
    pub(self) fn errno(&mut self) -> i32 {
        self.errno
    }

    pub(self) fn attach_inotify(&mut self, path_inotify: Rc<PathInotify>) {
        self.path_inotify = Some(path_inotify)
    }

    pub(self) fn path_inotify(&self) -> Rc<PathInotify> {
        self.path_inotify.as_ref().unwrap().clone()
    }

    pub(self) fn set_forbid_restart(&mut self, forbid_restart: bool) {
        self.forbid_restart = forbid_restart
    }

    pub(self) fn forbid_restart(&self) -> bool {
        self.forbid_restart
    }

    pub(self) fn set_reset_restart(&mut self, reset: bool) {
        self.reset_restarts = reset
    }

    pub(self) fn reset_restart(&self) -> bool {
        self.reset_restarts
    }

    pub(self) fn set_will_auto_restart(&mut self, will_auto_restart: bool) {
        self.will_auto_restart = will_auto_restart
    }

    pub(self) fn will_auto_restart(&self) -> bool {
        self.will_auto_restart
    }

    pub(self) fn attach_timer(&mut self, timer: Rc<ServiceTimer>) {
        self.timer = Some(timer)
    }

    pub(self) fn timer(&self) -> Rc<ServiceTimer> {
        self.timer.as_ref().unwrap().clone()
    }

    pub(self) fn armd_timer(&self) -> bool {
        self.timer.is_some()
    }

    pub(self) fn add_restarts(&mut self) {
        self.restarts += 1;
    }

    pub(self) fn clear_restarts(&mut self) {
        self.restarts = 0;
    }

    pub(super) fn set_restarts(&mut self, restarts: u32) {
        self.restarts = restarts;
    }

    pub(self) fn restarts(&self) -> u32 {
        self.restarts
    }

    pub(super) fn set_wait_status(&mut self, wait_status: WaitStatus) {
        self.exec_status = wait_status;
    }

    pub(self) fn wait_status(&self) -> WaitStatus {
        self.exec_status
    }

    pub(self) fn attach_watchdog(&mut self, watchdog: Rc<ServiceMonitorData>) {
        self.watchdog = Some(watchdog)
    }

    pub(self) fn watchdog(&self) -> Rc<ServiceMonitorData> {
        self.watchdog.as_ref().unwrap().clone()
    }

    pub(self) fn armd_watchdog(&self) -> bool {
        self.watchdog.is_some()
    }
}

struct PathInotify {
    spec: PathSpec,
    mng: RefCell<Weak<ServiceMng>>,
}

impl fmt::Display for PathInotify {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.spec)
    }
}

impl PathInotify {
    fn new(path: PathBuf, p_type: PathType) -> Self {
        PathInotify {
            spec: PathSpec::new(path, p_type),
            mng: RefCell::new(Weak::new()),
        }
    }

    pub(self) fn attach(&self, mng: Weak<ServiceMng>) {
        log::debug!("attach service mng to path inotify");
        *self.mng.borrow_mut() = mng;
    }

    fn watch(&self) -> Result<()> {
        self.spec.watch()
    }

    fn unwatch(&self) {
        self.spec.unwatch()
    }

    pub(self) fn mng(&self) -> Rc<ServiceMng> {
        self.mng.borrow().clone().upgrade().unwrap()
    }

    fn do_dispatch(&self) -> i32 {
        log::debug!("dispatch inotify pid file: {:?}", self.spec.path());
        match self.spec.read_fd_event() {
            Ok(_) => {
                if let Ok(_v) = self.mng().retry_pid_file() {
                    return 0;
                }

                if let Ok(_v) = self.mng().watch_pid_file() {
                    return 0;
                }
            }
            Err(e) => {
                log::error!("in inotify dispatch, read event error: {}", e);
                return -1;
            }
        }

        self.mng().unwatch_pid_file();
        self.mng()
            .enter_signal(ServiceState::StopSigterm, ServiceResult::FailureResources);
        0
    }
}

impl Source for PathInotify {
    fn fd(&self) -> RawFd {
        self.spec.inotify_fd()
    }

    fn event_type(&self) -> EventType {
        EventType::Io
    }

    fn epoll_event(&self) -> u32 {
        (libc::EPOLLIN) as u32
    }

    fn priority(&self) -> i8 {
        0i8
    }

    fn dispatch(&self, _: &Events) -> i32 {
        let ret = self.do_dispatch();
        self.mng().db_update();
        ret
    }

    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }
}

pub(super) struct ServiceTimer {
    time: RefCell<u64>,
    mng: RefCell<Weak<ServiceMng>>,
}

impl ServiceTimer {
    pub fn new(usec: u64) -> Self {
        ServiceTimer {
            time: RefCell::new(usec),
            mng: RefCell::new(Weak::new()),
        }
    }

    pub(super) fn attach_mng(&self, mng: Weak<ServiceMng>) {
        *self.mng.borrow_mut() = mng;
    }

    pub(super) fn set_time(&self, usec: u64) {
        *self.time.borrow_mut() = usec
    }

    pub(self) fn mng(&self) -> Rc<ServiceMng> {
        self.mng.borrow().clone().upgrade().unwrap()
    }

    fn do_dispatch(&self) -> i32 {
        self.mng().log(Level::Debug, "dispatch service timer");

        match self.mng().state() {
            ServiceState::Condition
            | ServiceState::StartPre
            | ServiceState::Start
            | ServiceState::StartPost => {
                self.mng().log(
                    Level::Warn,
                    &format!(
                        "{} operation time out, enter StopSigterm operation",
                        self.mng().state()
                    ),
                );
                self.mng()
                    .enter_signal(ServiceState::StopSigterm, ServiceResult::FailureTimeout);
            }
            ServiceState::Running => {
                self.mng().log(
                    Level::Warn,
                    "Running operation time out, enter Stop operation",
                );
                self.mng().enter_stop(ServiceResult::FailureTimeout);
            }
            ServiceState::Reload => {
                self.mng().log(
                    Level::Warn,
                    "Reload operation time out, kill control process and enter running",
                );
                self.mng().kill_control_process();
                self.mng().set_reload_result(ServiceResult::FailureTimeout);

                self.mng().enter_running(ServiceResult::Success);
            }
            ServiceState::Stop => {
                self.mng()
                    .log(Level::Warn, "Stop operation time out, enter StopSigterm");

                self.mng()
                    .enter_signal(ServiceState::StopSigterm, ServiceResult::FailureTimeout);
            }
            ServiceState::StopWatchdog => {
                self.mng().log(
                    Level::Warn,
                    "StopWatchdog operation time out, enter StopPost",
                );
                self.mng().enter_stop_post(ServiceResult::FailureTimeout);
            }
            ServiceState::StopPost => {
                self.mng().log(
                    Level::Warn,
                    "StopPost operation time out, enter FinalSigterm",
                );

                self.mng()
                    .enter_signal(ServiceState::FinalSigterm, ServiceResult::FailureTimeout);
            }
            ServiceState::StopSigterm | ServiceState::StopSigkill => {
                self.mng().log(
                    Level::Warn,
                    "StopSigterm or StopSigkill operation time out, enter StopPost",
                );
                self.mng().enter_stop_post(ServiceResult::FailureTimeout)
            }
            ServiceState::FinalWatchdog | ServiceState::FinalSigterm => {
                self.mng().log(
                    Level::Warn,
                    "FinalWatchdog or FinalSigterm operation time out, enter Dead",
                );
                self.mng().enter_dead(ServiceResult::FailureTimeout, false)
            }
            ServiceState::FinalSigkill => {
                self.mng()
                    .log(Level::Warn, "FinalSigkill operation time out, enter Dead");
                self.mng().enter_dead(ServiceResult::FailureTimeout, true)
            }
            ServiceState::AutoRestart => {
                self.mng()
                    .log(Level::Warn, "AutoStart operation time out, enter Restart");
                self.mng().enter_restart();
            }
            ServiceState::Cleaning => self
                .mng()
                .enter_signal(ServiceState::FinalSigkill, ServiceResult::FailureTimeout),
            _ => {
                unreachable!()
            }
        }
        0
    }
}

impl Source for ServiceTimer {
    fn fd(&self) -> RawFd {
        0
    }

    fn event_type(&self) -> EventType {
        EventType::TimerMonotonic
    }

    fn epoll_event(&self) -> u32 {
        (libc::EPOLLIN) as u32
    }

    fn time_relative(&self) -> u64 {
        *self.time.borrow()
    }

    fn dispatch(&self, _: &Events) -> i32 {
        self.do_dispatch()
    }

    fn priority(&self) -> i8 {
        0
    }

    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }
}

struct ServiceMonitorData {
    mng: RefCell<Weak<ServiceMng>>,
    // owned objects
    time: RefCell<u64>, /* usec */
}

// the declaration "pub(self)" is for identification only.
#[allow(dead_code)]
impl ServiceMonitorData {
    pub(super) fn new(usec: u64) -> ServiceMonitorData {
        ServiceMonitorData {
            mng: RefCell::new(Weak::new()),
            time: RefCell::new(usec),
        }
    }

    pub(self) fn attach_mng(&self, mng: Weak<ServiceMng>) {
        log::debug!("attach service mng to path watchdog");
        *self.mng.borrow_mut() = mng;
    }

    pub(self) fn mng(&self) -> Rc<ServiceMng> {
        self.mng.borrow().clone().upgrade().unwrap()
    }

    pub(super) fn set_time(&self, usec: u64) {
        *self.time.borrow_mut() = usec
    }

    pub(super) fn time(&self) -> u64 {
        *self.time.borrow()
    }

    fn do_dispatch(&self) -> i32 {
        log::debug!(
            "dispatch service watchdog, watchdog timer is: {}",
            self.time()
        );

        self.mng()
            .enter_signal(ServiceState::StopWatchdog, ServiceResult::FailureWatchdog);
        0
    }
}

impl Source for ServiceMonitorData {
    fn fd(&self) -> RawFd {
        0
    }

    fn event_type(&self) -> EventType {
        EventType::TimerMonotonic
    }

    fn epoll_event(&self) -> u32 {
        (libc::EPOLLIN) as u32
    }

    fn time_relative(&self) -> u64 {
        *self.time.borrow()
    }

    fn dispatch(&self, _: &Events) -> i32 {
        self.do_dispatch()
    }

    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }

    fn priority(&self) -> i8 {
        100i8
    }
}

#[cfg(test)]
mod tests {
    use super::{RunningData, ServiceMng};
    use crate::{comm::ServiceUnitComm, config::ServiceConfig};
    use core::{exec::ExecContext, UmIf};
    use std::{collections::HashMap, rc::Rc};

    use libtests::get_project_root;

    struct UmIfD;
    impl UmIf for UmIfD {}

    fn create_mng() -> (Rc<ServiceMng>, Rc<RunningData>, Rc<ServiceConfig>) {
        let mut file_path = get_project_root().unwrap();
        file_path.push("tests/test_units/config.service");
        let paths = vec![file_path];

        let comm = Rc::new(ServiceUnitComm::new());
        comm.attach_um(Rc::new(UmIfD));

        let config = Rc::new(ServiceConfig::new(&comm));
        let context = Rc::new(ExecContext::new());

        let result = config.load(paths, "config.service", false);
        assert!(result.is_ok());

        let rt = Rc::new(RunningData::new(&comm));
        let mng = Rc::new(ServiceMng::new(&comm, &config, &rt, &context));
        rt.attach_mng(mng.clone());
        (mng, rt, config)
    }

    #[test]
    fn test_watchdog_on() {
        use crate::rentry::NotifyAccess;
        use nix::sys::socket::UnixCredentials;

        let (mng, rt, config) = create_mng();

        let ucred = UnixCredentials::new();
        let mut messages = HashMap::new();

        messages.insert("WATCHDOG", "1");
        let fds = vec![];
        mng.config.set_notify_access(NotifyAccess::All);
        assert!(mng.notify_message(&ucred, &messages, fds).is_ok());
        assert!(rt.armd_watchdog());
        assert_eq!(
            rt.watchdog().time(),
            config.config_data().borrow().Service.WatchdogSec
        );
    }

    #[test]
    fn test_watchdog_reset_usec() {
        use crate::rentry::NotifyAccess;
        use nix::sys::socket::UnixCredentials;

        let (mng, rt, config) = create_mng();

        let ucred = UnixCredentials::new();
        let mut messages = HashMap::new();
        messages.insert("WATCHDOG", "1");
        let fds = vec![];

        mng.config.set_notify_access(NotifyAccess::All);
        assert!(mng.notify_message(&ucred, &messages, fds).is_ok());
        assert!(rt.armd_watchdog());
        assert_eq!(
            rt.watchdog().time(),
            config.config_data().borrow().Service.WatchdogSec
        );

        messages.remove("WATCHDOG");
        messages.insert("WATCHDOG_USEC", "15");

        let fds2 = vec![];
        assert!(mng.notify_message(&ucred, &messages, fds2).is_ok());

        assert!(rt.armd_watchdog());
        assert_eq!(rt.watchdog().time(), 15);
    }
}
