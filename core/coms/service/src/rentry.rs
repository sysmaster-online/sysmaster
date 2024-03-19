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
//
#![allow(non_snake_case)]
use crate::monitor::ServiceMonitor;

use basic::fs::parse_pathbuf;
use basic::fs::{path_is_abosolute, path_length_is_valid, path_name_is_safe, path_simplify};
use core::exec::PreserveMode;
use macros::{EnumDisplay, UnitSection};
use nix::sys::signal::Signal;
use nix::sys::wait::WaitStatus;
use nix::unistd::Pid;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;
use std::str::FromStr;
use unit_parser::internal::UnitEntry;

use core::error::*;
use core::exec::{ExecCommand, Rlimit, RuntimeDirectory, StateDirectory, WorkingDirectory};
use core::rel::{ReDb, ReDbRwTxn, ReDbTable, ReliSwitch, Reliability};
use core::unit::KillMode;

use basic::time::USEC_PER_MSEC;
use basic::time::USEC_PER_SEC;
use basic::EXEC_RUNTIME_PREFIX;

struct ServiceReDb<K, V>(ReDb<K, V>);

const RELI_DB_HSERVICE_CONF: &str = "svcconf";
const RELI_DB_HSERVICE_MNG: &str = "svcmng";

#[derive(PartialEq, Eq, Serialize, Deserialize, Debug, Clone, Copy)]
pub enum ServiceType {
    #[serde(alias = "simple")]
    Simple,
    #[serde(alias = "forking")]
    Forking,
    #[serde(alias = "oneshot")]
    Oneshot,
    #[serde(alias = "notify")]
    Notify,
    Idle,
    Exec,
    TypeMax,
    TypeInvalid = -1,
}

impl UnitEntry for ServiceType {
    type Error = core::error::Error;

    fn parse_from_str<S: AsRef<str>>(input: S) -> std::result::Result<Self, Self::Error> {
        Ok(match input.as_ref() {
            "simple" => ServiceType::Simple,
            "forking" => ServiceType::Forking,
            "oneshot" => ServiceType::Oneshot,
            "notify" => ServiceType::Notify,
            _ => ServiceType::Simple,
        })
    }
}

impl Default for ServiceType {
    fn default() -> Self {
        Self::Simple
    }
}

#[derive(PartialEq, Eq, Serialize, Deserialize, Debug, Clone, Copy)]
pub enum NotifyAccess {
    #[serde(alias = "none")]
    None,
    #[serde(alias = "all")]
    All,
    #[serde(alias = "main")]
    Main,
    #[serde(alias = "exec")]
    Exec,
}

impl UnitEntry for NotifyAccess {
    type Error = core::error::Error;

    fn parse_from_str<S: AsRef<str>>(input: S) -> std::result::Result<Self, Self::Error> {
        Ok(match input.as_ref() {
            "none" => NotifyAccess::None,
            "all" => NotifyAccess::All,
            "main" => NotifyAccess::Main,
            "exec" => NotifyAccess::Exec,
            _ => NotifyAccess::None,
        })
    }
}

#[derive(PartialEq, Eq, Serialize, Deserialize, Debug, Clone, Copy)]
pub enum ServiceRestart {
    #[serde(alias = "no")]
    No,
    #[serde(alias = "on-success")]
    OnSuccess,
    #[serde(alias = "on-failure")]
    OnFailure,
    #[serde(alias = "on-watchdog")]
    OnWatchdog,
    #[serde(alias = "on-abnormal")]
    OnAbnormal,
    #[serde(alias = "on-abort")]
    OnAbort,
    #[serde(alias = "always")]
    Always,
}

impl UnitEntry for ServiceRestart {
    type Error = core::error::Error;

    fn parse_from_str<S: AsRef<str>>(input: S) -> std::result::Result<Self, Self::Error> {
        Ok(match input.as_ref() {
            "no" => ServiceRestart::No,
            "on-success" => ServiceRestart::OnSuccess,
            "on-failure" => ServiceRestart::OnFailure,
            "on-watchdog" => ServiceRestart::OnWatchdog,
            "on-abnormal" => ServiceRestart::OnAbnormal,
            "on-abort" => ServiceRestart::OnAbort,
            "always" => ServiceRestart::Always,
            _ => ServiceRestart::No,
        })
    }
}

