use core::fmt::{Display, Formatter, Result as FmtResult};
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use process1::manager::{KillOperation, UnitActiveState};
use config_proc_macro::ConfigParseM;

#[derive(Serialize, Deserialize,ConfigParseM)]
#[serdeName("Service")]
pub struct ServiceConf {
    #[serde(alias = "Type")]
    pub service_type: Option<String>,
    #[serde(alias = "ExecStart")]
    pub exec_start: Option<String>,
    #[serde(alias = "Sockets")]
    pub sockets: Option<String>,
    #[serde(alias = "Restart")]
    pub restart: Option<String>,
    #[serde(alias = "RestrictRealtime")]
    pub restrict_realtime: Option<String>,
    #[serde(alias = "RebootArgument")]
    pub reboot_argument: Option<String>,
    #[serde(alias = "ExecReload")]
    pub exec_reload: Option<String>,
    #[serde(alias = "OOMScoreAdjust")]
    pub oom_score_adjust: Option<String>,
    #[serde(alias = "RestartSec")]
    pub restart_sec: Option<u64>,
    #[serde(alias = "Slice")]
    pub slice: Option<String>,
    #[serde(alias = "MemoryLimit")]
    pub memory_limit: Option<u64>,
    #[serde(alias = "MemoryLow")]
    pub memory_low: Option<u64>,
    #[serde(alias = "MemoryMin")]
    pub memory_min: Option<u64>,
    #[serde(alias = "MemoryMax")]
    pub memory_max: Option<u64>,
    #[serde(alias = "MemoryHigh")]
    pub memory_high: Option<u64>,
    #[serde(alias = "MemorySwapMax")]
    pub memory_swap_max: Option<u64>,
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
    ServiceIdle,
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

pub enum ServiceConf {
    Type,
    ExecCondition,
    ExecStart,
    ExecReload,
    ExecStop,
}

impl Display for ServiceConf {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            ServiceConf::Type => write!(f, "Type"),
            ServiceConf::ExecCondition => write!(f, "ExecCondition"),
            ServiceConf::ExecStart => write!(f, "ExecStart"),
            ServiceConf::ExecReload => write!(f, "ExecReload"),
            ServiceConf::ExecStop => write!(f, "ExecStop"),
        }
    }
}

impl From<ServiceConf> for String {
    fn from(service_conf: ServiceConf) -> Self {
        match service_conf {
            ServiceConf::Type => "Type".into(),
            ServiceConf::ExecCondition => "ExecCondition".into(),
            ServiceConf::ExecStart => "ExecStart".into(),
            ServiceConf::ExecReload => "ExecReload".into(),
            ServiceConf::ExecStop => "ExecStop".into(),
        }
    }
}
