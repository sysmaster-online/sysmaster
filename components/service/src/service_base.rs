use serde::{Deserialize, Serialize};

#[derive(PartialEq, EnumString, Display, Debug)]
pub(super) enum ServiceTimeoutFailureMode {
    #[strum(serialize = "terminate")]
    TimeoutTerminate,
    #[strum(serialize = "abort")]
    TimeoutAbort,
    #[strum(serialize = "kill")]
    TimeoutKill,
    TimeoutFailureModeMax,
    TimeoutFailureModeInvalid = -1,
}

impl Default for ServiceTimeoutFailureMode {
    fn default() -> Self {
        ServiceTimeoutFailureMode::TimeoutTerminate
    }
}

#[derive(PartialEq, Default, Debug)]
pub(super) struct ExitStatusSet {}

#[derive(PartialEq, EnumString, Display, Debug)]
pub(super) enum ServiceRestart {
    #[strum(serialize = "no")]
    RestartNo,
    #[strum(serialize = "on-success")]
    RestartOnSuccess,
    #[strum(serialize = "on-failure")]
    RestartOnFailure,
    #[strum(serialize = "on-abnormal")]
    RestartOnAbnormal,
    #[strum(serialize = "on-abort")]
    RestartOnAbort,
    #[strum(serialize = "always")]
    RestartAlways,
    RestartMax,
    RestartInvalid = -1,
}

impl Default for ServiceRestart {
    fn default() -> Self {
        ServiceRestart::RestartNo
    }
}

#[derive(PartialEq, Eq, Serialize, Deserialize, EnumString, Display, Debug, Clone, Copy)]
pub(super) enum ServiceType {
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

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Copy, Clone)]
pub(super) enum ServiceCommand {
    Condition,
    StartPre,
    Start,
    StartPost,
    Reload,
    Stop,
    StopPost,
    CommandMax,
}

#[derive(PartialEq, Default, Debug)]
pub(super) struct DualTimestamp {}
