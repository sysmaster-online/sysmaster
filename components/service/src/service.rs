use process1::manager::unit::unit_manager::UnitManager;
use process1::manager::unit::{KillOperation, Unit, UnitActiveState, UnitObj};
use process1::watchdog;
use std::any::{Any, TypeId};
use std::collections::hash_map::DefaultHasher;
use std::collections::LinkedList;
use std::error::Error;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::io::{Error as IOError, ErrorKind};
use std::str::FromStr;

use super::service_start;
use nix::errno::Errno;
use nix::sys::signal::Signal;
use nix::unistd::Pid;
use std::cell::RefCell;
use std::path::Path;
use std::rc::{Rc, Weak};

#[derive(PartialEq, Default, Debug)]
struct ExitStatusSet {}

#[derive(PartialEq, EnumString, Display, Debug)]
enum ServiceTimeoutFailureMode {
    #[strum(serialize = "terminate")]
    ServiceTimeoutTerminate,
    #[strum(serialize = "abort")]
    ServiceTimeoutAbort,
    #[strum(serialize = "kill")]
    ServiceTimeoutKill,
    ServiceTimeoutFailureModeMax,
    ServiceTimeoutFailureModeInvalid = -1,
}

impl Default for ServiceTimeoutFailureMode {
    fn default() -> Self {
        ServiceTimeoutFailureMode::ServiceTimeoutTerminate
    }
}

#[derive(PartialEq, EnumString, Display, Debug)]
enum ServiceRestart {
    #[strum(serialize = "no")]
    ServiceRestartNo,
    #[strum(serialize = "on-success")]
    ServiceRestartOnSuccess,
    #[strum(serialize = "on-failure")]
    ServiceRestartOnFailure,
    #[strum(serialize = "on-abnormal")]
    ServiceRestartOnAbnormal,
    #[strum(serialize = "on-abort")]
    ServiceRestartOnAbort,
    #[strum(serialize = "always")]
    ServiceRestartAlways,
    ServiceRestartMax,
    ServiceRestartInvalid = -1,
}

impl Default for ServiceRestart {
    fn default() -> Self {
        ServiceRestart::ServiceRestartNo
    }
}

#[derive(PartialEq, Eq, EnumString, Display, Debug)]
pub(crate) enum ServiceType {
    #[strum(serialize = "simple")]
    ServiceSimple,
    #[strum(serialize = "forking")]
    SserviceForking,
    #[strum(serialize = "oneshot")]
    ServiceOneshot,
    #[strum(serialize = "dbus")]
    ServiceDbus,
    #[strum(serialize = "notify")]
    ServiceNotify,
    #[strum(serialize = "idle")]
    SserviceIdle,
    #[strum(serialize = "exec")]
    ServiceExec,
    ServiceTypeMax,
    ServiceTypeInvalid = -1,
}

impl Default for ServiceType {
    fn default() -> Self {
        ServiceType::ServiceSimple
    }
}
pub enum ServiceCommand {
    ServiceCondition,
    ServiceStartPre,
    ServiceStart,
    ServiceStartPost,
    ServiceReload,
    ServiceStop,
    ServiceStopPost,
    ServiceCommandMax,
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum ServiceResult {
    ServiceSuccess,
    ServiceFailureResources,
    ServiceFailureTimeout,
    ServiceFailureSignal,
    ServiceFailureKill,
    ServiceResultInvalid,
}

impl Default for ServiceResult {
    fn default() -> Self {
        ServiceResult::ServiceResultInvalid
    }
}

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum ServiceState {
    ServiceDead,
    ServiceCondition,
    ServiceStartPre,
    ServiceStart,
    ServiceStartPost,
    ServiceRuning,
    ServiceExited,
    ServiceReload,
    ServiceStop,
    ServiceStopWatchdog,
    ServiceStopPost,
    ServiceStopSigterm,
    ServiceStopSigkill,
    ServiceFinalWatchdog,
    ServiceFinalSigterm,
    ServiceFinalSigkill,
    ServiceFailed,
    ServiceStateMax,
}

impl Default for ServiceState {
    fn default() -> Self {
        ServiceState::ServiceStateMax
    }
}

impl ServiceState {
    fn to_unit_active_state(&self) -> UnitActiveState {
        match *self {
            ServiceState::ServiceDead => UnitActiveState::UnitInactive,
            ServiceState::ServiceCondition
            | ServiceState::ServiceStartPre
            | ServiceState::ServiceStart
            | ServiceState::ServiceStartPost => UnitActiveState::UnitActivating,
            ServiceState::ServiceRuning | ServiceState::ServiceExited => {
                UnitActiveState::UnitActive
            }
            ServiceState::ServiceReload => UnitActiveState::UnitReloading,
            ServiceState::ServiceStop
            | ServiceState::ServiceStopWatchdog
            | ServiceState::ServiceStopPost
            | ServiceState::ServiceStopSigterm
            | ServiceState::ServiceStopSigkill
            | ServiceState::ServiceStateMax
            | ServiceState::ServiceFinalSigterm
            | ServiceState::ServiceFinalSigkill
            | ServiceState::ServiceFinalWatchdog => UnitActiveState::UnitDeactiviting,
            ServiceState::ServiceFailed => UnitActiveState::UnitFailed,
        }
    }

