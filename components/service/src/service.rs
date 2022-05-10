use process1::manager::{
    KillOperation, Unit, UnitActiveState, UnitManager, UnitMngUtil, UnitObj, UnitSubClass,
};
use process1::watchdog;
use std::any::{Any, TypeId};
use std::collections::hash_map::DefaultHasher;
use std::collections::LinkedList;
use std::error::Error;
use std::hash::{Hash, Hasher};
use utils::unit_conf::{Conf, Section};

use super::service_base::{
    CommandLine, ExitStatusSet, ServiceCommand, ServiceRestart, ServiceResult, ServiceState,
    ServiceType,
};
use super::service_start;
use log;
use nix::errno::Errno;
use nix::sys::signal::Signal;
use nix::unistd::Pid;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use utils::logger;
use utils::IN_SET;

use process1::manager::UnitActionError;

const LOG_LEVEL: u32 = 4;
const PLUGIN_NAME: &str = "ServiceUnit";
#[allow(dead_code)]
#[derive(Default)]
pub struct ServiceUnit {
    pub unit: Option<Weak<Unit>>,
    pub um: Option<Weak<UnitManager>>,
    pub(crate) service_type: ServiceType,
    state: ServiceState,
    restart: ServiceRestart,
    restart_prevent_status: ExitStatusSet,
    restart_force_status: ExitStatusSet,
    success_status: ExitStatusSet,
    pid_file: String,
    restart_usec: u64,
    watchdog_usec: u64,
    watchdog_original_usec: u64,
    watchdog_override_usec: u64,
    watchdog_override_enable: bool,
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
    pub fn new() -> Self {
        Self {
            unit: None,
            um: None,
            service_type: ServiceType::ServiceTypeInvalid,
            state: ServiceState::ServiceStateMax,
            restart: ServiceRestart::ServiceRestartInvalid,
            restart_prevent_status: ExitStatusSet {},
            restart_force_status: ExitStatusSet {},
            success_status: ExitStatusSet {},
            pid_file: String::from(""),
            restart_usec: 0,
            watchdog_usec: 0,
            watchdog_original_usec: u64::MAX,
            watchdog_override_usec: 0,
            watchdog_override_enable: false,
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

    pub fn start(&mut self) {
        log::debug!("enter running service condition command");
        let cmds = self.exec_commands[ServiceCommand::ServiceCondition as usize].clone();
        let mut cmd = cmds.iter();

        match cmd.next() {
            Some(cmd) => {
                self.control_command = Some(cmd.clone());
                match service_start::start_service(self, &*cmd.borrow()) {
                    Ok(pid) => self.control_pid = Some(pid),
                    Err(_e) => {
                        self.run_dead(ServiceResult::ServiceFailureResources);
                    }
                }
                self.set_state(ServiceState::ServiceCondition);
            }
            None => {
                self.run_prestart();
            }
        }
    }

    fn run_prestart(&mut self) {
        log::debug!("enter running service prestart command");
        let cmds = self.exec_commands[ServiceCommand::ServiceStartPre as usize].clone();
        let mut cmd = cmds.iter();

        self.unwatch_control_pid();
        match cmd.next() {
            Some(cmd) => {
                self.control_command = Some(cmd.clone());

                match service_start::start_service(self, &*cmd.borrow()) {
                    Ok(pid) => self.control_pid = Some(pid),
                    Err(_e) => {
                        self.run_dead(ServiceResult::ServiceFailureResources);
                    }
                }
                self.set_state(ServiceState::ServiceStartPre);
            }
            None => self.run_start(),
        }
    }

    fn unwatch_control_pid(&mut self) {
        match self.control_pid {
            Some(pid) => {
                self.um
                    .as_ref()
                    .cloned()
                    .unwrap()
                    .upgrade()
                    .as_ref()
                    .cloned()
                    .unwrap()
                    .child_unwatch_pid(pid);
                self.control_pid = None;
            }
            None => {}
        }
    }

    fn unwatch_main_pid(&mut self) {
        match self.main_pid {
            Some(pid) => {
                self.um
                    .as_ref()
                    .cloned()
                    .unwrap()
                    .upgrade()
                    .as_ref()
                    .cloned()
                    .unwrap()
                    .child_unwatch_pid(pid);
                self.main_pid = None;
            }
            None => {}
        }
    }

    fn run_next_control(&mut self) {
        log::debug!("runing next control command");
        if let Some(control_command) = &self.control_command {
            if let Some(cmd) = &control_command.clone().borrow().next {
                self.control_command = Some(cmd.clone());
                match service_start::start_service(self, &*cmd.borrow()) {
                    Ok(pid) => {
                        self.control_pid = Some(pid);
                    }
                    Err(_e) => {
                        log::error!(
                            "failed to start service: {}",
                            self.unit
                                .as_ref()
                                .cloned()
                                .unwrap()
                                .upgrade()
                                .as_ref()
                                .cloned()
                                .unwrap()
                                .get_id()
                        );
                    }
                }
            }
        }
    }

    fn run_next_main(&mut self) {
        if let Some(main_command) = &self.main_command {
            if let Some(cmd) = &main_command.clone().borrow().next {
                self.main_command = Some(cmd.clone());
                match service_start::start_service(self, &*cmd.borrow()) {
                    Ok(pid) => {
                        self.main_pid = Some(pid);
                    }
                    Err(_e) => {
                        log::error!(
                            "failed to run main command: {}",
                            self.unit
                                .as_ref()
                                .cloned()
                                .unwrap()
                                .upgrade()
                                .as_ref()
                                .cloned()
                                .unwrap()
                                .get_id()
                        );
                    }
                }
            }
        }
    }

    fn set_state(&mut self, state: ServiceState) {
        let original_state = self.state;
        self.state = state;

        // TODO
        // check the new state
        if !vec![
            ServiceState::ServiceStart,
            ServiceState::ServiceStartPost,
            ServiceState::ServiceRuning,
            ServiceState::ServiceReload,
            ServiceState::ServiceStop,
            ServiceState::ServiceStopWatchdog,
            ServiceState::ServiceStopSigterm,
            ServiceState::ServiceStopSigkill,
            ServiceState::ServiceStopPost,
            ServiceState::ServiceFinalWatchdog,
            ServiceState::ServiceFinalSigterm,
            ServiceState::ServiceFinalSigkill,
        ]
        .contains(&state)
        {
            self.unwatch_main_pid();
            self.main_command = None;
        }

        if !vec![
            ServiceState::ServiceCondition,
            ServiceState::ServiceStartPre,
            ServiceState::ServiceStart,
            ServiceState::ServiceStartPost,
            ServiceState::ServiceReload,
            ServiceState::ServiceStop,
            ServiceState::ServiceStopWatchdog,
            ServiceState::ServiceStopSigterm,
            ServiceState::ServiceStopSigkill,
            ServiceState::ServiceStopPost,
            ServiceState::ServiceFinalWatchdog,
            ServiceState::ServiceFinalSigterm,
            ServiceState::ServiceFinalSigkill,
            ServiceState::ServiceCleaning,
        ]
        .contains(&state)
        {
            self.unwatch_control_pid();
            self.control_command = None;
        }

        log::debug!(
            "original state: {:?}, change to: {:?}",
            original_state,
            state
        );
        // todo!()
        // trigger the unit the dependency trigger_by

        self.unit
            .as_ref()
            .cloned()
            .unwrap()
            .upgrade()
            .as_ref()
            .cloned()
            .unwrap()
            .notify(
                self.trans_to_active_state(original_state),
                self.trans_to_active_state(state),
                0,
            );
    }

    fn run_start(&mut self) {
        log::debug!("enter running service start command");

        self.control_command = None;
        let cmds = self.exec_commands[ServiceCommand::ServiceStart as usize].clone();
        let mut cmd = cmds.iter();

        self.unwatch_control_pid();
        self.unwatch_main_pid();
        match cmd.next() {
            Some(cmd) => {
                self.main_command = Some(cmd.clone());

                match service_start::start_service(self, &*cmd.borrow()) {
                    Ok(pid) => self.main_pid = Some(pid),
                    Err(_e) => {
                        log::error!(
                            "failed to start service: {}",
                            self.unit
                                .as_ref()
                                .cloned()
                                .unwrap()
                                .upgrade()
                                .as_ref()
                                .cloned()
                                .unwrap()
                                .get_id()
                        );
                        self.send_signal(
                            ServiceState::ServiceStopSigterm,
                            ServiceResult::ServiceFailureResources,
                        );
                    }
                }
                self.run_start_post();
                // self.set_state(ServiceState::ServiceStart);
            }
            None => {
                self.run_start_post();
            }
        }
    }

    fn run_start_post(&mut self) {
        log::debug!("enter running service startpost command");
        let cmds = self.exec_commands[ServiceCommand::ServiceStartPost as usize].clone();
        let mut cmd = cmds.iter();

        self.unwatch_control_pid();
        match cmd.next() {
            Some(cmd) => {
                self.control_command = Some(cmd.clone());

                match service_start::start_service(self, &*cmd.borrow()) {
                    Ok(pid) => self.control_pid = Some(pid),
                    Err(_e) => {
                        log::error!(
                            "Failed to run start post service: {}",
                            self.unit
                                .as_ref()
                                .cloned()
                                .unwrap()
                                .upgrade()
                                .as_ref()
                                .cloned()
                                .unwrap()
                                .get_id()
                        );
                    }
                }
                self.set_state(ServiceState::ServiceStartPost);
            }
            None => self.enter_running(ServiceResult::ServiceSuccess),
        }
    }

    fn enter_running(&mut self, sr: ServiceResult) {
        self.unwatch_control_pid();
        if self.result == ServiceResult::ServiceSuccess {
            self.result = sr;
        }

        if self.result != ServiceResult::ServiceSuccess {
            self.send_signal(ServiceState::ServiceStopSigterm, sr);
        } else if self.service_alive() {
            self.set_state(ServiceState::ServiceRuning);
        } else {
            self.run_stop(sr);
        }
    }

    fn service_alive(&mut self) -> bool {
        // todo!()
        true
    }

    fn send_signal(&mut self, state: ServiceState, res: ServiceResult) {
        log::debug!(
            "Sending signal of state: {:?}, service result: {:?}",
            state,
            res
        );
        let operation = state.to_kill_operation();

        match self.kill_service(operation) {
            Ok(_) => {}
            Err(_e) => {
                if IN_SET!(
                    state,
                    ServiceState::ServiceStopWatchdog,
                    ServiceState::ServiceStopSigterm,
                    ServiceState::ServiceStopSigkill
                ) {
                    return self.run_stop_post(ServiceResult::ServiceFailureResources);
                } else {
                    return self.run_dead(ServiceResult::ServiceSuccess);
                }
            }
        }

        if vec![
            ServiceState::ServiceStopWatchdog,
            ServiceState::ServiceStopSigterm,
            ServiceState::ServiceStopSigkill,
        ]
        .contains(&state)
        {
            self.run_stop_post(ServiceResult::ServiceSuccess);
        } else if vec![
            ServiceState::ServiceFinalWatchdog,
            ServiceState::ServiceFinalSigterm,
        ]
        .contains(&state)
        {
            self.send_signal(
                ServiceState::ServiceFinalSigkill,
                ServiceResult::ServiceSuccess,
            );
        } else {
            self.run_dead(ServiceResult::ServiceSuccess);
        }
    }

    pub fn run_stop(&mut self, res: ServiceResult) {
        log::debug!("enter running stop command");
        if self.result == ServiceResult::ServiceSuccess {
            self.result = res;
        }

        let cmds = self.exec_commands[ServiceCommand::ServiceStop as usize].clone();
        let mut cmd = cmds.iter();

        match cmd.next() {
            Some(cmd) => {
                self.control_command = Some(cmd.clone());

                match service_start::start_service(self, &*cmd.borrow()) {
                    Ok(pid) => self.control_pid = Some(pid),
                    Err(_e) => {
                        log::error!(
                            "Failed to run stop service: {}",
                            self.unit
                                .as_ref()
                                .cloned()
                                .unwrap()
                                .upgrade()
                                .as_ref()
                                .cloned()
                                .unwrap()
                                .get_id()
                        );
                    }
                }
                self.set_state(ServiceState::ServiceStop);
            }
            None => {
                self.send_signal(
                    ServiceState::ServiceStopSigterm,
                    ServiceResult::ServiceSuccess,
                );
            }
        }
    }

    pub fn run_stop_post(&mut self, res: ServiceResult) {
        log::debug!("runing stop post, service result: {:?}", res);
        if self.result == ServiceResult::ServiceSuccess {
            self.result = res;
        }

        let cmds = self.exec_commands[ServiceCommand::ServiceStopPost as usize].clone();
        let mut cmd = cmds.iter();

        match cmd.next() {
            Some(cmd) => {
                self.control_command = Some(cmd.clone());

                match service_start::start_service(self, &*cmd.borrow()) {
                    Ok(pid) => self.control_pid = Some(pid),
                    Err(_e) => {
                        self.send_signal(
                            ServiceState::ServiceFinalSigterm,
                            ServiceResult::ServiceFailureResources,
                        );
                        log::error!(
                            "Failed to run stop service: {}",
                            self.unit
                                .as_ref()
                                .cloned()
                                .unwrap()
                                .upgrade()
                                .as_ref()
                                .cloned()
                                .unwrap()
                                .get_id()
                        );
                    }
                }
                self.set_state(ServiceState::ServiceStopPost);
            }
            None => {
                self.send_signal(
                    ServiceState::ServiceFinalSigterm,
                    ServiceResult::ServiceSuccess,
                );
            }
        }
    }

    fn run_dead(&mut self, res: ServiceResult) {
        log::debug!("Running into dead state, res: {:?}", res);
        if self.result == ServiceResult::ServiceSuccess {
            self.result = res;
        }

        let state = if self.result == ServiceResult::ServiceSuccess {
            ServiceState::ServiceDead
        } else {
            ServiceState::ServiceFailed
        };

        self.set_state(state);
    }

    fn run_reload(&mut self) {
        log::debug!("running service reload command");
        self.control_command = None;
        let cmds = self.exec_commands[ServiceCommand::ServiceReload as usize].clone();
        let mut cmd = cmds.iter();

        self.unwatch_control_pid();
        match cmd.next() {
            Some(cmd) => {
                self.control_command = Some(cmd.clone());

                match service_start::start_service(self, &*cmd.borrow()) {
                    Ok(pid) => self.control_pid = Some(pid),
                    Err(_e) => {
                        log::error!(
                            "failed to start service: {}",
                            self.unit
                                .as_ref()
                                .cloned()
                                .unwrap()
                                .upgrade()
                                .as_ref()
                                .cloned()
                                .unwrap()
                                .get_id()
                        );
                        self.enter_running(ServiceResult::ServiceSuccess);
                    }
                }
                self.set_state(ServiceState::ServiceReload);
            }
            None => {
                self.enter_running(ServiceResult::ServiceSuccess);
            }
        }
    }

    fn kill_service(&mut self, operation: KillOperation) -> Result<(), Errno> {
        let sig = operation.to_signal();
        if self.main_pid.is_some() {
            match nix::sys::signal::kill(self.main_pid.unwrap(), sig) {
                Ok(_) => {
                    if sig != Signal::SIGCONT && sig != Signal::SIGKILL {
                        match nix::sys::signal::kill(self.main_pid.unwrap(), Signal::SIGCONT) {
                            Ok(_) => {}
                            Err(e) => {
                                log::debug!("kill pid {} errno: {}", self.main_pid.unwrap(), e)
                            }
                        }
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
                        match nix::sys::signal::kill(self.control_pid.unwrap(), Signal::SIGCONT) {
                            Ok(_) => {}
                            Err(e) => {
                                log::debug!("kill pid {} errno: {}", self.control_pid.unwrap(), e)
                            }
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Failed to kill control service: error: {}", e);
                }
            }
        }

        Ok(())
    }

    fn current_active_state(&self) -> UnitActiveState {
        if self.service_type == ServiceType::ServiceIdle {
            return self.state.to_unit_active_state_idle();
        }

        self.state.to_unit_active_state()
    }

    fn trans_to_active_state(&self, state: ServiceState) -> UnitActiveState {
        if self.service_type == ServiceType::ServiceIdle {
            return state.to_unit_active_state_idle();
        }

        state.to_unit_active_state()
    }
}

impl ServiceUnit {
    fn sigchld_event(&mut self, pid: Pid, code: i32, status: Signal) {
        log::debug!(
            "ServiceUnit sigchld exit, pid: {:?} code:{}, status:{}",
            pid,
            code,
            status
        );
        log::debug!(
            "main_pid: {:?}, control_pid: {:?}, state: {:?}",
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
                self.run_next_main();
            } else {
                self.main_command = None;
                match self.state {
                    ServiceState::ServiceDead => todo!(),
                    ServiceState::ServiceStart => {
                        self.send_signal(ServiceState::ServiceStopSigterm, res);
                    }

                    ServiceState::ServiceStartPost | ServiceState::ServiceReload => {
                        self.run_stop(res);
                    }
                    ServiceState::ServiceRuning => {
                        self.enter_running(res);
                    }
                    ServiceState::ServiceStop => {}
                    ServiceState::ServiceStopWatchdog
                    | ServiceState::ServiceStopSigkill
                    | ServiceState::ServiceStopSigterm => {
                        self.run_stop_post(res);
                    }
                    ServiceState::ServiceFinalSigterm | ServiceState::ServiceFinalSigkill => {
                        self.run_dead(res);
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
                self.run_next_control();
            } else {
                self.control_command = None;
                match self.state {
                    ServiceState::ServiceCondition => {
                        if res == ServiceResult::ServiceSuccess {
                            self.run_prestart();
                        } else {
                            self.send_signal(ServiceState::ServiceStopSigterm, res);
                        }
                    }
                    ServiceState::ServiceStartPre => {
                        if res == ServiceResult::ServiceSuccess {
                            self.run_start();
                        } else {
                            self.send_signal(ServiceState::ServiceStopSigterm, res);
                        }
                    }
                    ServiceState::ServiceStart => {
                        if res == ServiceResult::ServiceSuccess {
                            self.run_start_post();
                        }
                    }
                    ServiceState::ServiceStartPost => {
                        self.enter_running(ServiceResult::ServiceSuccess);
                    }
                    ServiceState::ServiceRuning => todo!(),
                    ServiceState::ServiceReload => {
                        self.enter_running(res);
                    }
                    ServiceState::ServiceStop => {
                        self.send_signal(ServiceState::ServiceStopSigterm, res);
                    }
                    ServiceState::ServiceStopSigterm
                    | ServiceState::ServiceStopSigkill
                    | ServiceState::ServiceStopWatchdog => {
                        self.run_stop_post(res);
                    }
                    ServiceState::ServiceStopPost => {
                        self.send_signal(ServiceState::ServiceFinalSigterm, res);
                    }
                    ServiceState::ServiceFinalSigterm | ServiceState::ServiceFinalSigkill => {
                        self.run_dead(res);
                    }
                    _ => {}
                }
            }
        }
    }
}

impl ServiceUnit {
    /// 软件看门狗，在service中的watchdog主要是定期接收服务进程发来的READY=1的消息，如果没收到则执行杀死或重启操作。
    /// 打开看门狗，需要比较原有的超时时间和复写的超时时间，并判断如果是非法值则要关闭看门狗
    /// 直接调用recvmsg系统调用从socket文件中读取字符串，再判断是否是看门狗相关的字段，如READY=1
    /// 功能未完全实现，依赖timer sd-event的实现
    pub fn start_watchdog(self) {
        // 允许覆盖timeout则使用覆盖值
        let watchdog_usec = if self.watchdog_override_enable {
            self.watchdog_override_usec
        } else {
            self.watchdog_original_usec
        };
        // timeout为0则关闭看门狗
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
    fn load(&mut self, section: &Section<Conf>) -> Result<(), Box<dyn Error>> {
        self.parse(section)?;

        self.service_add_extras();

        return self.service_verify();
    }
    fn coldplug(&self) {
        todo!()
    }
    fn start(&mut self) -> Result<(), UnitActionError> {
        log::debug!("begin to start the service unit");
        if IN_SET!(
            self.state,
            ServiceState::ServiceStop,
            ServiceState::ServiceStopWatchdog,
            ServiceState::ServiceStopSigterm,
            ServiceState::ServiceStopSigkill,
            ServiceState::ServiceStopPost,
            ServiceState::ServiceFinalWatchdog,
            ServiceState::ServiceFinalSigterm,
            ServiceState::ServiceFinalSigkill,
            ServiceState::ServiceCleaning
        ) {
            return Err(UnitActionError::UnitActionEAgain);
        }

        self.result = ServiceResult::ServiceSuccess;
        self.forbid_restart = false;

        self.watchdog_original_usec = self.watchdog_usec;
        self.watchdog_override_enable = false;
        self.watchdog_override_usec = u64::MAX;

        self.start();

        Ok(())
    }
    fn dump(&self) {
        todo!()
    }
    fn stop(&mut self) -> Result<(), UnitActionError> {
        self.forbid_restart = true;
        let stop_state = vec![
            ServiceState::ServiceStop,
            ServiceState::ServiceStopSigterm,
            ServiceState::ServiceStopSigkill,
            ServiceState::ServiceStopPost,
        ];

        if stop_state.contains(&self.state) {
            return Ok(());
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
                ServiceState::ServiceStopSigterm,
                ServiceResult::ServiceSuccess,
            );
            return Ok(());
        }

        self.run_stop(ServiceResult::ServiceSuccess);

        Ok(())
    }
    fn reload(&mut self) {
        self.run_reload();
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
    fn sigchld_events(&mut self, pid: Pid, code: i32, status: Signal) {
        self.sigchld_event(pid, code, status)
    }
    fn reset_failed(&self) {
        todo!()
    }

    fn eq(&self, other: &dyn UnitObj) -> bool {
        if let Some(other) = other.as_any().downcast_ref::<ServiceUnit>() {
            return self
                .unit
                .as_ref()
                .cloned()
                .unwrap()
                .upgrade()
                .as_ref()
                .cloned()
                .unwrap()
                == other
                    .unit
                    .as_ref()
                    .cloned()
                    .unwrap()
                    .upgrade()
                    .as_ref()
                    .cloned()
                    .unwrap();
        }
        false
    }

    fn hash(&self) -> u64 {
        let mut h = DefaultHasher::new();
        Hash::hash(&(TypeId::of::<ServiceUnit>()), &mut h);
        h.write(
            self.unit
                .as_ref()
                .cloned()
                .unwrap()
                .upgrade()
                .as_ref()
                .cloned()
                .unwrap()
                .get_id()
                .as_bytes(),
        );
        h.finish()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn get_private_conf_section_name(&self) -> Option<&str> {
        Some("Service")
    }

    fn current_active_state(&self) -> UnitActiveState {
        self.current_active_state()
    }

    fn attach_unit(&mut self, unit: Rc<Unit>) {
        self.unit = Some(Rc::downgrade(&unit));
    }
}

impl UnitMngUtil for ServiceUnit {
    fn attach(&mut self, um: Rc<UnitManager>) {
        self.um = Some(Rc::downgrade(&um));
    }
}

impl UnitSubClass for ServiceUnit {
    fn into_unitobj(self: Box<Self>) -> Box<dyn UnitObj> {
        Box::new(*self)
    }
}

use process1::declure_unitobj_plugin;
declure_unitobj_plugin!(ServiceUnit, ServiceUnit::default, PLUGIN_NAME, LOG_LEVEL);
