//! socket_mng模块是socket类型的核心逻辑，主要实现socket端口的管理，子进程的拉起及子类型的状态管理。
//!

use std::{cell::RefCell, path::Path, rc::Rc};

use event::EventState;
use nix::{sys::signal::Signal, unistd::Pid};
use process1::manager::{
    ExecCommand, ExecContext, KillOperation, UnitActionError, UnitActiveState, UnitNotifyFlags,
    UnitRef, UnitType,
};
use utils::IN_SET;

use crate::{
    socket_base::SocketCommand, socket_comm::SocketComm, socket_config::SocketConfig,
    socket_pid::SocketPid, socket_port::SocketPorts, socket_spawn::SocketSpawn,
};

#[allow(dead_code)]
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub(super) enum SocketState {
    Dead,
    StartPre,
    StartChown,
    StartPost,
    Listening,
    Running,
    StopPre,
    StopPreSigterm,
    StopPreSigkill,
    StopPost,
    FinalSigterm,
    FinalSigkill,
    Failed,
    Cleaning,
    StateMax,
}

impl SocketState {
    pub(super) fn to_unit_active_state(&self) -> UnitActiveState {
        match *self {
            SocketState::Dead => UnitActiveState::UnitInActive,
            SocketState::StartPre | SocketState::StartChown | SocketState::StartPost => {
                UnitActiveState::UnitActivating
            }
            SocketState::Listening | SocketState::Running => UnitActiveState::UnitActive,
            SocketState::StopPre
            | SocketState::StopPreSigterm
            | SocketState::StopPost
            | SocketState::StopPreSigkill
            | SocketState::StateMax
            | SocketState::FinalSigterm
            | SocketState::FinalSigkill => UnitActiveState::UnitDeActivating,
            SocketState::Failed => UnitActiveState::UnitFailed,
            SocketState::Cleaning => UnitActiveState::UnitMaintenance,
        }
    }