    fn to_kill_operation(&self) -> KillOperation {
        match self {
            ServiceState::ServiceStopWatchdog => KillOperation::KillWatchdog,
            ServiceState::ServiceStopSigterm | ServiceState::ServiceFinalSigterm => {
                KillOperation::KillTerminate
            }
            ServiceState::ServiceStopSigkill | ServiceState::ServiceFinalSigkill => {
                KillOperation::KillKill
            }
            _ => KillOperation::KillInvalid,
        }
    }
}

pub enum CmdError {
    Timeout,
    NoCmdFound,
    SpawnError,
}

pub enum ErrorService {
    ServiceAlreadyStarted(nix::unistd::Pid),
    ServicePreStartFailed(String),
    ServiceStartFailed(String),
    ServicePostStartFailed(String),
    ServiceCommandNotFound,
}

#[derive(PartialEq, Default, Debug)]
struct DualTimestamp {}

#[derive(PartialEq, Clone, Eq, Debug)]
pub struct CommandLine {
    pub cmd: String,
    pub args: Vec<String>,
    pub next: Option<Rc<RefCell<CommandLine>>>,
}

impl CommandLine {
    pub fn update_next(&mut self, next: Rc<RefCell<CommandLine>>) {
        self.next = Some(next)
    }
}

impl fmt::Display for CommandLine {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Display: {}", self.cmd)
    }
}

#[derive(Default, Debug)]
pub struct ServiceUnit {
    pub unit: Weak<Unit>,
    service_type: ServiceType,
    state: ServiceState,
    restart: ServiceRestart,
    restart_prevent_status: ExitStatusSet,
    restart_force_status: ExitStatusSet,
    success_status: ExitStatusSet,
    pid_file: String,
    restart_usec: u64,
    timeout_start_usec: u64,
    timeout_stop_usec: u64,
    timeout_abort_usec: u64,
    timeout_abort_set: bool,
    runtime_max_usec: u64,
    timeout_start_failure_mode: ServiceTimeoutFailureMode,
    timeout_stop_failure_mode: ServiceTimeoutFailureMode,
    watchdog_timestamp: DualTimestamp,
    watchdog_usec: u64,
    watchdog_original_usec: u64,
    watchdog_override_usec: u64,
    watchdog_override_enable: bool,
    socket_fd: isize,
    bus_name: String,
    forbid_restart: bool,
    result: ServiceResult,
    pub main_command: Option<Rc<RefCell<CommandLine>>>,
    pub control_command: Option<Rc<RefCell<CommandLine>>>,
    pub main_pid: Option<nix::unistd::Pid>,
    pub control_pid: Option<nix::unistd::Pid>,
    pub exec_commands:
        [LinkedList<Rc<RefCell<CommandLine>>>; ServiceCommand::ServiceCommandMax as usize],
}

