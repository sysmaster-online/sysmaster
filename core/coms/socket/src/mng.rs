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

//! socket_mng is the core of the socket unit，implement the state transition, ports management and sub child management.
//!
use super::{
    comm::SocketUnitComm,
    config::SocketConfig,
    pid::SocketPid,
    port::SocketPort,
    rentry::{PortType, SocketCommand, SocketRe, SocketReFrame, SocketResult, SocketState},
    spawn::SocketSpawn,
};
use basic::{
    unistd::{get_group_creds, get_user_creds},
    IN_SET,
};
use core::exec::{ExecCommand, ExecContext, ExecFlag};
use core::rel::ReliLastFrame;
use core::rel::{ReStation, Reliability};
use core::unit::{KillOperation, UnitActiveState, UnitNotifyFlags, UnitType};
use core::{
    error::*,
    unit::{UnitDependencyMask, UnitRelations},
};
use event::EventState;
use event::{EventType, Events, Source};
use nix::sys::{socket, wait::WaitStatus};
use nix::unistd::{Gid, Uid};
use nix::{
    libc::{self},
    unistd::unlink,
};
use std::os::unix::prelude::RawFd;
use std::rc::{Rc, Weak};
use std::{cell::RefCell, collections::VecDeque};

impl SocketState {
    pub(super) fn to_unit_active_state(self) -> UnitActiveState {
        match self {
            SocketState::Dead => UnitActiveState::InActive,
            SocketState::StartPre | SocketState::StartChown | SocketState::StartPost => {
                UnitActiveState::Activating
            }
            SocketState::Listening | SocketState::Running => UnitActiveState::Active,
            SocketState::StopPre
            | SocketState::StopPreSigterm
            | SocketState::StopPost
            | SocketState::StopPreSigkill
            | SocketState::StateMax
            | SocketState::FinalSigterm
            | SocketState::FinalSigkill => UnitActiveState::DeActivating,
            SocketState::Failed => UnitActiveState::Failed,
            SocketState::Cleaning => UnitActiveState::Maintenance,
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

impl ReStation for SocketMng {
    // input: do nothing

    // compensate: do nothing

    // data
    fn db_map(&self, _reload: bool) {
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

    fn db_insert(&self) {
        self.comm.rentry_mng_insert(
            self.state(),
            self.result(),
            self.pid.control(),
            *self.control_cmd_type.borrow(),
            self.control_command.borrow().len(),
            *self.refused.borrow(),
            self.ports()
                .iter()
                .map(|p| (p.p_type(), String::from(p.listen()), p.fd()))
                .collect::<_>(),
        );
    }

    // reload: entry-only
    fn entry_coldplug(&self) {
        if self.state() == SocketState::Listening {
            self.watch_fds();
        }
    }

    fn entry_clear(&self) {
        // port fd is a long-term monitoring file and cannot be closed
        self.unwatch_fds();
    }
}

pub(crate) struct SocketMng {
    // associated objects
    comm: Rc<SocketUnitComm>,
    config: Rc<SocketConfig>,

    // owned objects
    pid: SocketPid,
    spawn: SocketSpawn,
    n_accept: RefCell<i32>,
    ports: RefCell<Vec<Rc<SocketMngPort>>>,
    state: RefCell<SocketState>,
    result: RefCell<SocketResult>,
    control_cmd_type: RefCell<Option<SocketCommand>>,
    control_command: RefCell<VecDeque<ExecCommand>>,
    current_control_command: RefCell<ExecCommand>,
    refused: RefCell<i32>,
}

// the declaration "pub(self)" is for identification only.
impl SocketMng {
    pub(crate) fn new(
        commr: &Rc<SocketUnitComm>,
        configr: &Rc<SocketConfig>,
        exec_ctx: &Rc<ExecContext>,
    ) -> SocketMng {
        SocketMng {
            comm: Rc::clone(commr),
            config: Rc::clone(configr),

            pid: SocketPid::new(commr),
            spawn: SocketSpawn::new(commr, exec_ctx),
            n_accept: RefCell::new(0),
            ports: RefCell::new(Vec::new()),
            state: RefCell::new(SocketState::StateMax),
            result: RefCell::new(SocketResult::Success),
            control_cmd_type: RefCell::new(None),
            control_command: RefCell::new(VecDeque::new()),
            current_control_command: RefCell::new(ExecCommand::empty()),
            refused: RefCell::new(0),
        }
    }

    pub(crate) fn start_check(&self) -> Result<bool> {
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
            return Err(Error::UnitActionEAgain);
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
        let ret = self.comm.owner().map(|u| u.test_start_limit());
        if ret.is_none() || !ret.unwrap() {
            self.enter_dead(SocketResult::FailureStartLimitHit);
            return Err(Error::UnitActionECanceled);
        }
        Ok(false)
    }

    pub(crate) fn start_action(&self) {
        /* make sure the former failure doesn't disturb later action. */
        self.set_result(SocketResult::Success);
        self.enter_start_pre();
        self.db_update();
    }

    pub(crate) fn stop_action(&self) {
        self.enter_stop_pre(SocketResult::Success);
        self.db_update();
    }

    pub(crate) fn stop_check(&self) -> Result<bool> {
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
            return Err(Error::UnitActionEAgain);
        }

        Ok(false)
    }

    pub(crate) fn current_active_state(&self) -> UnitActiveState {
        self.state().to_unit_active_state()
    }

    #[allow(dead_code)]
    pub(self) fn clear_ports(&self) {
        self.ports.borrow_mut().clear();
    }

    pub(crate) fn collect_fds(&self) -> Vec<i32> {
        let mut fds = Vec::new();
        for port in self.ports().iter() {
            if port.fd() >= 0 {
                fds.push(port.fd());
            }
        }

        fds
    }

    pub(crate) fn enter_start_pre(&self) {
        log::debug!("enter start pre command");
        self.pid.unwatch_control();

        self.control_command_fill(SocketCommand::StartPre);
        let cmd = match self.control_command_pop() {
            None => {
                self.enter_start_chown();
                return;
            }
            Some(v) => v,
        };
        *self.current_control_command.borrow_mut() = cmd.clone();
        let pid = match self.spawn.start_socket(&cmd) {
            Err(e) => {
                let unit_name = self.comm.owner().map_or("null".to_string(), |u| u.id());
                log::error!("Failed to run ExecStartPre for unit {}: {:?}", unit_name, e);
                self.enter_dead(SocketResult::FailureResources);
                return;
            }
            Ok(v) => v,
        };
        self.pid.set_control(pid);
        self.set_state(SocketState::StartPre);
    }

    pub(crate) fn push_port(&self, port: Rc<SocketMngPort>) {
        self.ports.borrow_mut().push(port);
        self.db_update();
    }

    fn enter_start_chown(&self) {
        log::debug!("enter start chown command");
        match self.open_fds() {
            Ok(_) => {
                let socket_data = self.config.config_data();
                let user = &socket_data.borrow().Socket.SocketUser;
                let group = &socket_data.borrow().Socket.SocketGroup;

                if !user.is_empty() || !group.is_empty() {
                    if let Err(e) = self.socket_chown(user, group) {
                        log::error!("chown path error: {}", e);
                        self.enter_stop_pre(SocketResult::FailureResources);
                        return;
                    }
                }

                self.enter_start_post();
            }
            Err(_) => self.enter_stop_pre(SocketResult::FailureResources),
        }
    }

    fn enter_start_post(&self) {
        log::debug!("enter start post command");
        self.pid.unwatch_control();
        self.control_command_fill(SocketCommand::StartPost);

        let cmd = match self.control_command_pop() {
            None => {
                self.enter_listening();
                return;
            }
            Some(v) => v,
        };
        *self.current_control_command.borrow_mut() = cmd.clone();
        match self.spawn.start_socket(&cmd) {
            Ok(pid) => self.pid.set_control(pid),
            Err(e) => {
                let unit_name = self.comm.owner().map_or("null".to_string(), |u| u.id());
                log::error!(
                    "Failed to run ExecStartPost for unit {}: {:?}",
                    unit_name,
                    e
                );
                self.enter_stop_pre(SocketResult::FailureResources);
                return;
            }
        }
        self.set_state(SocketState::StartPost);
    }

    pub(crate) fn enter_listening(&self) {
        log::debug!("enter start listening state");
        if !self.config.config_data().borrow().Socket.Accept
            && self.config.config_data().borrow().Socket.FlushPending
        {
            self.flush_ports();
        }

        self.watch_fds();

        self.set_state(SocketState::Listening)
    }

    fn enter_running(&self, fd: i32) {
        log::info!("enter running, fd: {}", fd);
        if let Some(u) = self.comm.owner() {
            if self.comm.um().has_stop_job(&u.id()) {
                if fd >= 0 {
                    *self.refused.borrow_mut() += 1;
                    return;
                }
                self.flush_ports();
                return;
            }
            if fd < 0 {
                if !self.comm.um().relation_active_or_pending(&u.id()) {
                    if self.config.unit_ref_target().is_none() {
                        self.enter_stop_pre(SocketResult::FailureResources);
                        return;
                    }
                    let service = self.config.unit_ref_target().unwrap();

                    // start corresponding *.service
                    self.rentry().set_last_frame(SocketReFrame::FdListen(false)); // protect 'start_unit'
                    let ret = self.comm.um().unit_start_by_job(&service);
                    self.rentry().set_last_frame(SocketReFrame::FdListen(true));
                    if ret.is_err() {
                        self.enter_stop_pre(SocketResult::FailureResources);
                        return;
                    }
                }
                self.set_state(SocketState::Running);
            } else {
                /* When Accept is set to yes, we can no longer use "Service=" in
                 * the socket file. The corresponding service name will be forily
                 * set to "socket prefix + @ + automatically generated instance".
                 * i.e. the service name of test.socket may be test@1-5427-0.service,
                 * where 1 is the total accept number, 5247 is PID, 0 is UID. */

                /* 1. build service name */
                let socket_name = u.id();
                let socket_prefix = match socket_name.split_once('.') {
                    None => {
                        log::error!("Invalid socket name: {}, weird.", u.id());
                        return;
                    }
                    Some(v) => v.0,
                };
                let n_accept = self.get_accept_number();
                let instance = Self::instance_from_socket_fd(fd, n_accept);
                let service = socket_prefix.to_string() + "@" + &instance + ".service";

                /* 2. load the service */
                if !self.comm.um().load_unit_success(&service) {
                    log::error!("Failed to load the triggered service: {}", service);
                    return;
                }
                /* 3. add dependency */
                if self
                    .comm
                    .um()
                    .unit_add_two_dependency(
                        &u.id(),
                        UnitRelations::UnitBefore,
                        UnitRelations::UnitTriggers,
                        &service,
                        false,
                        UnitDependencyMask::Implicit,
                    )
                    .is_err()
                {
                    log::error!("Failed to add dependency for {} -> {}", u.id(), service);
                    return;
                }
                /* 4. set the service socket fd */
                self.comm.um().service_set_socket_fd(&service, fd);
                self.increase_accept_number();
                /* 5. start */
                let ret = self.comm.um().unit_start_by_job(&service);
                if ret.is_err() {
                    self.comm.um().service_release_socket_fd(&service, fd);
                    self.enter_stop_pre(SocketResult::FailureResources);
                }
            }
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
                *self.current_control_command.borrow_mut() = cmd.clone();
                match self.spawn.start_socket(&cmd) {
                    Ok(pid) => self.pid.set_control(pid),
                    Err(_e) => {
                        if let Some(u) = self.comm.owner() {
                            log::error!("Failed to run stop pre cmd for service: {}", u.id());
                        } else {
                            log::error!("Failed to run stop pre cmd and service unit id is None");
                        }
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
                *self.current_control_command.borrow_mut() = cmd.clone();
                match self.spawn.start_socket(&cmd) {
                    Ok(pid) => self.pid.set_control(pid),
                    Err(e) => {
                        #[allow(clippy::unit_arg)]
                        let _ = self.comm.owner().map_or(
                            log::error!("Failed to run stop post cmd and service unit id is None"),
                            |u| {
                                log::error!(
                                    "Failed to run stop post cmd for service: {},err {}",
                                    u.id(),
                                    e
                                )
                            },
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
        if let Some(u) = self.comm.owner() {
            match u.kill_context(
                self.config.kill_context(),
                None,
                self.pid.control(),
                op,
                false,
            ) {
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
        };

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
            *self.current_control_command.borrow_mut() = cmd.clone();
            match self.spawn.start_socket(&cmd) {
                Ok(pid) => self.pid.set_control(pid),
                Err(_e) => {
                    if let Some(u) = self.comm.owner() {
                        log::error!("failed to run main command unit{},err {}", u.id(), _e);
                    } else {
                        log::error!("failed to run main command unit is None,Error: {}", _e);
                    }
                }
            }
        }
    }

    fn open_fds(&self) -> Result<()> {
        for port in self.ports().iter() {
            let ret = port.open_port(true);
            if ret.is_err() {
                self.close_fds();
                return ret;
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

        for port in self.ports().iter() {
            port.close(true);
        }

        if !self.config.config_data().borrow().Socket.RemoveOnStop {
            return;
        }

        // remove only when RemoveOnStop is true
        for port in self.ports().iter() {
            port.unlink();
        }
        // remove symlinks
        let config = self.config.config_data();
        for symlink in &config.borrow().Socket.Symlinks {
            let _ = unlink(symlink.to_str().unwrap());
        }
    }

    fn watch_fds(&self) {
        let events = self.comm.um().events();
        for mport in self.mports().iter() {
            if mport.fd() < 0 {
                continue;
            }
            let source = Rc::clone(mport);
            if !events.has_source(source.clone()) {
                events.add_source(source.clone()).unwrap();
            }
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
            if port.fd() < 0 {
                continue;
            }
            let _ = port.flush_accept();
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

        if let Some(u) = self.comm.owner() {
            u.notify(
                original_state.to_unit_active_state(),
                state.to_unit_active_state(),
                UnitNotifyFlags::RELOAD_FAILURE,
            )
        }
    }

    pub(crate) fn state(&self) -> SocketState {
        *self.state.borrow()
    }

    fn control_command_fill(&self, cmd_type: SocketCommand) {
        if let Some(cmds) = self.config.get_exec_cmds(cmd_type) {
            *self.control_command.borrow_mut() = cmds
        }
    }

    fn control_command_pop(&self) -> Option<ExecCommand> {
        self.control_command.borrow_mut().pop_front()
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

    fn map_ports_fd(&self, rports: Vec<(PortType, String, RawFd)>) {
        for (p_type, listen, fd) in rports.iter() {
            match self.ports_find(*p_type, listen) {
                Some(port) => {
                    port.set_fd(self.comm.reli().fd_take(*fd));
                }
                None => log::debug!("Not find {:?}:{:?}", *p_type, listen),
            }
        }
    }

    fn mports(&self) -> Vec<Rc<SocketMngPort>> {
        self.ports.borrow().iter().map(Rc::clone).collect::<_>()
    }

    fn ports_find(&self, p_type: PortType, listen: &str) -> Option<Rc<SocketPort>> {
        let ports = self.ports();
        for port in ports.iter() {
            if port.p_type() == p_type && port.listen() == listen {
                return Some(Rc::clone(port));
            }
        }

        None
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

    fn db_update(&self) {
        self.db_insert();
    }

    fn sigchld_result(&self, wait_status: WaitStatus) -> SocketResult {
        match wait_status {
            WaitStatus::Exited(_, status) => {
                if status == 0 {
                    SocketResult::Success
                } else {
                    SocketResult::FailureExitCode
                }
            }
            WaitStatus::Signaled(_, _, core_dump) => {
                if core_dump {
                    SocketResult::FailureCoreDump
                } else {
                    SocketResult::FailureSignal
                }
            }
            _ => unreachable!(),
        }
    }

    pub(crate) fn sigchld_event(&self, wait_status: WaitStatus) {
        let mut res = self.sigchld_result(wait_status);

        if self
            .current_control_command
            .borrow()
            .get_exec_flag()
            .contains(ExecFlag::EXEC_COMMAND_IGNORE_FAILURE)
        {
            res = SocketResult::Success;
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

        self.db_update();
    }

    fn socket_chown(&self, user: &str, group: &str) -> Result<()> {
        let mut uid = Uid::from_raw(u32::MAX);
        let mut gid = Gid::from_raw(u32::MAX);
        if !user.is_empty() {
            let user = get_user_creds(user)?;
            uid = user.uid;
            gid = user.gid;
        }

        if !group.is_empty() {
            let group = get_group_creds(group)?;
            gid = group.gid
        }
        for port in self.ports().iter() {
            port.chown(uid, gid)?;
        }

        Ok(())
    }

    fn get_accept_number(&self) -> i32 {
        *self.n_accept.borrow()
    }

    fn increase_accept_number(&self) {
        let current = self.get_accept_number();
        *self.n_accept.borrow_mut() = current + 1;
    }

    fn instance_from_socket_fd(fd: i32, n_accept: i32) -> String {
        match socket::getsockopt(fd, socket::sockopt::PeerCredentials) {
            Err(e) => {
                log::error!(
                    "Failed to get the credentials when building instance name: {}, use unknown.",
                    e
                );
                format!("{}-unknown", n_accept)
            }
            Ok(v) => format!("{}-{}-{}", n_accept, v.pid(), v.uid()),
        }
    }
}

pub(crate) struct SocketMngPort {
    // associated objects
    mng: Weak<SocketMng>,

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

    fn dispatch(&self, _: &Events) -> i32 {
        self.reli().set_last_frame2(
            ReliLastFrame::SubManager as u32,
            UnitType::UnitSocket as u32,
        );
        self.rentry().set_last_frame(SocketReFrame::FdListen(true));
        self.reli()
            .set_last_unit(&self.mng().comm.owner().unwrap().id());
        let ret = self.dispatch_io().map_err(|_| event::Error::Other {
            word: "Dispatch IO failed!",
        });
        self.reli().clear_last_unit();
        self.rentry().clear_last_frame();
        self.reli().clear_last_frame();
        ret.unwrap_or(-1)
    }

    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }
}

// the declaration "pub(self)" is for identification only.
impl SocketMngPort {
    pub(crate) fn new(mng: &Rc<SocketMng>, port: Rc<SocketPort>) -> SocketMngPort {
        SocketMngPort {
            mng: Rc::downgrade(mng),
            port,
        }
    }

    fn dispatch_io(&self) -> Result<i32> {
        let mut afd: i32 = -1;

        if self.mng().state() != SocketState::Listening {
            return Ok(0);
        }

        if self.mng().config.config_data().borrow().Socket.Accept
            && self.port.p_type() == PortType::Socket
            && self.port.can_accept()
        {
            afd = self.port.accept().map_err(|_e| Error::Other {
                msg: "accept err".to_string(),
            })?;

            self.port.apply_sock_opt(afd);
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

    fn mng(&self) -> Rc<SocketMng> {
        self.mng.clone().upgrade().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::SocketState;
    use core::unit::UnitActiveState;
    #[test]
    fn test_socket_active_state() {
        assert_eq!(
            SocketState::Dead.to_unit_active_state(),
            UnitActiveState::InActive
        );
        assert_eq!(
            SocketState::StartPre.to_unit_active_state(),
            UnitActiveState::Activating
        );
        assert_eq!(
            SocketState::StartChown.to_unit_active_state(),
            UnitActiveState::Activating
        );
        assert_eq!(
            SocketState::StartPost.to_unit_active_state(),
            UnitActiveState::Activating
        );
        assert_eq!(
            SocketState::Listening.to_unit_active_state(),
            UnitActiveState::Active
        );
        assert_eq!(
            SocketState::Running.to_unit_active_state(),
            UnitActiveState::Active
        );
        assert_eq!(
            SocketState::StopPre.to_unit_active_state(),
            UnitActiveState::DeActivating
        );
        assert_eq!(
            SocketState::StopPreSigterm.to_unit_active_state(),
            UnitActiveState::DeActivating
        );
        assert_eq!(
            SocketState::StopPost.to_unit_active_state(),
            UnitActiveState::DeActivating
        );
        assert_eq!(
            SocketState::StopPreSigkill.to_unit_active_state(),
            UnitActiveState::DeActivating
        );
        assert_eq!(
            SocketState::FinalSigterm.to_unit_active_state(),
            UnitActiveState::DeActivating
        );
        assert_eq!(
            SocketState::FinalSigterm.to_unit_active_state(),
            UnitActiveState::DeActivating
        );
        assert_eq!(
            SocketState::Failed.to_unit_active_state(),
            UnitActiveState::Failed
        );
        assert_eq!(
            SocketState::Cleaning.to_unit_active_state(),
            UnitActiveState::Maintenance
        );
    }
}