    fn to_kill_operation(&self) -> KillOperation {
        match self {
            SocketState::StopPreSigterm => {
                // todo!() check has a restart job
                KillOperation::KillKill
            }
            SocketState::FinalSigterm => KillOperation::KillTerminate,
            _ => KillOperation::KillKill,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
enum SocketResult {
    Success,
    FailureResources,
    FailureTimeout,
    FailureExitCode,
    FailureSignal,
    FailureCoreDump,
    FailureStartLimitHit,
    FailureTriggerLimitHit,
    FailureServiceStartLimitHit,
    ResultInvalid,
}

#[allow(dead_code)]
pub(super) struct SocketMng {
    comm: Rc<SocketComm>,
    config: Rc<SocketConfig>,
    ports: Rc<SocketPorts>,

    pid: Rc<SocketPid>,
    spawn: SocketSpawn,
    state: Rc<RefCell<SocketState>>,
    result: RefCell<SocketResult>,
    control_command: RefCell<Vec<ExecCommand>>,
    refused: RefCell<i32>,
    service: RefCell<UnitRef>,
}

impl SocketMng {
    pub(super) fn new(
        commr: &Rc<SocketComm>,
        configr: &Rc<SocketConfig>,
        ports: &Rc<SocketPorts>,
        exec_ctx: &Rc<ExecContext>,
    ) -> SocketMng {
        let pid = Rc::new(SocketPid::new(commr));
        SocketMng {
            comm: commr.clone(),
            config: configr.clone(),
            ports: ports.clone(),
            spawn: SocketSpawn::new(commr, exec_ctx),
            state: Rc::new(RefCell::new(SocketState::StateMax)),
            result: RefCell::new(SocketResult::Success),
            control_command: RefCell::new(Vec::new()),
            pid: pid.clone(),
            refused: RefCell::new(0),
            service: RefCell::new(UnitRef::new()),
        }
    }

    pub(super) fn set_ref(&self, source: String, target: String) {
        self.service.borrow_mut().set_ref(source, target);
    }

    pub(super) fn load_related_unit(&self, related_type: UnitType) -> bool {
        let unit_name = self.comm.unit().get_id().to_string();
        let stem_name = Path::new(&unit_name).file_stem().unwrap().to_str().unwrap();

        let suffix = String::from(related_type);
        if suffix.len() == 0 {
            return false;
        }

        let relate_name = format!("{}.{}", stem_name, suffix);
        if !self.comm.um().load_unit_success(&relate_name) {
            return false;
        }

        self.set_ref(self.comm.unit().get_id().to_string(), relate_name);

        true
    }

    pub(super) fn unit_ref_target(&self) -> Option<String> {
        self.service
            .borrow()
            .target()
            .map_or(None, |v| Some(v.to_string()))
    }

    pub(super) fn start_check(&self) -> Result<bool, UnitActionError> {
        if IN_SET!(
            self.state(),
            SocketState::StopPre,
            SocketState::StopPreSigkill,
            SocketState::StopPreSigterm,
            SocketState::StopPost,
            SocketState::FinalSigterm,
            SocketState::FinalSigkill,
            SocketState::Cleaning
        ) {
            return Err(UnitActionError::UnitActionEAgain);
        }

        if IN_SET!(
            self.state(),
            SocketState::StartPre,
            SocketState::StartChown,
            SocketState::StartPost
        ) {
            return Ok(true);
        }

        self.unit_ref_target()
            .map_or(Ok(()), |name| match self.comm.um().unit_enabled(&name) {
                Ok(_) => Ok(()),
                Err(e) => Err(e),
            })?;

        Ok(false)
    }

    pub(super) fn start_action(&self) {
        self.enter_start_pre();
    }

    pub(super) fn stop_action(&self) {
        self.enter_stop_pre(SocketResult::Success)
    }

    pub(super) fn stop_check(&self) -> Result<bool, UnitActionError> {
        if IN_SET!(
            self.state(),
            SocketState::StopPre,
            SocketState::StopPreSigterm,
            SocketState::StopPreSigkill,
            SocketState::StopPost,
            SocketState::FinalSigterm,
            SocketState::FinalSigkill
        ) {
            return Ok(true);
        }

        if IN_SET!(
            self.state(),
            SocketState::StartPre,
            SocketState::StartChown,
            SocketState::StartPost
        ) {
            self.enter_signal(SocketState::StopPreSigterm, SocketResult::Success);
            return Err(UnitActionError::UnitActionEAgain);
        }

        Ok(false)
    }

    pub(super) fn current_active_state(&self) -> UnitActiveState {
        self.state().to_unit_active_state()
    }

    pub(super) fn enter_running(&self, fd: i32) {
        if self.comm.um().has_stop_job(self.comm.unit().get_id()) {
            if fd >= 0 {
                *self.refused.borrow_mut() += 1;
                return;
            }

            self.flush_ports();
            return;
        }

        if fd < 0 {
            if !self
                .comm
                .um()
                .relation_active_or_pending(self.comm.unit().get_id())
            {
                if self.unit_ref_target().is_none() {
                    self.enter_stop_pre(SocketResult::FailureResources);
                    return;
                }
                match self.comm.um().start_unit(&self.unit_ref_target().unwrap()) {
                    Ok(_) => {}
                    Err(_) => {
                        self.enter_stop_pre(SocketResult::FailureResources);
                        return;
                    }
                }
            }

            self.set_state(SocketState::Running);
            return;
        } else {
            // template support
            todo!()
        }
    }

    pub(super) fn state(&self) -> SocketState {
        *self.state.borrow()
    }

    fn enter_start_pre(&self) {
        log::debug!("enter start pre command");
        self.pid.unwatch_control();

        self.control_command_fill(SocketCommand::StartPre);
        match self.control_command_pop() {
            Some(cmd) => {
                match self.spawn.start_socket(&cmd) {
                    Ok(pid) => self.pid.set_control(pid),
                    Err(e) => {
                        log::error!(
                            "Failed to run start pre service: {}, error: {:?}",
                            self.comm.unit().get_id(),
                            e
                        );
                        self.enter_dead(SocketResult::FailureResources);
                        return;
                    }
                }
                self.set_state(SocketState::StartPre);
            }
            None => self.enter_start_chown(),
        }
    }

    fn enter_start_chown(&self) {
        log::debug!("enter start chown command");
        match self.open_fds() {
            Ok(_) => {
                self.enter_start_post();
            }
            Err(_) => self.enter_stop_pre(SocketResult::FailureResources),
        }
    }

    fn enter_start_post(&self) {
        log::debug!("enter start post command");
        self.pid.unwatch_control();
        self.control_command_fill(SocketCommand::StartPost);

        match self.control_command_pop() {
            Some(cmd) => {
                match self.spawn.start_socket(&cmd) {
                    Ok(pid) => self.pid.set_control(pid),
                    Err(_e) => {
                        log::error!(
                            "Failed to run start post service: {}",
                            self.comm.unit().get_id()
                        );
                        self.enter_stop_pre(SocketResult::FailureResources);
                        return;
                    }
                }
                self.set_state(SocketState::StartPost);
            }
            None => self.enter_listening(),
        }
    }

    fn enter_listening(&self) {
        log::debug!("enter start listening state");
        if !self.config.config_data().borrow().Socket.Accept {
            self.flush_ports();
        }

        self.watch_fds();

        self.set_state(SocketState::Listening)
    }

    fn enter_stop_pre(&self, res: SocketResult) {
        log::debug!("enter stop pre command");
        if self.result() == SocketResult::Success {
            self.set_result(res);
        }

        self.pid.unwatch_control();

        self.control_command_fill(SocketCommand::StopPre);

        match self.control_command_pop() {
            Some(cmd) => {
                match self.spawn.start_socket(&cmd) {
                    Ok(pid) => self.pid.set_control(pid),
                    Err(_e) => {
                        log::error!(
                            "Failed to run start post service: {}",
                            self.comm.unit().get_id()
                        );
                        self.enter_stop_post(SocketResult::FailureResources);
                        return;
                    }
                }
                self.set_state(SocketState::StopPre);
            }
            None => self.enter_stop_post(SocketResult::Success),
        }
    }

    fn enter_stop_post(&self, res: SocketResult) {
        log::debug!("enter stop post command");
        if self.result() == SocketResult::Success {
            self.set_result(res);
        }

        self.control_command_fill(SocketCommand::StopPost);

        match self.control_command_pop() {
            Some(cmd) => {
                match self.spawn.start_socket(&cmd) {
                    Ok(pid) => self.pid.set_control(pid),
                    Err(_e) => {
                        log::error!(
                            "Failed to run start post service: {}",
                            self.comm.unit().get_id()
                        );
                        self.enter_signal(
                            SocketState::FinalSigterm,
                            SocketResult::FailureResources,
                        );
                        return;
                    }
                }
                self.set_state(SocketState::StopPost);
            }
            None => self.enter_signal(SocketState::FinalSigterm, SocketResult::Success),
        }
    }

    fn enter_signal(&self, state: SocketState, res: SocketResult) {
        log::debug!("enter enter signal {:?}, res: {:?}", state, res);
        if self.result() == SocketResult::Success {
            self.set_result(res);
        }

        let op = state.to_kill_operation();
        match self.comm.unit().kill_context(None, self.pid.control(), op) {
            Ok(_) => {}
            Err(_e) => {
                if IN_SET!(
                    state,
                    SocketState::StopPreSigterm,
                    SocketState::StopPreSigkill
                ) {
                    return self.enter_stop_post(SocketResult::FailureResources);
                } else {
                    return self.enter_dead(SocketResult::FailureResources);
                }
            }
        }

        if state == SocketState::StopPreSigterm {
            self.enter_signal(SocketState::StopPreSigkill, SocketResult::Success);
        } else if state == SocketState::StopPreSigkill {
            self.enter_stop_post(SocketResult::Success);
        } else if state == SocketState::FinalSigterm {
            self.enter_signal(SocketState::FinalSigkill, SocketResult::Success);
        } else {
            self.enter_dead(SocketResult::Success)
        }
    }

    fn enter_dead(&self, res: SocketResult) {
        log::debug!("enter enter dead state, res {:?}", res);
        if self.result() == SocketResult::Success {
            self.set_result(res);
        }

        let state = if self.result() == SocketResult::Success {
            SocketState::Dead
        } else {
            SocketState::Failed
        };

        self.set_state(state);
    }

    fn run_next(&self) {
        if let Some(cmd) = self.control_command_pop() {
            match self.spawn.start_socket(&cmd) {
                Ok(pid) => self.pid.set_control(pid),
                Err(_e) => {
                    log::error!("failed to run main command: {}", self.comm.unit().get_id());
                }
            }
        }
    }

    fn open_fds(&self) -> Result<(), UnitActionError> {
        let ports = self.ports.ports();
        for port in ports.iter() {
            port.open_port().map_err(|_e| {
                log::error!("open port error: {}", _e);
                return UnitActionError::UnitActionEFailed;
            })?;

            port.apply_sock_opt(port.fd());
        }

        Ok(())
    }

    fn close_fds(&self) {
        let ports = self.ports.ports();
        for port in ports.iter() {
            port.close();
        }
    }

    fn watch_fds(&self) {
        let ports = self.ports.ports();
        for port in ports.iter() {
            self.comm.um().register(port.clone());

            self.comm.um().enable(port.clone(), EventState::On);
        }
    }

    fn unwatch_fds(&self) {
        let ports = self.ports.ports();
        for port in ports.iter() {
            self.comm.um().enable(port.clone(), EventState::Off);
        }
    }

    fn flush_ports(&self) {
        let ports = self.ports.ports();
        for port in ports.iter() {
            port.flush_accept();

            port.flush_fd();
        }
    }

    fn set_state(&self, state: SocketState) {
        let original_state = self.state();
        *self.state.borrow_mut() = state;

        // TODO
        // check the new state
        if !vec![
            SocketState::StartPre,
            SocketState::StartChown,
            SocketState::StartPost,
            SocketState::StopPre,
            SocketState::StopPreSigterm,
            SocketState::StopPreSigkill,
            SocketState::StopPost,
            SocketState::FinalSigterm,
            SocketState::FinalSigkill,
        ]
        .contains(&state)
        {
            self.pid.unwatch_control();
        }

        if state != SocketState::Listening {
            self.unwatch_fds();
        }

        if !vec![
            SocketState::StartChown,
            SocketState::StartPost,
            SocketState::Listening,
            SocketState::Running,
            SocketState::StopPre,
            SocketState::StopPreSigterm,
            SocketState::StopPreSigkill,
        ]
        .contains(&state)
        {
            self.close_fds();
        }

        log::debug!(
            "original state: {:?}, change to: {:?}",
            original_state,
            state
        );
        // todo!()
        // trigger the unit the dependency trigger_by

        self.comm.unit().notify(
            original_state.to_unit_active_state(),
            state.to_unit_active_state(),
            UnitNotifyFlags::UNIT_NOTIFY_RELOAD_FAILURE,
        );
    }

    fn control_command_fill(&self, cmd_type: SocketCommand) {
        if let Some(cmds) = self.config.get_exec_cmds(cmd_type) {
            *self.control_command.borrow_mut() = cmds
        }
    }

    fn control_command_pop(&self) -> Option<ExecCommand> {
        self.control_command.borrow_mut().pop()
    }

    fn result(&self) -> SocketResult {
        *self.result.borrow()
    }

    fn set_result(&self, res: SocketResult) {
        *self.result.borrow_mut() = res;
    }
}

impl SocketMng {
    pub(super) fn sigchld_event(&self, _pid: Pid, code: i32, status: Signal) {
        let res: SocketResult;
        if code == 0 {
            res = SocketResult::Success;
        } else if status != Signal::SIGCHLD {
            res = SocketResult::FailureSignal;
        } else {
            res = SocketResult::Success
        }

        if !self.control_command.borrow().is_empty() && res == SocketResult::Success {
            self.run_next();
        } else {
            match self.state() {
                SocketState::StartPre => {
                    if res == SocketResult::Success {
                        self.enter_start_chown();
                    } else {
                        self.enter_signal(SocketState::FinalSigterm, res);
                    }
                }
                SocketState::StartChown => {
                    if res == SocketResult::Success {
                        self.enter_start_post();
                    } else {
                        self.enter_stop_pre(res);
                    }
                }
                SocketState::StartPost => {
                    if res == SocketResult::Success {
                        self.enter_listening();
                    } else {
                        self.enter_stop_pre(res);
                    }
                }
                SocketState::StopPre
                | SocketState::StopPreSigterm
                | SocketState::StopPreSigkill => {
                    self.enter_stop_post(res);
                }
                SocketState::StopPost | SocketState::FinalSigterm | SocketState::FinalSigkill => {
                    self.enter_dead(res);
                }
                _ => {
                    log::error!(
                        "control command should not exit， current state is : {:?}",
                        self.state()
                    );
                    assert!(false);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_socket_active_state() {
        use super::SocketState;
        use process1::manager::UnitActiveState;

        assert_eq!(
            SocketState::Dead.to_unit_active_state(),
            UnitActiveState::UnitInActive
        );
        assert_eq!(
            SocketState::StartPre.to_unit_active_state(),
            UnitActiveState::UnitActivating
        );
        assert_eq!(
            SocketState::StartChown.to_unit_active_state(),
            UnitActiveState::UnitActivating
        );
        assert_eq!(
            SocketState::StartPost.to_unit_active_state(),
            UnitActiveState::UnitActivating
        );
        assert_eq!(
            SocketState::Listening.to_unit_active_state(),
            UnitActiveState::UnitActive
        );
        assert_eq!(
            SocketState::Running.to_unit_active_state(),
            UnitActiveState::UnitActive
        );
        assert_eq!(
            SocketState::StopPre.to_unit_active_state(),
            UnitActiveState::UnitDeActivating
        );
        assert_eq!(
            SocketState::StopPreSigterm.to_unit_active_state(),
            UnitActiveState::UnitDeActivating
        );
        assert_eq!(
            SocketState::StopPost.to_unit_active_state(),
            UnitActiveState::UnitDeActivating
        );
        assert_eq!(
            SocketState::StopPreSigkill.to_unit_active_state(),
            UnitActiveState::UnitDeActivating
        );
        assert_eq!(
            SocketState::FinalSigterm.to_unit_active_state(),
            UnitActiveState::UnitDeActivating
        );
        assert_eq!(
            SocketState::FinalSigterm.to_unit_active_state(),
            UnitActiveState::UnitDeActivating
        );
        assert_eq!(
            SocketState::Failed.to_unit_active_state(),
            UnitActiveState::UnitFailed
        );
        assert_eq!(
            SocketState::Cleaning.to_unit_active_state(),
            UnitActiveState::UnitMaintenance
        );
    }
}
