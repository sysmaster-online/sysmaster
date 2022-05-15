use core::fmt::Result as FmtResult;
use proc_macro_utils::ConfigParseM;
use process1::manager::{KillOperation, UnitActiveState};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::fmt;
use std::io::{Error as IoError, ErrorKind};
use std::rc::Rc;
use utils::config_parser::{toml_str_parse, ConfigParse};

#[derive(Serialize, Deserialize, ConfigParseM)]
#[serdeName("Service")]
#[serde(rename_all = "PascalCase")]
pub(crate) struct ServiceConf {
    #[serde(alias = "Type", default = "ServiceType::default")]
    service_type: ServiceType,
    #[serde(alias = "ExecStart")]
    exec_start: Option<Vec<String>>,
    #[serde(alias = "ExecStop")]
    exec_stop: Option<Vec<String>>,
    #[serde(alias = "ExecCondition")]
    exec_condition: Option<Vec<String>>,
    #[serde(alias = "Sockets")]
    sockets: Option<String>,
    #[serde(alias = "Restart")]
    restart: Option<Vec<String>>,
    #[serde(alias = "RestrictRealtime")]
    restrict_realtime: Option<String>,
    #[serde(alias = "RebootArgument")]
    reboot_argument: Option<String>,
    #[serde(alias = "ExecReload")]
    exec_reload: Option<Vec<String>>,
    #[serde(alias = "OOMScoreAdjust")]
    oom_score_adjust: Option<String>,
    #[serde(alias = "RestartSec")]
    restart_sec: Option<u64>,
    #[serde(alias = "Slice")]
    slice: Option<String>,
    #[serde(alias = "MemoryLimit")]
    memory_limit: Option<u64>,
    #[serde(alias = "MemoryLow")]
    memory_low: Option<u64>,
    #[serde(alias = "MemoryMin")]
    memory_min: Option<u64>,
    #[serde(alias = "MemoryMax")]
    memory_max: Option<u64>,
    #[serde(alias = "MemoryHigh")]
    memory_high: Option<u64>,
    #[serde(alias = "MemorySwapMax")]
    memory_swap_max: Option<u64>,
}

#[derive(PartialEq, EnumString, Display, Debug)]
pub(in crate) enum ServiceTimeoutFailureMode {
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

#[derive(PartialEq, Default, Debug)]
pub(super) struct ExitStatusSet {}

#[derive(PartialEq, EnumString, Display, Debug)]
pub(crate) enum ServiceRestart {
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

#[derive(PartialEq, Eq, Serialize, Deserialize, EnumString, Display, Debug, Clone)]
pub(crate) enum ServiceType {
    #[strum(serialize = "simple")]
    #[serde(alias = "simple")]
    Simple,
    #[strum(serialize = "forking")]
    Forking,
    #[strum(serialize = "oneshot")]
    Oneshot,
    #[strum(serialize = "dbus")]
    Dbus,
    #[strum(serialize = "notify")]
    Notify,
    #[strum(serialize = "idle")]
    Idle,
    #[strum(serialize = "exec")]
    Exec,
    TypeMax,
    TypeInvalid = -1,
}

impl Default for ServiceType {
    fn default() -> Self {
        ServiceType::Simple
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
#[allow(dead_code)]
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
    ServiceAutoRestart,
    ServiceCleaning,
    ServiceStateMax,
}

impl Default for ServiceState {
    fn default() -> Self {
        ServiceState::ServiceStateMax
    }
}

impl ServiceState {
    pub fn to_unit_active_state(&self) -> UnitActiveState {
        match *self {
            ServiceState::ServiceDead => UnitActiveState::UnitInActive,
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
            | ServiceState::ServiceFinalWatchdog => UnitActiveState::UnitDeActivating,
            ServiceState::ServiceFailed => UnitActiveState::UnitFailed,
            ServiceState::ServiceAutoRestart => UnitActiveState::UnitActivating,
            ServiceState::ServiceCleaning => UnitActiveState::UnitMaintenance,
        }
    }

    pub fn to_unit_active_state_idle(&self) -> UnitActiveState {
        match *self {
            ServiceState::ServiceDead => UnitActiveState::UnitInActive,
            ServiceState::ServiceCondition
            | ServiceState::ServiceStartPre
            | ServiceState::ServiceStart
            | ServiceState::ServiceStartPost
            | ServiceState::ServiceRuning
            | ServiceState::ServiceExited => UnitActiveState::UnitActive,
            ServiceState::ServiceReload => UnitActiveState::UnitReloading,
            ServiceState::ServiceStop
            | ServiceState::ServiceStopWatchdog
            | ServiceState::ServiceStopPost
            | ServiceState::ServiceStopSigterm
            | ServiceState::ServiceStopSigkill
            | ServiceState::ServiceStateMax
            | ServiceState::ServiceFinalSigterm
            | ServiceState::ServiceFinalSigkill
            | ServiceState::ServiceFinalWatchdog => UnitActiveState::UnitDeActivating,
            ServiceState::ServiceFailed => UnitActiveState::UnitFailed,
            ServiceState::ServiceAutoRestart => UnitActiveState::UnitActivating,
            ServiceState::ServiceCleaning => UnitActiveState::UnitMaintenance,
        }
    }

    pub fn to_kill_operation(&self) -> KillOperation {
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

#[allow(dead_code)]
pub enum CmdError {
    Timeout,
    NoCmdFound,
    SpawnError,
}

#[derive(PartialEq, Default, Debug)]
pub(super) struct DualTimestamp {}

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
    fn fmt(&self, f: &mut fmt::Formatter) -> FmtResult {
        write!(f, "Display: {}", self.cmd)
    }
}
