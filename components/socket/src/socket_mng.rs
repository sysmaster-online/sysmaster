//! socket_mng is the core of the socket unit，implement the state transition, ports management and sub child management.
//!
use super::{
    socket_base::PortType,
    socket_comm::SocketUnitComm,
    socket_config::SocketConfig,
    socket_pid::SocketPid,
    socket_port::SocketPort,
    socket_rentry::{SocketCommand, SocketRe, SocketReFrame, SocketResult, SocketState},
    socket_spawn::SocketSpawn,
};
use event::EventState;
use event::{EventType, Events, Source};
use nix::errno::Errno;
use nix::libc::{self};
use nix::{sys::signal::Signal, unistd::Pid};
use process1::manager::{
    ExecCommand, ExecContext, KillOperation, ReliLastFrame, UnitActionError, UnitActiveState,
    UnitNotifyFlags, UnitType,
};
use process1::{ReStation, Reliability};
use std::cell::RefCell;
use std::os::unix::prelude::RawFd;
use std::rc::{Rc, Weak};
use utils::Error;
use utils::IN_SET;

impl SocketState {
    pub(super) fn to_unit_active_state(self) -> UnitActiveState {
        match self {
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

    fn to_kill_operation(self) -> KillOperation {
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

pub(super) struct SocketMng {
    data: Rc<SocketMngData>,
}

impl ReStation for SocketMng {
    // input: do nothing

    // compensate: do nothing

    // data
    fn db_map(&self) {
        self.data.db_map();
    }

    fn db_insert(&self) {
        self.data.db_insert();
    }

    // reload: entry-only
    fn entry_coldplug(&self) {
        self.data.entry_coldplug();
    }

    fn entry_clear(&self) {
        self.data.entry_clear();
    }
}

impl SocketMng {
    pub(super) fn new(
        commr: &Rc<SocketUnitComm>,
        configr: &Rc<SocketConfig>,
        exec_ctx: &Rc<ExecContext>,
    ) -> SocketMng {
        SocketMng {
            data: SocketMngData::new(commr, configr, exec_ctx),
        }
    }

    pub(super) fn start_check(&self) -> Result<bool, UnitActionError> {
        self.data.start_check()
    }

    pub(super) fn start_action(&self) {
        self.data.start_action();
        self.db_update();
    }

    pub(super) fn stop_check(&self) -> Result<bool, UnitActionError> {
        self.data.stop_check()
    }

    pub(super) fn stop_action(&self) {
        self.data.stop_action();
        self.db_update();
    }

    pub(super) fn sigchld_event(&self, _pid: Pid, code: i32, status: Signal) {
        self.data.sigchld_event(_pid, code, status);
        self.db_update();
    }

    pub(super) fn current_active_state(&self) -> UnitActiveState {
        self.data.current_active_state()
    }

    pub(super) fn collect_fds(&self) -> Vec<i32> {
        self.data.collect_fds()
    }
}

struct SocketMngData {
    // associated objects
    comm: Rc<SocketUnitComm>,
    config: Rc<SocketConfig>,

    // owned objects
    pid: SocketPid,
    spawn: SocketSpawn,
    ports: RefCell<Vec<Rc<SocketMngPort>>>,
    state: Rc<RefCell<SocketState>>,
    result: RefCell<SocketResult>,
    control_cmd_type: RefCell<Option<SocketCommand>>,
    control_command: RefCell<Vec<ExecCommand>>,
    refused: RefCell<i32>,
}

// the declaration "pub(self)" is for identification only.
impl SocketMngData {
    pub(self) fn new(
        commr: &Rc<SocketUnitComm>,
        configr: &Rc<SocketConfig>,
        exec_ctx: &Rc<ExecContext>,
    ) -> Rc<SocketMngData> {
        let mng = Rc::new(SocketMngData {
            comm: Rc::clone(commr),
            config: Rc::clone(configr),

            pid: SocketPid::new(commr),
            spawn: SocketSpawn::new(commr, exec_ctx),
            ports: RefCell::new(Vec::new()),
            state: Rc::new(RefCell::new(SocketState::StateMax)),
            result: RefCell::new(SocketResult::Success),
            control_cmd_type: RefCell::new(None),
            control_command: RefCell::new(Vec::new()),
            refused: RefCell::new(0),
        });
        mng.build_ports(configr, &mng);
        mng
    }

    pub(self) fn db_map(&self) {
        if let Some((state, result, c_pid, control_cmd_type, control_cmd_len, refused, rports)) =
            self.comm.rentry_mng_get()
        {
            *self.state.borrow_mut() = state;
            *self.result.borrow_mut() = result;
            self.pid.update_control(c_pid);
            self.control_command_update(control_cmd_type, control_cmd_len);
            *self.refused.borrow_mut() = refused;
            self.map_ports_fd(rports);
        }
    }

    fn entry_clear(&self) {
        self.unwatch_fds();
        // self.unwatch_pid_file: todo!()
    }

    fn entry_coldplug(&self) {
        self.watch_fds();
    }

    pub(self) fn start_check(&self) -> Result<bool, UnitActionError> {
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

        self.config.unit_ref_target().map_or(Ok(()), |name| {
            match self.comm.um().unit_enabled(&name) {
                Ok(_) => Ok(()),
                Err(e) => Err(e),
            }
        })?;

        if self.comm.unit().test_start_limit() {
            self.enter_dead(SocketResult::FailureStartLimitHit);
            return Err(UnitActionError::UnitActionECanceled);
        }

        Ok(false)
    }

    pub(self) fn start_action(&self) {
        self.enter_start_pre();
    }

    pub(self) fn stop_action(&self) {
        self.enter_stop_pre(SocketResult::Success);
    }

    pub(self) fn stop_check(&self) -> Result<bool, UnitActionError> {
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

    pub(self) fn current_active_state(&self) -> UnitActiveState {
        self.state().to_unit_active_state()
    }

    #[allow(dead_code)]
    pub(self) fn clear_ports(&self) {
        self.ports.borrow_mut().clear();
    }

    pub(self) fn collect_fds(&self) -> Vec<i32> {
        let mut fds = Vec::new();
        for port in self.ports().iter() {
            if port.fd() >= 0 {
                fds.push(port.fd() as i32);
            }
        }

        fds
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
                            self.comm.unit().id(),
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
                            self.comm.unit().id()
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

    fn enter_running(&self, fd: i32) {
        if self.comm.um().has_stop_job(self.comm.unit().id()) {
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
                .relation_active_or_pending(self.comm.unit().id())
            {
                if self.config.unit_ref_target().is_none() {
                    self.rentry().set_last_frame(SocketReFrame::FdListen(true));
                    self.enter_stop_pre(SocketResult::FailureResources);
                    return;
                }
                match self
                    .comm
                    .um()
                    .start_unit(&self.config.unit_ref_target().unwrap())
                {
                    Ok(_) => {}
                    Err(_) => {
                        self.rentry().set_last_frame(SocketReFrame::FdListen(true));
                        self.enter_stop_pre(SocketResult::FailureResources);
                        return;
                    }
                }
            }

            self.set_state(SocketState::Running);
        } else {
            // template support
            todo!()
        }
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
                            self.comm.unit().id()
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
                            self.comm.unit().id()
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
                    log::error!("failed to run main command: {}", self.comm.unit().id());
                }
            }
        }
    }

    fn open_fds(&self) -> Result<(), Errno> {
        for port in self.ports().iter() {
            let ret1 = port.open_port(true);
            if ret1.is_err() {
                self.close_fds();
                return ret1.map(|_| ());
            }

            port.apply_sock_opt(port.fd());
        }

        Ok(())
    }

    fn close_fds(&self) {
        // event
        let events = self.comm.um().events();
        for mport in self.mports().iter() {
            let source = Rc::clone(mport);
            events.del_source(source).unwrap();
        }

        // close
        for port in self.ports().iter() {
            port.close();
        }
    }

    fn watch_fds(&self) {
        let events = self.comm.um().events();
        for mport in self.mports().iter() {
            let source = Rc::clone(mport);
            events.add_source(source).unwrap();
            let source = Rc::clone(mport);
            events.set_enabled(source, EventState::On).unwrap();
        }
    }

    fn unwatch_fds(&self) {
        let events = self.comm.um().events();
        for mport in self.mports().iter() {
            let source = Rc::clone(mport);
            events.set_enabled(source, EventState::Off).unwrap();
        }
    }

    fn flush_ports(&self) {
        for port in self.ports().iter() {
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

    fn state(&self) -> SocketState {
        *self.state.borrow()
    }

    fn control_command_fill(&self, cmd_type: SocketCommand) {
        if let Some(cmds) = self.config.get_exec_cmds(cmd_type) {
            *self.control_command.borrow_mut() = cmds
        }
    }

    fn control_command_pop(&self) -> Option<ExecCommand> {
        self.control_command.borrow_mut().pop()
    }

    fn control_command_update(&self, cmd_type: Option<SocketCommand>, len: usize) {
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

    fn result(&self) -> SocketResult {
        *self.result.borrow()
    }

    fn set_result(&self, res: SocketResult) {
        *self.result.borrow_mut() = res;
    }

    fn build_ports(&self, configr: &Rc<SocketConfig>, mng: &Rc<SocketMngData>) {
        for p_conf in configr.ports().iter() {
            let port = Rc::new(SocketPort::new(&self.comm, configr, p_conf));
            let mport = Rc::new(SocketMngPort::new(mng, port));
            self.ports.borrow_mut().push(mport);
        }
    }

    fn map_ports_fd(&self, rports: Vec<RawFd>) {
        let ports = self.ports();
        assert_eq!(rports.len(), ports.len());

        // one to one in turn
        let mut rfds = rports;
        for port in ports.iter() {
            let fd = rfds.pop().unwrap();
            port.set_fd(self.comm.reli().fd_take(fd));
        }
    }

    fn mports(&self) -> Vec<Rc<SocketMngPort>> {
        self.ports
            .borrow()
            .iter()
            .map(|p| Rc::clone(p))
            .collect::<_>()
    }

    fn ports(&self) -> Vec<Rc<SocketPort>> {
        self.ports
            .borrow()
            .iter()
            .map(|p| Rc::clone(&p.port))
            .collect::<_>()
    }

    fn rentry(&self) -> Rc<SocketRe> {
        self.comm.rentry()
    }

    fn db_insert(&self) {
        self.comm.rentry_mng_insert(
            self.state(),
            self.result(),
            self.pid.control(),
            *self.control_cmd_type.borrow(),
            self.control_command.borrow().len(),
            *self.refused.borrow(),
            self.ports().iter().map(|p| p.fd()).collect::<_>(),
        );
    }

    fn db_update(&self) {
        self.db_insert();
    }
}

// the declaration "pub(self)" is for identification only.
impl SocketMngData {
    pub(self) fn sigchld_event(&self, _pid: Pid, code: i32, status: Signal) {
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
                        "control command should not exit, current state is : {:?}",
                        self.state()
                    );
                    unreachable!();
                }
            }
        }
    }
}

struct SocketMngPort {
    // associated objects
    mng: Weak<SocketMngData>,

    // owned objects
    port: Rc<SocketPort>,
}

impl Source for SocketMngPort {
    fn fd(&self) -> RawFd {
        self.port.fd()
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
        println!("Dispatching IO!");

        self.reli().set_last_frame2(
            ReliLastFrame::SubManager as u32,
            UnitType::UnitSocket as u32,
        );
        self.rentry().set_last_frame(SocketReFrame::FdListen(false));
        let ret = self.dispatch_io();
        self.rentry().clear_last_frame();
        self.reli().clear_last_frame();
        ret
    }

    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }
}

// the declaration "pub(self)" is for identification only.
impl SocketMngPort {
    pub(self) fn new(mng: &Rc<SocketMngData>, port: Rc<SocketPort>) -> SocketMngPort {
        SocketMngPort {
            mng: Rc::downgrade(mng),
            port,
        }
    }

    fn dispatch_io(&self) -> Result<i32, Error> {
        let afd: i32 = -1;

        if self.mng().state() != SocketState::Listening {
            return Ok(0);
        }

        if self.mng().config.config_data().borrow().Socket.Accept
            && self.port.p_type() == PortType::Socket
            && self.port.sa().can_accept()
        {
            self.rentry().set_last_frame(SocketReFrame::FdListen(true));
            let afd = self
                .port
                .accept()
                .map_err(|_e| Error::Other { msg: "accept err" })?;

            self.port.apply_sock_opt(afd)
        }

        self.mng().enter_running(afd);
        self.mng().db_update();

        Ok(0)
    }

    fn reli(&self) -> Rc<Reliability> {
        self.mng().comm.reli()
    }

    fn rentry(&self) -> Rc<SocketRe> {
        self.mng().comm.rentry()
    }

    fn mng(&self) -> Rc<SocketMngData> {
        self.mng.clone().upgrade().unwrap()
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
