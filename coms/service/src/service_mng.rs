use super::service_comm::ServiceUnitComm;
use super::service_config::ServiceConfig;
use super::service_pid::ServicePid;
use super::service_rentry::{
    NotifyState, ServiceCommand, ServiceResult, ServiceState, ServiceType,
};
use super::service_spawn::ServiceSpawn;
use libevent::{EventState, EventType, Events, Source};
use libutils::{fd_util, Error, IN_SET};
use libutils::{file_util, process_util};
use nix::errno::Errno;
use nix::libc;
use nix::sys::inotify::{AddWatchFlags, InitFlags, Inotify, WatchDescriptor};
use nix::sys::signal::Signal;
use nix::sys::socket::UnixCredentials;
use nix::unistd::Pid;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::os::unix::prelude::AsRawFd;
use std::path::Path;
use std::rc::Rc;
use std::{
    os::unix::prelude::{FromRawFd, RawFd},
    path::PathBuf,
    rc::Weak,
};
use sysmaster::reliability::ReStation;
use sysmaster::unit::{
    ExecCommand, ExecContext, ExecFlags, KillOperation, UnitActionError, UnitActiveState,
    UnitNotifyFlags,
};

pub(super) struct ServiceMng {
    // associated objects
    comm: Rc<ServiceUnitComm>,
    config: Rc<ServiceConfig>,

    // owned objects
    pid: Rc<ServicePid>,
    spawn: ServiceSpawn,
    state: RefCell<ServiceState>,
    result: RefCell<ServiceResult>,
    main_command: RefCell<Vec<ExecCommand>>,
    control_cmd_type: RefCell<Option<ServiceCommand>>,
    control_command: RefCell<Vec<ExecCommand>>,
    rd: Rc<RunningData>,
}

impl ReStation for ServiceMng {
    // input: do nothing

    // compensate: do nothing

    // data
    fn db_map(&self) {
        if let Some((
            state,
            result,
            m_pid,
            c_pid,
            main_cmd_len,
            control_cmd_type,
            control_cmd_len,
            notify_state,
        )) = self.comm.rentry_mng_get()
        {
            *self.state.borrow_mut() = state;
            *self.result.borrow_mut() = result;
            self.pid.update_main(m_pid);
            self.pid.update_control(c_pid);
            self.main_command_update(main_cmd_len);
            self.control_command_update(control_cmd_type, control_cmd_len);
            self.rd.set_notify_state(notify_state);
        }
    }

    fn db_insert(&self) {
        self.comm.rentry_mng_insert(
            self.state(),
            self.result(),
            self.pid.main(),
            self.pid.control(),
            self.main_command.borrow().len(),
            *self.control_cmd_type.borrow(),
            self.control_command.borrow().len(),
            self.rd.notify_state(),
        );
    }