impl Default for ServiceRestart {
    fn default() -> Self {
        Self::No
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExitStatusSet {
    status: Vec<u8>,
    signal: Vec<String>,
}

impl ExitStatusSet {
    fn add_status(&mut self, status: u8) {
        self.status.push(status);
    }

    fn add_signal(&mut self, sig: String) {
        self.signal.push(sig);
    }

    pub fn exit_status_enabled(&self, wait_status: WaitStatus) -> bool {
        log::debug!("exit status enabled: {:?}", wait_status);
        match wait_status {
            WaitStatus::Exited(_, status) => self.status.contains(&(status as u8)),
            WaitStatus::Signaled(_, sig, _) => self.signal.contains(&sig.as_str().to_string()),
            _ => false,
        }
    }
}

fn exit_status_from_string(status: &str) -> Result<u8> {
    let s = status.parse::<u8>()?;

    Ok(s)
}

impl UnitEntry for ExitStatusSet {
    type Error = core::error::Error;

    fn parse_from_str<S: AsRef<str>>(input: S) -> std::result::Result<Self, Self::Error> {
        let mut status_set = ExitStatusSet::default();

        for cmd in input.as_ref().split_whitespace() {
            if cmd.is_empty() {
                continue;
            }

            if let Ok(v) = exit_status_from_string(cmd) {
                status_set.add_status(v);
                continue;
            }

            if let Ok(_sig) = Signal::from_str(cmd) {
                status_set.add_signal(cmd.to_string());
                continue;
            }
            log::warn!("RestartPreventExitStatus: invalid config value {}", cmd);
        }

        Ok(status_set)
    }
}

fn parse_pidfile(s: &str) -> Result<PathBuf> {
    if !path_name_is_safe(s) {
        return Err(core::error::Error::ConfigureError {
            msg: "PIDFile contains invalid character".to_string(),
        });
    }

    if !path_length_is_valid(s) {
        return Err(core::error::Error::ConfigureError {
            msg: "PIDFile is too long".to_string(),
        });
    }

    let s = match path_simplify(s) {
        None => {
            return Err(core::error::Error::ConfigureError {
                msg: "PIDFile is not valid".to_string(),
            });
        }
        Some(v) => v,
    };

    if path_is_abosolute(&s) {
        Ok(PathBuf::from(s))
    } else {
        Ok(Path::new(EXEC_RUNTIME_PREFIX).join(s))
    }
}

fn parse_sec(s: &str) -> Result<u64> {
    basic::time::parse_sec(s).context(NixSnafu)
}

fn parse_timeout(s: &str) -> Result<u64> {
    let timeout = s.parse::<u64>().unwrap();
    if timeout == 0 {
        return Ok(u64::MAX);
    }
    parse_sec(s)
}

#[derive(UnitSection, Serialize, Deserialize, Debug, Default, Clone)]
pub struct SectionService {
    #[entry(default=ServiceType::Simple)]
    pub Type: ServiceType,
    #[entry(append, parser = core::exec::parse_exec_command)]
    pub ExecStart: Vec<ExecCommand>,
    #[entry(append, parser = core::exec::parse_exec_command)]
    pub ExecStartPre: Vec<ExecCommand>,
    #[entry(append, parser = core::exec::parse_exec_command)]
    pub ExecStartPost: Vec<ExecCommand>,
    #[entry(append, parser = core::exec::parse_exec_command)]
    pub ExecStop: Vec<ExecCommand>,
    #[entry(append, parser = core::exec::parse_exec_command)]
    pub ExecStopPost: Vec<ExecCommand>,
    #[entry(append, parser = core::exec::parse_exec_command)]
    pub ExecReload: Vec<ExecCommand>,
    #[entry(append, parser = core::exec::parse_exec_command)]
    pub ExecCondition: Vec<ExecCommand>,
    #[entry(append)]
    pub Sockets: Vec<String>,
    #[entry(default = 0, parser = parse_sec)]
    pub WatchdogSec: u64,
    #[entry(parser = parse_pidfile)]
    pub PIDFile: Option<PathBuf>,
    #[entry(default = false)]
    pub RemainAfterExit: bool,
    pub NotifyAccess: Option<NotifyAccess>,
    #[entry(default = false)]
    pub NonBlocking: bool,
    #[entry(default = ServiceRestart::No)]
    pub Restart: ServiceRestart,
    #[entry(default = ExitStatusSet::default())]
    pub RestartPreventExitStatus: ExitStatusSet,
    #[entry(default = 100 * USEC_PER_MSEC, parser = parse_sec)]
    pub RestartSec: u64,
    #[entry(default = 90 * USEC_PER_SEC, parser = parse_timeout)]
    pub TimeoutSec: u64,
    #[entry(default = 90 * USEC_PER_SEC, parser = parse_timeout)]
    pub TimeoutStartSec: u64,
    #[entry(default = 90 * USEC_PER_SEC, parser = parse_timeout)]
    pub TimeoutStopSec: u64,

    // Exec
    #[entry(default = String::new())]
    pub User: String,
    #[entry(default = String::new())]
    pub Group: String,
    #[entry(default = String::from("0022"))]
    pub UMask: String,
    #[entry(parser = basic::fs::parse_pathbuf)]
    pub RootDirectory: Option<PathBuf>,
    #[entry(default = WorkingDirectory::default(), parser = core::exec::parse_working_directory)]
    pub WorkingDirectory: WorkingDirectory,
    #[entry(default = StateDirectory::default(), parser = core::exec::parse_state_directory)]
    pub StateDirectory: StateDirectory,
    #[entry(default = RuntimeDirectory::default(), parser = core::exec::parse_runtime_directory)]
    pub RuntimeDirectory: RuntimeDirectory,
    #[entry(default = PreserveMode::No)]
    pub RuntimeDirectoryPreserve: PreserveMode,
    pub LimitCORE: Option<Rlimit>,
    pub LimitNOFILE: Option<Rlimit>,
    pub LimitNPROC: Option<Rlimit>,
    #[entry(parser = core::exec::parse_environment)]
    pub Environment: Option<HashMap<String, String>>,
    #[entry(append)]
    pub EnvironmentFile: Vec<String>,
    pub SELinuxContext: Option<String>,

    // Kill
    #[entry(default = KillMode::ControlGroup)]
    pub KillMode: KillMode,
    #[entry(default = String::from("SIGTERM"))]
    pub KillSignal: String,
}

impl SectionService {
    pub(super) fn set_notify_access(&mut self, v: NotifyAccess) {
        self.NotifyAccess = Some(v);
    }

    pub(super) fn set_timeout_start(&mut self, time_out: u64) {
        if self.TimeoutStartSec == u64::MAX {
            self.TimeoutStartSec = time_out;
        }
    }

    pub(super) fn set_timeout_stop(&mut self, time_out: u64) {
        if self.TimeoutStopSec == u64::MAX {
            self.TimeoutStopSec = time_out;
        }
    }

    pub(super) fn set_property(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "Type" => self.Type = ServiceType::parse_from_str(value)?,
            "ExecStart" => self.ExecStart = core::exec::parse_exec_command(value)?,
            "ExecStartPre" => self.ExecStartPre = core::exec::parse_exec_command(value)?,
            "ExecStartPost" => self.ExecStartPost = core::exec::parse_exec_command(value)?,
            "ExecStop" => self.ExecStop = core::exec::parse_exec_command(value)?,
            "ExecStopPost" => self.ExecStopPost = core::exec::parse_exec_command(value)?,
            "ExecReload" => self.ExecReload = core::exec::parse_exec_command(value)?,
            "ExecCondition" => self.ExecCondition = core::exec::parse_exec_command(value)?,
            "Sockets" => self.Sockets = value.split_whitespace().map(|s| s.to_string()).collect(),
            "WatchdogSec" => self.WatchdogSec = value.parse::<u64>()?,
            "PIDFile" => self.PIDFile = Some(parse_pidfile(value)?),
            "RemainAfterExit" => self.RemainAfterExit = basic::config::parse_boolean(value)?,
            "NotifyAccess" => self.NotifyAccess = Some(NotifyAccess::parse_from_str(value)?),
            "NonBlocking" => self.NonBlocking = basic::config::parse_boolean(value)?,
            "Restart" => self.Restart = ServiceRestart::parse_from_str(value)?,
            "RestartPreventExitStatus" => {
                self.RestartPreventExitStatus = ExitStatusSet::parse_from_str(value)?
            }
            "RestartSec" => self.RestartSec = value.parse::<u64>()?,
            "TimeoutSec" => self.TimeoutSec = parse_timeout(value)?,
            "TimeoutStartSec" => self.TimeoutStartSec = parse_timeout(value)?,
            "TimeoutStopSec" => self.TimeoutStopSec = parse_timeout(value)?,

            //exec context
            "User" => self.User = value.to_string(),
            "Group" => self.User = value.to_string(),
            "UMask" => self.UMask = value.to_string(),
            "RootDirectory" => self.RootDirectory = Some(parse_pathbuf(value)?),
            "WorkingDirectory" => {
                self.WorkingDirectory = core::exec::parse_working_directory(value)?
            }
            "StateDirectory" => self.StateDirectory = core::exec::parse_state_directory(value)?,
            "RuntimeDirectory" => {
                self.RuntimeDirectory = core::exec::parse_runtime_directory(value)?
            }
            "RuntimeDirectoryPreserve" => {
                self.RuntimeDirectoryPreserve = PreserveMode::parse_from_str(value)?
            }
            "LimitCORE" => self.LimitCORE = Some(Rlimit::parse_from_str(value)?),
            "LimitNOFILE" => self.LimitNOFILE = Some(Rlimit::parse_from_str(value)?),
            "LimitNPROC" => self.LimitNPROC = Some(Rlimit::parse_from_str(value)?),
            "Environment" => self.Environment = Some(core::exec::parse_environment(value)?),
            "EnvironmentFile" => {
                self.EnvironmentFile = value.split_whitespace().map(|s| s.to_string()).collect()
            }
            "SELinuxContext" => self.SELinuxContext = Some(value.to_string()),

            //kill context
            "KillMode" => self.KillMode = KillMode::parse_from_str(value)?,
            "KillSignal" => self.KillSignal = value.to_string(),
            str_key => {
                return Err(Error::NotFound {
                    what: format!("set property:{}", str_key),
                })
            }
        };
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ServiceReConf {
    service: SectionService,
}

impl ServiceReConf {
    fn new(servicer: &SectionService) -> ServiceReConf {
        ServiceReConf {
            service: servicer.clone(),
        }
    }
}

#[derive(PartialEq, Eq, Debug, Copy, Clone, Serialize, Deserialize, EnumDisplay)]
pub(super) enum ServiceState {
    Dead,
    Condition,
    StartPre,
    Start,
    StartPost,
    Running,
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
    AutoRestart,
    Failed,
    Cleaning,
}

impl Default for ServiceState {
    fn default() -> Self {
        Self::Dead
    }
}

#[derive(PartialEq, Eq, Debug, Copy, Clone, Serialize, Deserialize)]
pub(super) enum ServiceResult {
    Success,
    FailureProtocol,
    FailureResources,
    FailureSignal,
    FailureStartLimitHit,
    FailureWatchdog,
    FailureExitCode,
    FailureCoreDump,
    FailureTimeout,
    SkipCondition,
    ResultInvalid,
}

impl Default for ServiceResult {
    fn default() -> Self {
        Self::ResultInvalid
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Copy, Clone, Serialize, Deserialize)]
pub(super) enum ServiceCommand {
    Condition,
    StartPre,
    Start,
    StartPost,
    Reload,
    Stop,
    StopPost,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy, Serialize, Deserialize)]
pub(super) enum NotifyState {
    Unknown,
    Ready,
    Stopping,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy, Serialize, Deserialize)]
pub(super) enum ExitStatus {
    Status(i32, i32),
}

impl From<ExitStatus> for WaitStatus {
    fn from(exit: ExitStatus) -> WaitStatus {
        match exit {
            ExitStatus::Status(pid, status) => {
                if !libc::WIFEXITED(status)
                    && !libc::WIFSIGNALED(status)
                    && !libc::WIFSTOPPED(status)
                    && !libc::WIFCONTINUED(status)
                {
                    // avoid WaitStatus::from_raw() assert
                    log::error!("pid:{:?} status:{:?} is illegal!", pid, status);
                    return WaitStatus::Exited(Pid::from_raw(pid), 0);
                } else if let Ok(wait) = WaitStatus::from_raw(Pid::from_raw(pid), status) {
                    return wait;
                }
                WaitStatus::Exited(Pid::from_raw(-1), 0)
            }
        }
    }
}

impl From<WaitStatus> for ExitStatus {
    fn from(wait_status: WaitStatus) -> Self {
        match wait_status {
            WaitStatus::Exited(pid, status) => ExitStatus::Status(i32::from(pid), status),
            WaitStatus::Signaled(pid, sig, _) => ExitStatus::Status(i32::from(pid), sig as i32),
            _ => ExitStatus::Status(0, 0),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ServiceReMng {
    state: ServiceState,
    result: ServiceResult,
    main_pid: Option<i32>,    // i32 ==> nix::unistd::Pid ==> libc::pid_t
    control_pid: Option<i32>, // i32 ==> nix::unistd::Pid ==> libc::pid_t
    main_cmd_len: usize,
    control_cmd_type: Option<ServiceCommand>,
    control_cmd_len: usize,
    notify_state: NotifyState,
    forbid_restart: bool,
    reset_restart: bool,
    restarts: u32,
    exit_status: ExitStatus,
    monitor: ServiceMonitor,
}

impl ServiceReMng {
    #[allow(clippy::too_many_arguments)]
    fn new(
        state: ServiceState,
        result: ServiceResult,
        main_pid: Option<i32>,
        control_pid: Option<i32>,
        main_cmd_len: usize,
        control_cmd_type: Option<ServiceCommand>,
        control_cmd_len: usize,
        notify_state: NotifyState,
        forbid_restart: bool,
        reset_restart: bool,
        restarts: u32,
        exit_status: ExitStatus,
        monitor: ServiceMonitor,
    ) -> ServiceReMng {
        ServiceReMng {
            state,
            result,
            main_pid,
            control_pid,
            main_cmd_len,
            control_cmd_type,
            control_cmd_len,
            notify_state,
            forbid_restart,
            reset_restart,
            restarts,
            exit_status,
            monitor,
        }
    }
}

pub(super) struct ServiceRe {
    // database: multi-instance(N)
    conf: Rc<ServiceReDb<String, ServiceReConf>>, // RELI_DB_ESERVICE_CONF; key: unit_id, data: config;
    mng: Rc<ServiceReDb<String, ServiceReMng>>, // RELI_DB_HSERVICE_MNG; key: unit_id, data: state+result+main(pid+cmd)+control(pid+cmd)+notify_state;
}

impl ServiceRe {
    pub(super) fn new(relir: &Rc<Reliability>) -> ServiceRe {
        let conf = Rc::new(ServiceReDb(ReDb::new(relir, RELI_DB_HSERVICE_CONF)));
        let mng = Rc::new(ServiceReDb(ReDb::new(relir, RELI_DB_HSERVICE_MNG)));
        let rentry = ServiceRe { conf, mng };
        rentry.register(relir);
        rentry
    }

    pub(super) fn conf_insert(&self, unit_id: &str, service: &SectionService) {
        let conf = ServiceReConf::new(service);
        self.conf.0.insert(unit_id.to_string(), conf);
    }

    pub(super) fn _conf_remove(&self, unit_id: &str) {
        self.conf.0.remove(&unit_id.to_string());
    }

    pub(super) fn conf_get(&self, unit_id: &str) -> Option<SectionService> {
        let conf = self.conf.0.get(&unit_id.to_string());
        conf.map(|c| c.service)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn mng_insert(
        &self,
        unit_id: &str,
        state: ServiceState,
        result: ServiceResult,
        main_pid: Option<Pid>,
        control_pid: Option<Pid>,
        main_cmd_len: usize,
        control_cmd_type: Option<ServiceCommand>,
        control_cmd_len: usize,
        notify_state: NotifyState,
        forbid_restart: bool,
        reset_restart: bool,
        restarts: u32,
        exit_status: ExitStatus,
        monitor: ServiceMonitor,
    ) {
        let m_pid = main_pid.map(|x| x.as_raw());
        let c_pid = control_pid.map(|x| x.as_raw());
        let mng = ServiceReMng::new(
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
        );
        self.mng.0.insert(unit_id.to_string(), mng);
    }

    pub(super) fn _mng_remove(&self, unit_id: &str) {
        self.mng.0.remove(&unit_id.to_string());
    }

    #[allow(clippy::type_complexity)]
    pub(super) fn mng_get(
        &self,
        unit_id: &str,
    ) -> Option<(
        ServiceState,
        ServiceResult,
        Option<Pid>,
        Option<Pid>,
        usize,
        Option<ServiceCommand>,
        usize,
        NotifyState,
        bool,
        bool,
        u32,
        ExitStatus,
        ServiceMonitor,
    )> {
        let mng = self.mng.0.get(&unit_id.to_string());
        mng.map(|m| {
            (
                m.state,
                m.result,
                m.main_pid.map(Pid::from_raw),
                m.control_pid.map(Pid::from_raw),
                m.main_cmd_len,
                m.control_cmd_type,
                m.control_cmd_len,
                m.notify_state,
                m.forbid_restart,
                m.reset_restart,
                m.restarts,
                m.exit_status,
                m.monitor,
            )
        })
    }

    fn register(&self, relir: &Reliability) {
        // rel-db: RELI_DB_HSERVICE_CONF
        let db = Rc::clone(&self.conf);
        relir.history_db_register(RELI_DB_HSERVICE_CONF, db);

        // rel-db: RELI_DB_HSERVICE_MNG
        let db = Rc::clone(&self.mng);
        relir.history_db_register(RELI_DB_HSERVICE_MNG, db);
    }
}

impl ReDbTable for ServiceReDb<String, ServiceReConf> {
    fn clear(&self, wtxn: &mut ReDbRwTxn) {
        self.0.do_clear(wtxn);
    }

    fn export(&self, db_wtxn: &mut ReDbRwTxn) {
        self.0.cache_2_db(db_wtxn);
    }

    fn flush(&self, db_wtxn: &mut ReDbRwTxn, switch: ReliSwitch) {
        self.0.data_2_db(db_wtxn, switch);
    }

    fn import<'a>(&self) {
        self.0.db_2_cache();
    }

    fn switch_set(&self, switch: ReliSwitch) {
        self.0.switch_buffer(switch);
    }
}

impl ReDbTable for ServiceReDb<String, ServiceReMng> {
    fn clear(&self, wtxn: &mut ReDbRwTxn) {
        self.0.do_clear(wtxn);
    }

    fn export(&self, db_wtxn: &mut ReDbRwTxn) {
        self.0.cache_2_db(db_wtxn);
    }

    fn flush(&self, db_wtxn: &mut ReDbRwTxn, switch: ReliSwitch) {
        self.0.data_2_db(db_wtxn, switch);
    }

    fn import<'a>(&self) {
        self.0.db_2_cache();
    }

    fn switch_set(&self, switch: ReliSwitch) {
        self.0.switch_buffer(switch);
    }
}