impl ServiceUnit {
    pub fn new(unit: Weak<Unit>) -> Self {
        Self {
            unit,
            service_type: ServiceType::ServiceTypeInvalid,
            state: ServiceState::ServiceStateMax,
            restart: ServiceRestart::ServiceRestartInvalid,
            restart_prevent_status: ExitStatusSet {},
            restart_force_status: ExitStatusSet {},
            success_status: ExitStatusSet {},
            pid_file: String::from(""),
            restart_usec: 0,
            timeout_start_usec: 0,
            timeout_stop_usec: 0,
            timeout_abort_usec: 0,
            timeout_abort_set: false,
            runtime_max_usec: u64::MAX,
            timeout_start_failure_mode: ServiceTimeoutFailureMode::ServiceTimeoutFailureModeInvalid,
            timeout_stop_failure_mode: ServiceTimeoutFailureMode::ServiceTimeoutFailureModeInvalid,
            watchdog_timestamp: DualTimestamp {},
            watchdog_usec: 0,
            watchdog_original_usec: u64::MAX,
            watchdog_override_usec: 0,
            watchdog_override_enable: false,
            socket_fd: -1,
            bus_name: String::from(""),
            exec_commands: Default::default(),
            main_command: None,
            control_command: None,
            main_pid: None,
            control_pid: None,
            forbid_restart: false,
            result: ServiceResult::ServiceSuccess,
        }
    }

    /*pub fn unit_service_load(&mut self, manager: &mut UnitManager) -> Result<(), Box<dyn Error>> {
        return self.unit.load(manager);
    }*/

    pub fn service_add_extras(&mut self) -> bool {
        if self.service_type == ServiceType::ServiceTypeInvalid {
            if !self.bus_name.is_empty() {
                self.service_type = ServiceType::ServiceDbus;
            }
        }
        true
    }

