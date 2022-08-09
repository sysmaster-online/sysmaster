use super::service_base::{ServiceCommand, ServiceType};
use super::service_comm::ServiceComm;
use super::service_config::ServiceConfig;
use super::service_pid::ServicePid;
use super::service_spawn::ServiceSpawn;
use log;
use nix::sys::signal::Signal;
use nix::unistd::Pid;
use process1::manager::{
    ExecCommand, ExecFlags, KillOperation, UnitActionError, UnitActiveState, UnitNotifyFlags,
};
use std::cell::RefCell;
use std::rc::Rc;
use utils::IN_SET;

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
enum ServiceState {
    Dead,
    Condition,
    StartPre,
    Start,
    StartPost,
    Runing,
    Exited,
    Reload,
    Stop,
    StopWatchdog,
    StopPost,
    StopSigterm,
    StopSigkill,
    FinalWatchdog,
    FinalSigterm,
    FinalSigkill,
    Failed,
    AutoRestart,
    Cleaning,
}

impl Default for ServiceState {
    fn default() -> Self {
        ServiceState::Dead
    }
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
enum ServiceResult {
    Success,
    FailureResources,
    FailureTimeout,
    FailureSignal,
    FailureKill,
    ResultInvalid,
}

impl Default for ServiceResult {
    fn default() -> Self {
        ServiceResult::ResultInvalid
    }
}

pub(super) struct ServiceMng {
    // associated objects
    comm: Rc<ServiceComm>,
    config: Rc<ServiceConfig>,

    // owned objects
    pid: Rc<ServicePid>,
    spawn: ServiceSpawn,
    state: RefCell<ServiceState>,
    result: RefCell<ServiceResult>,
    main_command: RefCell<Vec<ExecCommand>>,
    control_command: RefCell<Vec<ExecCommand>>,
}

impl ServiceMng {
    pub(super) fn new(commr: &Rc<ServiceComm>, configr: &Rc<ServiceConfig>) -> ServiceMng {
        let _pid = Rc::new(ServicePid::new(commr));
        ServiceMng {
            comm: Rc::clone(commr),
            config: Rc::clone(configr),
            pid: Rc::clone(&_pid),
            spawn: ServiceSpawn::new(commr, &_pid),
            state: RefCell::new(ServiceState::Dead),
            result: RefCell::new(ServiceResult::Success),
            main_command: RefCell::new(Vec::new()),
            control_command: RefCell::new(Vec::new()),
        }
    }

    pub(super) fn start_check(&self) -> Result<(), UnitActionError> {
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

        Ok(())
    }

    pub(super) fn start_action(&self) {
        self.set_result(ServiceResult::Success);
        self.enter_contion();
    }

    pub(super) fn stop_check(&self) -> Result<(), UnitActionError> {
        let stop_state = vec![
            ServiceState::Stop,
            ServiceState::StopSigterm,
            ServiceState::StopSigkill,
            ServiceState::StopPost,
        ];

        if stop_state.contains(&self.state()) {
            return Err(UnitActionError::UnitActionEAlready);
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
            return;
        }

        self.enter_stop(ServiceResult::Success);
    }

    pub(super) fn reload_action(&self) {
        self.enter_reload();
    }

    pub(super) fn current_active_state(&self) -> UnitActiveState {
        if let Some(service_type) = self.config.Service.Type {
            service_state_to_unit_state(service_type, self.state())
        } else {
            UnitActiveState::UnitFailed
        }
    }

    fn enter_contion(&self) {
        log::debug!("enter running service condition command");
        self.control_command_fill(ServiceCommand::Condition);
        match self.control_command_pop() {
            Some(cmd) => {
                match self.spawn.start_service(&cmd, 0, ExecFlags::CONTROL) {
                    Ok(pid) => self.pid.set_control(pid),
                    Err(_e) => self.enter_dead(ServiceResult::FailureResources),
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
                    Err(_e) => self.enter_dead(ServiceResult::FailureResources),
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
        self.main_command_fill(ServiceCommand::Start);
        match self.main_command_pop() {
            Some(cmd) => {
                match self.spawn.start_service(&cmd, 0, ExecFlags::PASS_FDS) {
                    Ok(pid) => self.pid.set_main(pid),
                    Err(_e) => {
                        log::error!("failed to start service: {}", self.comm.unit().get_id());
                        self.enter_signal(
                            ServiceState::StopSigterm,
                            ServiceResult::FailureResources,
                        );
                    }
                }
                self.enter_start_post();
                // self.set_state(ServiceState::Start);
            }
            None => {
                self.enter_start_post();
            }
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
                            "Failed to run start post service: {}",
                            self.comm.unit().get_id()
                        );
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
            self.set_state(ServiceState::Runing);
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
                        log::error!("Failed to run stop service: {}", self.comm.unit().get_id());
                    }
                }
                self.set_state(ServiceState::Stop);
            }
            None => self.enter_signal(ServiceState::StopSigterm, ServiceResult::Success),
        }
    }