    // reload: no external connections, no entry
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
            spawn: ServiceSpawn::new(commr, &_pid, configr, exec_ctx),
            state: RefCell::new(ServiceState::Dead),
            result: RefCell::new(ServiceResult::Success),
            main_command: RefCell::new(Vec::new()),
            control_cmd_type: RefCell::new(None),
            control_command: RefCell::new(Vec::new()),
            rd: rd.clone(),
        }
    }

    pub(super) fn start_check(&self) -> Result<bool, UnitActionError> {
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
            return Err(UnitActionError::UnitActionEAgain);
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
            self.enter_dead(ServiceResult::FailureStartLimitHit);
            return Err(UnitActionError::UnitActionECanceled);
        }

        Ok(false)
    }

    pub(super) fn start_action(&self) {
        self.set_result(ServiceResult::Success);
        self.enter_contion();
        self.db_update();
    }

    pub(super) fn stop_check(&self) -> Result<(), UnitActionError> {
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
        let starting_state = vec![
            ServiceState::Condition,
            ServiceState::StartPre,
            ServiceState::Start,
            ServiceState::StartPost,
            ServiceState::Reload,
            ServiceState::StopWatchdog,
        ];
        if starting_state.contains(&self.state()) {
            self.enter_signal(ServiceState::StopSigterm, ServiceResult::Success);
            self.db_update();
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
        log::debug!("enter running service condition command");
        self.control_command_fill(ServiceCommand::Condition);
        match self.control_command_pop() {
            Some(cmd) => {
                match self.spawn.start_service(&cmd, 0, ExecFlags::CONTROL) {
                    Ok(pid) => self.pid.set_control(pid),
                    Err(_e) => {
                        self.enter_dead(ServiceResult::FailureResources);
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
        log::debug!("enter running service prestart command");
        self.pid.unwatch_control();
        self.control_command_fill(ServiceCommand::StartPre);
        match self.control_command_pop() {
            Some(cmd) => {
                match self.spawn.start_service(&cmd, 0, ExecFlags::CONTROL) {
                    Ok(pid) => self.pid.set_control(pid),
                    Err(_e) => {
                        self.enter_dead(ServiceResult::FailureResources);
                        return;
                    }
                }
                self.set_state(ServiceState::StartPre);
            }
            None => self.enter_start(),
        }
    }

    fn enter_start(&self) {
        log::debug!("enter running service start command");
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
                log::error!("no start command is configured and service type is oneshot");
                self.enter_signal(ServiceState::StopSigterm, ServiceResult::FailureResources);
                return;
            }
            self.set_state(ServiceState::Start);
            self.enter_start_post();
            return;
        }

        let ret = self
            .spawn
            .start_service(&cmd.unwrap(), 0, ExecFlags::PASS_FDS);

        if ret.is_err() {
            log::error!(
                "failed to start service: unit Name{}",
                self.comm.get_owner_id()
            );
            self.enter_signal(ServiceState::StopSigterm, ServiceResult::FailureResources);
            return;
        }

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
                // for foring service type, we consider the process startup complete when the process exit;
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
        log::debug!("enter running service startpost command");
        self.pid.unwatch_control();
        self.control_command_fill(ServiceCommand::StartPost);
        match self.control_command_pop() {
            Some(cmd) => {
                match self.spawn.start_service(&cmd, 0, ExecFlags::CONTROL) {
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
            }
        } else if self.config.config_data().borrow().Service.RemainAfterExit {
            self.set_state(ServiceState::Exited);
        } else {
            self.enter_stop(sr);
        }
    }

    fn enter_stop(&self, res: ServiceResult) {
        log::debug!("enter running stop command");
        if self.result() == ServiceResult::Success {
            self.set_result(res);
        }

        self.control_command_fill(ServiceCommand::Stop);
        match self.control_command_pop() {
            Some(cmd) => {
                match self.spawn.start_service(&cmd, 0, ExecFlags::CONTROL) {
                    Ok(pid) => self.pid.set_control(pid),
                    Err(_e) => {
                        log::error!(
                            "Failed to run stop service: unit Name{}",
                            self.comm.get_owner_id()
                        );
                        self.enter_signal(
                            ServiceState::StopSigterm,
                            ServiceResult::FailureResources,
                        );
                        return;
                    }
                }
                self.set_state(ServiceState::Stop);
            }
            None => self.enter_signal(ServiceState::StopSigterm, ServiceResult::Success),
        }
    }

    fn enter_stop_by_notify(&self) {
        // todo
        // start a timer
        self.set_state(ServiceState::StopSigterm);
    }

    fn enter_stop_post(&self, res: ServiceResult) {
        log::debug!("runring stop post, service result: {:?}", res);
        if self.result() == ServiceResult::Success {
            self.set_result(res);
        }

        self.control_command_fill(ServiceCommand::StopPost);
        match self.control_command_pop() {
            Some(cmd) => {
                match self.spawn.start_service(&cmd, 0, ExecFlags::CONTROL) {
                    Ok(pid) => self.pid.set_control(pid),
                    Err(_e) => {
                        self.enter_signal(
                            ServiceState::FinalSigterm,
                            ServiceResult::FailureResources,
                        );
                        log::error!("Failed to run stop service: {}", self.comm.get_owner_id());
                        return;
                    }
                }
                self.set_state(ServiceState::StopPost);
            }
            None => self.enter_signal(ServiceState::FinalSigterm, ServiceResult::Success),
        }
    }

    fn enter_dead(&self, res: ServiceResult) {
        log::debug!("Running into dead state, res: {:?}", res);
        if self.result() == ServiceResult::Success {
            self.set_result(res);
        }

        let state = if self.result() == ServiceResult::Success {
            ServiceState::Dead
        } else {
            ServiceState::Failed
        };

        self.set_state(state);
    }

    fn enter_reload(&self) {
        log::debug!("running service reload command");
        self.control_command.borrow_mut().clear();
        self.pid.unwatch_control();
        self.control_command_fill(ServiceCommand::Reload);
        match self.control_command_pop() {
            Some(cmd) => {
                match self.spawn.start_service(&cmd, 0, ExecFlags::CONTROL) {
                    Ok(pid) => self.pid.set_control(pid),
                    Err(_e) => {
                        log::error!("failed to start service: {}", self.comm.get_owner_id());
                        self.enter_running(ServiceResult::Success);
                        return;
                    }
                }
                self.set_state(ServiceState::Reload);
            }
            None => self.enter_running(ServiceResult::Success),
        }
    }

    fn enter_signal(&self, state: ServiceState, res: ServiceResult) {
        log::debug!(
            "Sending signal of state: {:?}, service result: {:?}",
            state,
            res
        );

        if let Some(u) = self.comm.owner() {
            let op = state.to_kill_operation();
            self.comm
                .um()
                .child_watch_all_pids(&self.comm.get_owner_id());
            match u.kill_context(
                self.config.kill_context(),
                self.pid.main(),
                self.pid.control(),
                op,
            ) {
                Ok(_) => {}
                Err(_e) => {
                    if IN_SET!(
                        state,
                        ServiceState::StopWatchdog,
                        ServiceState::StopSigterm,
                        ServiceState::StopSigkill
                    ) {
                        return self.enter_stop_post(ServiceResult::FailureResources);
                    } else {
                        return self.enter_dead(ServiceResult::Success);
                    }
                }
            }
        }

        if vec![
            ServiceState::StopWatchdog,
            ServiceState::StopSigterm,
            ServiceState::StopSigkill,
        ]
        .contains(&state)
        {
            self.enter_stop_post(ServiceResult::Success);
        } else if vec![ServiceState::FinalWatchdog, ServiceState::FinalSigterm].contains(&state) {
            self.enter_signal(ServiceState::FinalSigkill, ServiceResult::Success);
        } else {
            self.enter_dead(ServiceResult::Success);
        }
    }

    fn set_state(&self, state: ServiceState) {
        let original_state = self.state();
        *self.state.borrow_mut() = state;

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
        }

        log::debug!(
            "unit: {}, original state: {:?}, change to: {:?}",
            self.comm.get_owner_id(),
            original_state,
            state
        );
        // todo!()
        // trigger the unit the dependency trigger_by

        let os = service_state_to_unit_state(self.config.service_type(), original_state);
        let ns = service_state_to_unit_state(self.config.service_type(), state);
        if let Some(u) = self.comm.owner() {
            u.notify(os, ns, UnitNotifyFlags::UNIT_NOTIFY_RELOAD_FAILURE)
        }
    }

    fn service_alive(&self) -> bool {
        if let Ok(v) = self.pid.main_alive() {
            return v;
        }

        self.cgroup_good()
    }

    fn run_next_control(&self) {
        log::debug!("runring next control command");
        if let Some(cmd) = self.control_command_pop() {
            match self.spawn.start_service(&cmd, 0, ExecFlags::CONTROL) {
                Ok(pid) => self.pid.set_control(pid),
                Err(_e) => {
                    log::error!("failed to start service: {}", self.comm.get_owner_id());
                }
            }
        }
    }

    fn run_next_main(&self) {
        if let Some(cmd) = self.main_command_pop() {
            match self.spawn.start_service(&cmd, 0, ExecFlags::PASS_FDS) {
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

    fn state(&self) -> ServiceState {
        *self.state.borrow()
    }

    fn set_result(&self, result: ServiceResult) {
        *self.result.borrow_mut() = result;
    }

    fn result(&self) -> ServiceResult {
        *self.result.borrow()
    }

    fn main_command_fill(&self) {
        let cmd_type = ServiceCommand::Start;
        if let Some(cmds) = self.config.get_exec_cmds(cmd_type) {
            *self.main_command.borrow_mut() = cmds
        }
    }

    fn main_command_pop(&self) -> Option<ExecCommand> {
        self.main_command.borrow_mut().pop()
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
            *self.control_command.borrow_mut() = cmds
        }
    }

    fn control_command_pop(&self) -> Option<ExecCommand> {
        self.control_command.borrow_mut().pop()
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

    // fn get_exec_cmds(&self, cmd_type: ServiceCommand) -> Vec<ExecCommand> {
    //     if let Some(cmds) = self.exec_commands.borrow_mut().get(&cmd_type) {
    //         cmds.as_slice()
    //     } else {
    //         Vec::new()
    //     }
    // }

    // fn insert_exec_cmds(&mut self, cmd_type: ServiceCommand, cmds: Vec<ExecCommand>) {
    //     self.exec_commands
    //         .borrow_mut()
    //         .insert(cmd_type, cmds.clone());
    // }

    fn load_pid_file(&self) -> Result<bool, Error> {
        let pid_file = self
            .config
            .config_data()
            .borrow()
            .Service
            .PIDFile
            .as_ref()
            .map(|s| s.to_string());
        if pid_file.is_none() {
            return Err(Error::Other {
                msg: "pid file is not configured",
            });
        }

        let file = &pid_file.unwrap();
        let pid_file_path = Path::new(file);
        if !pid_file_path.exists() || !pid_file_path.is_file() {
            return Err(Error::Other {
                msg: "pid file is not a file or not exist",
            });
        }

        let pid = match file_util::read_first_line(pid_file_path) {
            Ok(line) => line.trim().parse::<i32>(),
            Err(e) => return Err(Error::from(e)),
        };

        if pid.is_err() {
            log::debug!(
                "failed to parse pid from pid_file {:?}, err: {:?}",
                pid_file_path,
                pid
            );
            return Err(Error::Other {
                msg: "parsed the pid from pid file failed",
            });
        }

        let pid = Pid::from_raw(pid.unwrap());
        if self.pid.main().is_some() && self.pid.main().unwrap() == pid {
            return Ok(false);
        }

        self.valid_main_pid(pid)?;

        self.pid.unwatch_main();
        self.pid.set_main(pid).map_err(|_e| Error::Other {
            msg: "invalid main pid",
        })?;

        self.comm
            .um()
            .child_watch_pid(&self.comm.get_owner_id(), pid);

        Ok(true)
    }

    fn valid_main_pid(&self, pid: Pid) -> Result<bool, Error> {
        if pid == nix::unistd::getpid() {
            return Err(Error::Other {
                msg: "main pid is the sysmaster's pid",
            });
        }

        if self.pid.control().is_some() && self.pid.control().unwrap() == pid {
            return Err(Error::Other {
                msg: "main pid is the control process",
            });
        }

        if !process_util::alive(pid) {
            return Err(Error::Other {
                msg: "main pid is not alive",
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

    fn demand_pid_file(&self) -> Result<(), Error> {
        let pid_file_inotify = PathIntofy::new(PathBuf::from(
            self.config
                .config_data()
                .borrow()
                .Service
                .PIDFile
                .as_ref()
                .unwrap(),
        ));

        self.rd.attach_inotify(Rc::new(pid_file_inotify));

        self.watch_pid_file()
    }

    fn watch_pid_file(&self) -> Result<(), Error> {
        let pid_file_inotify = self.rd.path_inotify();
        log::debug!("watch pid file: {}", pid_file_inotify);
        match pid_file_inotify.add_watch_path() {
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
                    "failed to add watch for pid file {:?}, err: {}",
                    pid_file_inotify.path,
                    e
                );
                self.unwatch_pid_file();

                Err(e)
            }
        }
    }

    fn unwatch_pid_file(&self) {
        log::debug!("unwatch pid file {}", self.rd.path_inotify());
        self.rd.path_inotify().unwatch();
    }

    fn retry_pid_file(&self) -> Result<bool, Error> {
        log::debug!("retry loading pid file: {}", self.rd.path_inotify());
        self.load_pid_file()?;

        self.unwatch_pid_file();
        self.enter_running(ServiceResult::Success);

        Ok(true)
    }

    fn cgroup_good(&self) -> bool {
        if let Some(Ok(v)) = self
            .comm
            .owner()
            .map(|u| libcgroup::cg_is_empty_recursive(&u.cg_path()))
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
}

impl ServiceMng {
    pub(super) fn sigchld_event(&self, pid: Pid, code: i32, status: Signal) {
        self.do_sigchld_event(pid, code, status);
        self.db_update();
    }

    fn do_sigchld_event(&self, pid: Pid, code: i32, status: Signal) {
        log::debug!(
            "ServiceUnit sigchld exit, pid: {:?} code:{}, status:{}",
            pid,
            code,
            status
        );
        log::debug!(
            "main_pid: {:?}, control_pid: {:?}, state: {:?}",
            self.pid.main(),
            self.pid.control(),
            self.state()
        );
        let res: ServiceResult;
        if code == 0 {
            res = ServiceResult::Success;
        } else if status != Signal::SIGCHLD {
            res = ServiceResult::FailureSignal;
        } else {
            res = ServiceResult::Success
        }

        if self.pid.main() == Some(pid) {
            // for main pid updated by the process before its exited, updated the main pid.
            if let Ok(v) = self.load_pid_file() {
                if v {
                    return;
                }
            }

            self.pid.reset_main();

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
                    ServiceState::Dead => todo!(),
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
                    ServiceState::StartPost | ServiceState::Reload => {
                        self.enter_stop(res);
                    }
                    ServiceState::Running => {
                        self.enter_running(res);
                    }
                    ServiceState::Stop => {}
                    ServiceState::StopWatchdog
                    | ServiceState::StopSigkill
                    | ServiceState::StopSigterm => {
                        self.enter_stop_post(res);
                    }
                    ServiceState::FinalSigterm | ServiceState::FinalSigkill => {
                        self.enter_dead(res);
                    }
                    _ => {}
                }
            }
        } else if self.pid.control() == Some(pid) {
            self.pid.reset_control();

            if !self.control_command.borrow().is_empty() && res == ServiceResult::Success {
                self.run_next_control();
                return;
            }

            self.control_command.borrow_mut().clear();
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

                    if self.config.config_data().borrow().Service.PIDFile.is_some() {
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
                    if self.config.config_data().borrow().Service.PIDFile.is_some() {
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
                ServiceState::Running => todo!(),
                ServiceState::Reload => {
                    self.enter_running(res);
                }
                ServiceState::Stop => {
                    self.enter_signal(ServiceState::StopSigterm, res);
                }
                ServiceState::StopSigterm
                | ServiceState::StopSigkill
                | ServiceState::StopWatchdog => {
                    self.enter_stop_post(res);
                }
                ServiceState::StopPost => {
                    self.enter_signal(ServiceState::FinalSigterm, res);
                }
                ServiceState::FinalSigterm | ServiceState::FinalSigkill => {
                    self.enter_dead(res);
                }
                _ => {}
            }
        }
    }
}

impl ServiceMng {
    pub(super) fn notify_message(
        &self,
        ucred: &UnixCredentials,
        messages: &HashMap<&str, &str>,
        _fds: Vec<i32>,
    ) -> Result<(), Error> {
        let ret = self.do_notify_message(ucred, messages, _fds);
        self.db_update();
        ret
    }

    fn do_notify_message(
        &self,
        ucred: &UnixCredentials,
        messages: &HashMap<&str, &str>,
        _fds: Vec<i32>,
    ) -> Result<(), Error> {
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
        }

        Ok(())
    }
}

impl ServiceState {
    fn to_unit_active_state(self) -> UnitActiveState {
        match self {
            ServiceState::Dead => UnitActiveState::UnitInActive,
            ServiceState::Condition
            | ServiceState::StartPre
            | ServiceState::Start
            | ServiceState::StartPost => UnitActiveState::UnitActivating,
            ServiceState::Running | ServiceState::Exited => UnitActiveState::UnitActive,
            ServiceState::Reload => UnitActiveState::UnitReloading,
            ServiceState::Stop
            | ServiceState::StopWatchdog
            | ServiceState::StopPost
            | ServiceState::StopSigterm
            | ServiceState::StopSigkill
            | ServiceState::FinalSigterm
            | ServiceState::FinalSigkill
            | ServiceState::FinalWatchdog => UnitActiveState::UnitDeActivating,
            ServiceState::Failed => UnitActiveState::UnitFailed,
            ServiceState::Cleaning => UnitActiveState::UnitMaintenance,
        }
    }

    fn to_unit_active_state_idle(self) -> UnitActiveState {
        match self {
            ServiceState::Dead => UnitActiveState::UnitInActive,
            ServiceState::Condition
            | ServiceState::StartPre
            | ServiceState::Start
            | ServiceState::StartPost
            | ServiceState::Running
            | ServiceState::Exited => UnitActiveState::UnitActive,
            ServiceState::Reload => UnitActiveState::UnitReloading,
            ServiceState::Stop
            | ServiceState::StopWatchdog
            | ServiceState::StopPost
            | ServiceState::StopSigterm
            | ServiceState::StopSigkill
            | ServiceState::FinalSigterm
            | ServiceState::FinalSigkill
            | ServiceState::FinalWatchdog => UnitActiveState::UnitDeActivating,
            ServiceState::Failed => UnitActiveState::UnitFailed,
            ServiceState::Cleaning => UnitActiveState::UnitMaintenance,
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
    mng: RefCell<Weak<ServiceMng>>,
    data: RefCell<Rtdata>,
}

impl RunningData {
    pub(super) fn new() -> Self {
        RunningData {
            mng: RefCell::new(Weak::new()),
            data: RefCell::new(Rtdata::new()),
        }
    }

    pub(super) fn attach_mng(&self, mng: Rc<ServiceMng>) {
        *self.mng.borrow_mut() = Rc::downgrade(&mng);
    }

    pub(self) fn attach_inotify(&self, path_inotify: Rc<PathIntofy>) {
        path_inotify.attach(self.mng.borrow_mut().clone());
        self.data.borrow_mut().attach_inotify(path_inotify);
    }

    pub(self) fn path_inotify(&self) -> Rc<PathIntofy> {
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
}

struct Rtdata {
    errno: i32,
    notify_state: NotifyState,
    path_inotify: Option<Rc<PathIntofy>>,
}

impl Rtdata {
    pub(self) fn new() -> Self {
        Rtdata {
            errno: 0,
            notify_state: NotifyState::Unknown,
            path_inotify: None,
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

    pub(self) fn attach_inotify(&mut self, path_inotify: Rc<PathIntofy>) {
        self.path_inotify = Some(path_inotify)
    }

    pub(self) fn path_inotify(&self) -> Rc<PathIntofy> {
        self.path_inotify.as_ref().unwrap().clone()
    }
}

#[derive(PartialEq, Eq, Debug, Copy, Clone, Hash)]
enum PathType {
    Changed,
    Modified,
}

struct PathIntofy {
    path: PathBuf,
    p_type: PathType,
    inotify: RefCell<RawFd>,
    wd: RefCell<Option<WatchDescriptor>>,
    mng: RefCell<Weak<ServiceMng>>,
}

impl fmt::Display for PathIntofy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "path: {:?}, path type: {:?}, inotify fd: {}",
            self.path,
            self.p_type,
            *self.inotify.borrow()
        )
    }
}

impl PathIntofy {
    fn new(path: PathBuf) -> Self {
        PathIntofy {
            path,
            p_type: PathType::Modified,
            inotify: RefCell::new(-1),
            wd: RefCell::new(None),
            mng: RefCell::new(Weak::new()),
        }
    }

    pub(self) fn attach(&self, mng: Weak<ServiceMng>) {
        log::debug!("attach service mng to path inotify");
        *self.mng.borrow_mut() = mng;
    }

    fn add_watch_path(&self) -> Result<bool, Error> {
        self.unwatch();

        let inotify = Inotify::init(InitFlags::all()).map_err(|_e| Error::Other {
            msg: "create initofy fd err",
        })?;
        *self.inotify.borrow_mut() = inotify.as_raw_fd();

        let ansters = self.path.as_path().ancestors();
        let mut primary: bool = true;
        let mut flags: AddWatchFlags;

        let mut exist = false;
        for anster in ansters {
            flags = if primary {
                AddWatchFlags::IN_DELETE_SELF
                    | AddWatchFlags::IN_MOVE_SELF
                    | AddWatchFlags::IN_ATTRIB
                    | AddWatchFlags::IN_CLOSE_WRITE
                    | AddWatchFlags::IN_CREATE
                    | AddWatchFlags::IN_DELETE
                    | AddWatchFlags::IN_MOVED_FROM
                    | AddWatchFlags::IN_MOVED_TO
                    | AddWatchFlags::IN_MODIFY
            } else {
                AddWatchFlags::IN_DELETE_SELF
                    | AddWatchFlags::IN_MOVE_SELF
                    | AddWatchFlags::IN_ATTRIB
                    | AddWatchFlags::IN_CREATE
                    | AddWatchFlags::IN_MOVED_TO
            };

            log::debug!(
                "inotify fd is: {}, flags is: {:?}, path: {:?}",
                *self.inotify.borrow(),
                flags,
                anster
            );

            match inotify.add_watch(anster, flags) {
                Ok(wd) => {
                    if primary {
                        *self.wd.borrow_mut() = Some(wd);
                    }

                    exist = true;
                    break;
                }
                Err(err) => {
                    log::error!("watch on path {:?} error: {:?}", anster, err);
                }
            }

            primary = false;
        }

        if !exist {
            return Err(Error::Other {
                msg: "watch on any of the ancestor failed",
            });
        }

        Ok(true)
    }

    fn unwatch(&self) {
        fd_util::close(*self.inotify.borrow());
        *self.inotify.borrow_mut() = -1;
    }

    fn read_fd_event(&self) -> Result<bool, Error> {
        let inotify = unsafe { Inotify::from_raw_fd(*self.inotify.borrow_mut()) };
        let events = match inotify.read_events() {
            Ok(events) => events,
            Err(e) => {
                if e == Errno::EAGAIN || e == Errno::EINTR {
                    return Ok(false);
                }

                return Err(Error::Other {
                    msg: "read evnets from inotify error",
                });
            }
        };

        if IN_SET!(self.p_type, PathType::Changed, PathType::Modified) {
            for event in events {
                if let Some(ref wd) = *self.wd.borrow() {
                    if event.wd == *wd {
                        return Ok(true);
                    }
                }
            }
        }

        Ok(false)
    }

    pub(self) fn mng(&self) -> Rc<ServiceMng> {
        self.mng.borrow().clone().upgrade().unwrap()
    }

    fn do_dispatch(&self) -> Result<i32, Error> {
        log::debug!("dispatch initify pid file: {:?}", self.path);
        match self.read_fd_event() {
            Ok(_) => {
                if let Ok(_v) = self.mng().retry_pid_file() {
                    return Ok(0);
                }

                if let Ok(_v) = self.mng().watch_pid_file() {
                    return Ok(0);
                }
            }
            Err(e) => {
                log::error!("in inotify dispatch, read event error: {}", e);
            }
        }

        self.mng().unwatch_pid_file();
        self.mng()
            .enter_signal(ServiceState::StopSigterm, ServiceResult::FailureResources);
        Ok(0)
    }
}

impl Source for PathIntofy {
    fn fd(&self) -> RawFd {
        *self.inotify.borrow()
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

    fn dispatch(&self, _: &Events) -> Result<i32, Error> {
        let ret = self.do_dispatch();
        self.mng().db_update();
        ret
    }

    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }
}