    pub fn service_verify(&self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    /*pub fn get_unit_name(&self) -> String {
        self.unit.id.to_string()
    }*/

    pub fn start(&mut self, m: &mut UnitManager) {
        let cmds = self.exec_commands[ServiceCommand::ServiceCondition as usize].clone();
        let mut cmd = cmds.iter();

        match cmd.next() {
            Some(cmd) => {
                self.control_command = Some(cmd.clone());
                match service_start::start_service(self, m, &*cmd.borrow()) {
                    Ok(pid) => self.control_pid = Some(pid),
                    Err(_e) => {
                        self.run_dead(m, ServiceResult::ServiceFailureResources);
                    }
                }
                self.set_state(m, ServiceState::ServiceCondition);
            }
            None => {
                self.run_prestart(m);
            }
        }
    }

    fn run_prestart(&mut self, m: &mut UnitManager) {
        let cmds = self.exec_commands[ServiceCommand::ServiceStartPre as usize].clone();
        let mut cmd = cmds.iter();

        self.unwatch_control_pid(m);
        match cmd.next() {
            Some(cmd) => {
                self.control_command = Some(cmd.clone());

                match service_start::start_service(self, m, &*cmd.borrow()) {
                    Ok(pid) => self.control_pid = Some(pid),
                    Err(_e) => {
                        self.run_dead(m, ServiceResult::ServiceFailureResources);
                    }
                }
                self.set_state(m, ServiceState::ServiceStartPre);
            }
            None => self.run_start(m),
        }
    }

    fn unwatch_control_pid(&mut self, m: &mut UnitManager) {
        match self.control_pid {
            Some(pid) => m.unwatch_pid(pid),
            None => {}
        }
    }

    fn unwatch_main_pid(&mut self, m: &mut UnitManager) {
        match self.main_pid {
            Some(pid) => m.unwatch_pid(pid),
            None => {}
        }
    }

    fn run_next_control(&mut self, m: &mut UnitManager) {
        log::debug!("runing next control command");
        if let Some(control_command) = &self.control_command {
            if let Some(cmd) = &control_command.clone().borrow().next {
                self.control_command = Some(cmd.clone());
                match service_start::start_service(self, m, &*cmd.borrow()) {
                    Ok(pid) => {
                        self.control_pid = Some(pid);
                    }
                    Err(_e) => {
                        log::error!(
                            "failed to start service: {}",
                            self.unit.upgrade().as_ref().cloned().unwrap().id.clone()
                        );
                    }
                }
            }
        }
    }

    fn run_next_main(&mut self, m: &mut UnitManager) {
        if let Some(main_command) = &self.main_command {
            if let Some(cmd) = &main_command.clone().borrow().next {
                self.main_command = Some(cmd.clone());
                match service_start::start_service(self, m, &*cmd.borrow()) {
                    Ok(pid) => {
                        self.main_pid = Some(pid);
                    }
                    Err(_e) => {
                        log::error!(
                            "failed to run main command: {}",
                            self.unit.upgrade().as_ref().cloned().unwrap().id.clone()
                        );
                    }
                }
            }
        }
    }

    fn set_state(&mut self, m: &mut UnitManager, state: ServiceState) {
        let original_state = self.state;
        self.state = state;

        log::debug!(
            "original state: {:?}, change to: {:?}",
            original_state,
            state
        );
        // todo!()
        // trigger the unit the dependency trigger_by

        self.unit.upgrade().as_ref().cloned().unwrap().notify(
            m,
            original_state.to_unit_active_state(),
            state.to_unit_active_state(),
        );
    }

    fn run_start(&mut self, m: &mut UnitManager) {
        log::debug!("running service start command");
        self.control_command = None;
        let cmds = self.exec_commands[ServiceCommand::ServiceStart as usize].clone();
        let mut cmd = cmds.iter();

        self.unwatch_control_pid(m);
        self.unwatch_main_pid(m);
        match cmd.next() {
            Some(cmd) => {
                self.main_command = Some(cmd.clone());

                match service_start::start_service(self, m, &*cmd.borrow()) {
                    Ok(pid) => self.main_pid = Some(pid),
                    Err(_e) => {
                        log::error!(
                            "failed to start service: {}",
                            self.unit.upgrade().as_ref().cloned().unwrap().id.clone()
                        );
                        self.send_signal(
                            m,
                            ServiceState::ServiceStopSigterm,
                            ServiceResult::ServiceFailureResources,
                        );
                    }
                }
                self.set_state(m, ServiceState::ServiceStart);
            }
            None => {
                self.run_start_post(m);
            }
        }
    }

    fn run_start_post(&mut self, m: &mut UnitManager) {
        log::debug!("running start post command");
        let cmds = self.exec_commands[ServiceCommand::ServiceStartPost as usize].clone();
        let mut cmd = cmds.iter();

        self.unwatch_control_pid(m);
        match cmd.next() {
            Some(cmd) => {
                self.control_command = Some(cmd.clone());

                match service_start::start_service(self, m, &*cmd.borrow()) {
                    Ok(pid) => self.control_pid = Some(pid),
                    Err(_e) => {
                        log::error!(
                            "Failed to run start post service: {}",
                            self.unit.upgrade().as_ref().cloned().unwrap().id.clone()
                        );
                    }
                }
                self.set_state(m, ServiceState::ServiceStartPost);
            }
            None => self.enter_running(m, ServiceResult::ServiceSuccess),
        }
    }

    fn enter_running(&mut self, m: &mut UnitManager, sr: ServiceResult) {
        self.unwatch_control_pid(m);
        if self.result == ServiceResult::ServiceSuccess {
            self.result = sr;
        }

        if self.result != ServiceResult::ServiceSuccess {
            self.send_signal(m, ServiceState::ServiceStopSigterm, sr);
        } else if self.service_alive() {
            self.set_state(m, ServiceState::ServiceRuning);
        } else {
            self.run_stop(m, sr);
        }
    }

    fn service_alive(&mut self) -> bool {
        // todo!()
        true
    }

    fn send_signal(&mut self, m: &mut UnitManager, state: ServiceState, res: ServiceResult) {
        log::debug!(
            "Sending signalsend signal of state: {:?}, service result: {:?}",
            state,
            res
        );
        let operation = state.to_kill_operation();

        self.kill_service(operation);

        if vec![
            ServiceState::ServiceStopWatchdog,
            ServiceState::ServiceStopSigterm,
            ServiceState::ServiceStopSigkill,
        ]
        .contains(&state)
        {
            self.run_stop_post(m, ServiceResult::ServiceSuccess);
        } else if vec![
            ServiceState::ServiceFinalWatchdog,
            ServiceState::ServiceFinalSigterm,
        ]
        .contains(&state)
        {
            self.send_signal(
                m,
                ServiceState::ServiceFinalSigkill,
                ServiceResult::ServiceSuccess,
            );
        } else {
            self.run_dead(m, ServiceResult::ServiceSuccess);
        }

        log::debug!(
            "Sending signal, state: {:?}, service result: {:?}",
            state,
            res
        );
    }

    pub fn run_stop(&mut self, m: &mut UnitManager, res: ServiceResult) {
        if self.result == ServiceResult::ServiceSuccess {
            self.result = res;
        }

        let cmds = self.exec_commands[ServiceCommand::ServiceStop as usize].clone();
        let mut cmd = cmds.iter();

        match cmd.next() {
            Some(cmd) => {
                self.control_command = Some(cmd.clone());

                match service_start::start_service(self, m, &*cmd.borrow()) {
                    Ok(pid) => self.control_pid = Some(pid),
                    Err(_e) => {
                        log::error!(
                            "Failed to run stop service: {}",
                            self.unit.upgrade().as_ref().cloned().unwrap().id.clone()
                        );
                    }
                }
                self.set_state(m, ServiceState::ServiceStop);
            }
            None => {
                self.send_signal(
                    m,
                    ServiceState::ServiceStopSigterm,
                    ServiceResult::ServiceSuccess,
                );
            }
        }
    }

    pub fn run_stop_post(&mut self, m: &mut UnitManager, res: ServiceResult) {
        log::debug!("runing stop post, service result: {:?}", res);
        if self.result == ServiceResult::ServiceSuccess {
            self.result = res;
        }

        let cmds = self.exec_commands[ServiceCommand::ServiceStopPost as usize].clone();
        let mut cmd = cmds.iter();

        match cmd.next() {
            Some(cmd) => {
                self.control_command = Some(cmd.clone());

                match service_start::start_service(self, m, &*cmd.borrow()) {
                    Ok(pid) => self.control_pid = Some(pid),
                    Err(_e) => {
                        self.send_signal(
                            m,
                            ServiceState::ServiceFinalSigterm,
                            ServiceResult::ServiceFailureResources,
                        );
                        log::error!(
                            "Failed to run stop service: {}",
                            self.unit.upgrade().as_ref().cloned().unwrap().id.clone()
                        );
                    }
                }
                self.set_state(m, ServiceState::ServiceStopPost);
            }
            None => {
                self.send_signal(
                    m,
                    ServiceState::ServiceFinalSigterm,
                    ServiceResult::ServiceSuccess,
                );
            }
        }
    }

    fn run_dead(&mut self, m: &mut UnitManager, res: ServiceResult) {
        log::debug!("Running into dead state, res: {:?}", res);
        if self.result == ServiceResult::ServiceSuccess {
            self.result = res;
        }

        let state = if self.result == ServiceResult::ServiceSuccess {
            ServiceState::ServiceDead
        } else {
            ServiceState::ServiceFailed
        };

        self.set_state(m, state);
    }

    fn run_reload(&mut self, m: &mut UnitManager) {
        log::debug!("running service reload command");
        self.control_command = None;
        let cmds = self.exec_commands[ServiceCommand::ServiceReload as usize].clone();
        let mut cmd = cmds.iter();

        self.unwatch_control_pid(m);
        match cmd.next() {
            Some(cmd) => {
                self.control_command = Some(cmd.clone());

                match service_start::start_service(self, m, &*cmd.borrow()) {
                    Ok(pid) => self.control_pid = Some(pid),
                    Err(_e) => {
                        log::error!(
                            "failed to start service: {}",
                            self.unit.upgrade().as_ref().cloned().unwrap().id.clone()
                        );
                        self.enter_running(m, ServiceResult::ServiceSuccess);
                    }
                }
                self.set_state(m, ServiceState::ServiceReload);
            }
            None => {
                self.enter_running(m, ServiceResult::ServiceSuccess);
            }
        }
    }

    fn kill_service(&mut self, operation: KillOperation) -> Result<(), Errno> {
        let sig = operation.to_signal();
        if self.main_pid.is_some() {
            match nix::sys::signal::kill(self.main_pid.unwrap(), sig) {
                Ok(_) => {
                    if sig != Signal::SIGCONT && sig != Signal::SIGKILL {
                        nix::sys::signal::kill(self.main_pid.unwrap(), Signal::SIGCONT);
                    }
                }
                Err(e) => {
                    log::warn!("Failed to kill main service: error: {}", e);
                }
            }
        }

        if self.control_pid.is_some() {
            match nix::sys::signal::kill(self.control_pid.unwrap(), sig) {
                Ok(_) => {
                    if sig != Signal::SIGCONT && sig != Signal::SIGKILL {
                        nix::sys::signal::kill(self.control_pid.unwrap(), Signal::SIGCONT);
                    }
                }
                Err(e) => {
                    log::warn!("Failed to kill control service: error: {}", e);
                }
            }
        }

        Ok(())
    }
}

impl ServiceUnit {
    fn sigchld_event(&mut self, m: &mut UnitManager, pid: Pid, code: i32, status: Signal) {
        log::debug!(
            "ServiceUnit sigchld exit, pid: {:?} code:{}, status:{}",
            pid,
            code,
            status
        );
        log::debug!(
            "main_pid ：{:?}, control_pid: {:?}, state: {:?}",
            self.main_pid,
            self.control_pid,
            self.state
        );
        let res: ServiceResult;
        if code == 0 {
            res = ServiceResult::ServiceSuccess;
        } else if status != Signal::SIGCHLD {
            res = ServiceResult::ServiceFailureSignal;
        } else {
            res = ServiceResult::ServiceSuccess
        }

        if self.main_pid == Some(pid) {
            self.main_pid = None;

            if self.result == ServiceResult::ServiceSuccess {
                self.result = res;
            }

            if self.main_command.is_some()
                && self.main_command.as_ref().unwrap().borrow().next.is_some()
                && res == ServiceResult::ServiceSuccess
            {
                self.run_next_main(m);
            } else {
                self.main_command = None;
                match self.state {
                    ServiceState::ServiceDead => todo!(),
                    ServiceState::ServiceStart => {
                        self.send_signal(m, ServiceState::ServiceStopSigterm, res);
                    }
                    ServiceState::ServiceStartPost | ServiceState::ServiceReload => {
                        self.run_stop(m, res);
                    }
                    ServiceState::ServiceRuning => {
                        self.enter_running(m, res);
                    }
                    ServiceState::ServiceStop => {}
                    ServiceState::ServiceStopWatchdog
                    | ServiceState::ServiceStopSigkill
                    | ServiceState::ServiceStopSigterm => {
                        self.run_stop_post(m, res);
                    }
                    ServiceState::ServiceFinalSigterm | ServiceState::ServiceFinalSigkill => {
                        self.run_dead(m, res);
                    }
                    _ => {}
                }
            }
        } else if self.control_pid == Some(pid) {
            self.control_pid = None;

            if self.control_command.is_some()
                && self
                    .control_command
                    .as_ref()
                    .unwrap()
                    .borrow()
                    .next
                    .is_some()
                && res == ServiceResult::ServiceSuccess
            {
                self.run_next_control(m);
            } else {
                self.control_command = None;
                match self.state {
                    ServiceState::ServiceCondition => {
                        if res == ServiceResult::ServiceSuccess {
                            self.run_prestart(m);
                        } else {
                            self.send_signal(m, ServiceState::ServiceStopSigterm, res);
                        }
                    }
                    ServiceState::ServiceStartPre => {
                        if res == ServiceResult::ServiceSuccess {
                            self.run_start(m);
                        } else {
                            self.send_signal(m, ServiceState::ServiceStopSigterm, res);
                        }
                    }
                    ServiceState::ServiceStart => {
                        if res == ServiceResult::ServiceSuccess {
                            self.run_start_post(m);
                        }
                    }
                    ServiceState::ServiceStartPost => {
                        self.enter_running(m, ServiceResult::ServiceSuccess);
                    }
                    ServiceState::ServiceRuning => todo!(),
                    ServiceState::ServiceReload => {
                        self.enter_running(m, res);
                    }
                    ServiceState::ServiceStop => {
                        self.send_signal(m, ServiceState::ServiceStopSigterm, res);
                    }
                    ServiceState::ServiceStopSigterm
                    | ServiceState::ServiceStopSigkill
                    | ServiceState::ServiceStopWatchdog => {
                        self.run_stop_post(m, res);
                    }
                    ServiceState::ServiceStopPost => {
                        self.send_signal(m, ServiceState::ServiceFinalSigterm, res);
                    }
                    ServiceState::ServiceFinalSigterm | ServiceState::ServiceFinalSigkill => {
                        self.run_dead(m, res);
                    }
                    _ => {}
                }
            }
        }
    }
}

impl ServiceUnit {
    pub fn start_watchdog(self) {
        let watchdog_usec = if self.watchdog_override_enable {
            self.watchdog_override_usec
        } else {
            self.watchdog_original_usec
        };
        if watchdog_usec == 0 || watchdog_usec == u64::MAX {
            self.stop_watchdog()
        }
        watchdog::register_timer();
        watchdog::event_source_set_enabled(true);
    }