    fn enter_stop_post(&self, res: ServiceResult) {
        log::debug!("runing stop post, service result: {:?}", res);
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
                        log::error!("Failed to run stop service: {}", self.comm.unit().get_id());
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
                        log::error!("failed to start service: {}", self.comm.unit().get_id());
                        self.enter_running(ServiceResult::Success);
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

        self.comm
            .um()
            .child_watch_all_pids(self.comm.unit().get_id());

        let op = state.to_kill_operation();
        match self
            .comm
            .unit()
            .kill_context(self.pid.main(), self.pid.control(), op)
        {
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
            ServiceState::Runing,
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
            "original state: {:?}, change to: {:?}",
            original_state,
            state
        );
        // todo!()
        // trigger the unit the dependency trigger_by

        if let Some(service_type) = self.config.Service.Type {
            let os = service_state_to_unit_state(service_type, original_state);
            let ns = service_state_to_unit_state(service_type, state);
            self.comm
                .unit()
                .notify(os, ns, UnitNotifyFlags::UNIT_NOTIFY_RELOAD_FAILURE);
        }
    }

    fn service_alive(&self) -> bool {
        // todo!()
        true
    }

    fn run_next_control(&self) {
        log::debug!("runing next control command");
        if let Some(cmd) = self.control_command_pop() {
            match self.spawn.start_service(&cmd, 0, ExecFlags::CONTROL) {
                Ok(pid) => self.pid.set_control(pid),
                Err(_e) => {
                    log::error!("failed to start service: {}", self.comm.unit().get_id());
                }
            }
        }
    }

    fn run_next_main(&self) {
        if let Some(cmd) = self.main_command_pop() {
            match self.spawn.start_service(&cmd, 0, ExecFlags::PASS_FDS) {
                Ok(pid) => self.pid.set_main(pid),
                Err(_e) => {
                    log::error!("failed to run main command: {}", self.comm.unit().get_id());
                }
            }
        }
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

    fn main_command_fill(&self, cmd_type: ServiceCommand) {
        if let Some(cmds) = self.config.get_exec_cmds(cmd_type) {
            *self.main_command.borrow_mut() = cmds
        }
    }

    fn main_command_pop(&self) -> Option<ExecCommand> {
        self.main_command.borrow_mut().pop()
    }

    fn control_command_fill(&self, cmd_type: ServiceCommand) {
        if let Some(cmds) = self.config.get_exec_cmds(cmd_type) {
            *self.control_command.borrow_mut() = cmds
        }
    }

    fn control_command_pop(&self) -> Option<ExecCommand> {
        self.control_command.borrow_mut().pop()
    }

    // pub fn get_exec_cmds(&self, cmd_type: ServiceCommand) -> Vec<ExecCommand> {
    //     if let Some(cmds) = self.exec_commands.borrow_mut().get(&cmd_type) {
    //         cmds.as_slice()
    //     } else {
    //         Vec::new()
    //     }
    // }

    // pub fn insert_exec_cmds(&mut self, cmd_type: ServiceCommand, cmds: Vec<ExecCommand>) {
    //     self.exec_commands
    //         .borrow_mut()
    //         .insert(cmd_type, cmds.clone());
    // }
}

impl ServiceMng {
    pub(super) fn sigchld_event(&self, pid: Pid, code: i32, status: Signal) {
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
            self.pid.reset_main();

            if self.result() == ServiceResult::Success {
                self.set_result(res);
            }

            if !self.main_command.borrow().is_empty() && res == ServiceResult::Success {
                self.run_next_main();
            } else {
                self.main_command.borrow_mut().clear();
                match self.state() {
                    ServiceState::Dead => todo!(),
                    ServiceState::Start => {
                        self.enter_signal(ServiceState::StopSigterm, res);
                    }

                    ServiceState::StartPost | ServiceState::Reload => {
                        self.enter_stop(res);
                    }
                    ServiceState::Runing => {
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
            } else {
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
                        if res == ServiceResult::Success {
                            self.enter_start_post();
                        }
                    }
                    ServiceState::StartPost => {
                        self.enter_running(ServiceResult::Success);
                    }
                    ServiceState::Runing => todo!(),
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
}

impl ServiceState {
    fn to_unit_active_state(&self) -> UnitActiveState {
        match *self {
            ServiceState::Dead => UnitActiveState::UnitInActive,
            ServiceState::Condition
            | ServiceState::StartPre
            | ServiceState::Start
            | ServiceState::StartPost => UnitActiveState::UnitActivating,
            ServiceState::Runing | ServiceState::Exited => UnitActiveState::UnitActive,
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
            ServiceState::AutoRestart => UnitActiveState::UnitActivating,
            ServiceState::Cleaning => UnitActiveState::UnitMaintenance,
        }
    }

    fn to_unit_active_state_idle(&self) -> UnitActiveState {
        match *self {
            ServiceState::Dead => UnitActiveState::UnitInActive,
            ServiceState::Condition
            | ServiceState::StartPre
            | ServiceState::Start
            | ServiceState::StartPost
            | ServiceState::Runing
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
            ServiceState::AutoRestart => UnitActiveState::UnitActivating,
            ServiceState::Cleaning => UnitActiveState::UnitMaintenance,
        }
    }

    fn to_kill_operation(&self) -> KillOperation {
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