    pub fn stop_watchdog(self) {
        watchdog::event_source_set_enabled(false);
    }
}

impl UnitObj for ServiceUnit {
    fn init(&self) {
        todo!()
    }
    fn done(&self) {
        todo!()
    }
    fn load(&mut self, m: &mut UnitManager) -> Result<(), Box<dyn Error>> {
        self.unit
            .upgrade()
            .as_ref()
            .cloned()
            .unwrap()
            .load
            .unit_load(m);

        self.parse(m)?;

        self.service_add_extras();

        return self.service_verify();
    }
    fn coldplug(&self) {
        todo!()
    }
    fn start(&mut self, m: &mut UnitManager) {
        self.start(m);
    }
    fn dump(&self) {
        todo!()
    }
    fn stop(&mut self, m: &mut UnitManager) {
        self.forbid_restart = true;
        let stop_state = vec![
            ServiceState::ServiceStop,
            ServiceState::ServiceStopSigterm,
            ServiceState::ServiceStopSigkill,
            ServiceState::ServiceStopPost,
        ];

        if stop_state.contains(&self.state) {
            return;
        }

        let starting_state = vec![
            ServiceState::ServiceCondition,
            ServiceState::ServiceStartPre,
            ServiceState::ServiceStart,
            ServiceState::ServiceStartPost,
            ServiceState::ServiceReload,
            ServiceState::ServiceStopWatchdog,
        ];
        if starting_state.contains(&self.state) {
            self.send_signal(
                m,
                ServiceState::ServiceStopSigterm,
                ServiceResult::ServiceSuccess,
            );
            return;
        }

        self.run_stop(m, ServiceResult::ServiceSuccess);
    }
    fn reload(&mut self, m: &mut UnitManager) {
        self.run_reload(m);
    }
    fn kill(&self) {
        todo!()
    }
    fn check_gc(&self) -> bool {
        todo!()
    }
    fn release_resources(&self) {
        todo!()
    }
    fn check_snapshot(&self) {
        todo!()
    }
    fn sigchld_events(&mut self, m: &mut UnitManager, pid: Pid, code: i32, status: Signal) {
        self.sigchld_event(m, pid, code, status)
    }
    fn reset_failed(&self) {
        todo!()
    }

    fn eq(&self, other: &dyn UnitObj) -> bool {
        if let Some(other) = other.as_any().downcast_ref::<ServiceUnit>() {
            return self.unit.upgrade().as_ref().cloned().unwrap()
                == other.unit.upgrade().as_ref().cloned().unwrap();
        }
        false
    }

    fn hash(&self) -> u64 {
        let mut h = DefaultHasher::new();
        Hash::hash(&(TypeId::of::<ServiceUnit>()), &mut h);
        h.write(self.unit.upgrade().as_ref().cloned().unwrap().id.as_bytes());
        h.finish()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn in_load_queue(&self) -> bool {
        self.unit
            .upgrade()
            .as_ref()
            .cloned()
            .unwrap()
            .load
            .in_load_queue()
    }
}

use process1::declure_unitobj_plugin;
declure_unitobj_plugin!(ServiceUnit, ServiceUnit::default);

impl ServiceUnit {
    fn parse(&mut self, manager: &mut UnitManager) -> Result<(), Box<dyn Error>> {
        self.unit
            .upgrade()
            .as_ref()
            .cloned()
            .unwrap()
            .load
            .parse(manager)?;
        let conf = self
            .unit
            .upgrade()
            .as_ref()
            .cloned()
            .unwrap()
            .load
            .get_conf()
            .as_ref()
            .ok_or_else(|| IOError::new(ErrorKind::Other, "config file not loaded"))?
            .clone();

        let service = conf.service.as_ref().unwrap();

        match &service.exec_condition {
            None => {
                self.exec_commands[ServiceCommand::ServiceCondition as usize] = LinkedList::new();
            }
            Some(commands) => {
                match prepare_command(
                    commands,
                    &mut self.exec_commands[ServiceCommand::ServiceCondition as usize],
                ) {
                    Ok(_) => {}
                    Err(e) => return Err(e),
                }
            }
        }

        match &service.exec_prestart {
            None => {
                self.exec_commands[ServiceCommand::ServiceStartPre as usize] = LinkedList::new();
            }
            Some(commands) => {
                match prepare_command(
                    commands,
                    &mut self.exec_commands[ServiceCommand::ServiceStartPre as usize],
                ) {
                    Ok(_) => {}
                    Err(e) => return Err(e),
                }
            }
        }

        match &service.exec_start {
            None => {
                self.exec_commands[ServiceCommand::ServiceStart as usize] = LinkedList::new();
            }
            Some(commands) => {
                match prepare_command(
                    commands,
                    &mut self.exec_commands[ServiceCommand::ServiceStart as usize],
                ) {
                    Ok(_) => {}
                    Err(e) => return Err(e),
                }
            }
        }

        match &service.exec_startpost {
            None => {
                self.exec_commands[ServiceCommand::ServiceStartPost as usize] = LinkedList::new();
            }
            Some(commands) => {
                match prepare_command(
                    commands,
                    &mut self.exec_commands[ServiceCommand::ServiceStartPost as usize],
                ) {
                    Ok(_) => {}
                    Err(e) => return Err(e),
                }
            }
        }

        match &service.exec_reload {
            None => {
                self.exec_commands[ServiceCommand::ServiceReload as usize] = LinkedList::new();
            }
            Some(commands) => {
                match prepare_command(
                    commands,
                    &mut self.exec_commands[ServiceCommand::ServiceReload as usize],
                ) {
                    Ok(_) => {}
                    Err(e) => return Err(e),
                }
            }
        }

        match &service.exec_stop {
            None => {
                self.exec_commands[ServiceCommand::ServiceStop as usize] = LinkedList::new();
            }
            Some(commands) => {
                match prepare_command(
                    commands,
                    &mut self.exec_commands[ServiceCommand::ServiceStop as usize],
                ) {
                    Ok(_) => {}
                    Err(e) => return Err(e),
                }
            }
        }
        match &service.exec_stoppost {
            None => {
                self.exec_commands[ServiceCommand::ServiceStopPost as usize] = LinkedList::new();
            }
            Some(commands) => {
                match prepare_command(
                    commands,
                    &mut self.exec_commands[ServiceCommand::ServiceStopPost as usize],
                ) {
                    Ok(_) => {}
                    Err(e) => return Err(e),
                }
            }
        }

        match &service.restart {
            None => {
                self.restart = ServiceRestart::ServiceRestartNo;
            }
            Some(restart) => {
                self.restart = ServiceRestart::from_str(restart)?;
            }
        }

        match &service.service_type {
            None => {
                self.service_type = ServiceType::ServiceTypeInvalid;
            }
            Some(service_type) => {
                self.service_type = ServiceType::from_str(service_type)?;
            }
        }

        Ok(())
    }
}

fn prepare_command(
    commands: &Vec<String>,
    command_list: &mut LinkedList<Rc<RefCell<CommandLine>>>,
) -> Result<(), Box<dyn Error>> {
    if commands.len() == 0 {
        return Ok(());
    }

    for exec in commands.iter() {
        let mut cmd: Vec<String> = exec
            .trim_end()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        if cmd.is_empty() {
            return Ok(());
        }

        let exec_cmd = cmd.remove(0);
        let path = Path::new(&exec_cmd);
        if !path.exists() || !path.is_file() {
            return Err(format!("{:?} is not exist or commad is not a file", path).into());
        }

        let new_command = Rc::new(RefCell::new(CommandLine {
            cmd: path.to_str().unwrap().to_string(),
            args: cmd,
            next: None,
        }));
        match command_list.back() {
            Some(command) => {
                command.borrow_mut().next = Some(new_command.clone());
            }
            None => {}
        }

        command_list.push_back(new_command.clone());
    }

    Ok(())
}
