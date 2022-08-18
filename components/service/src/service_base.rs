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
    #[serde(alias = "forking")]
    Forking,
    #[strum(serialize = "oneshot")]
    #[serde(alias = "oneshot")]
    Oneshot,
    #[strum(serialize = "notify")]
    #[serde(alias = "notify")]
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

#[derive(PartialEq, Eq, Serialize, Deserialize, EnumString, Display, Debug, Clone, Copy)]
pub(super) enum NotifyAccess {
    #[strum(serialize = "none")]
    #[serde(alias = "none")]
    None,
    #[strum(serialize = "main")]
    #[serde(alias = "main")]
    Main,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub(super) enum NotifyState {
    Unknown,
    Ready,
    Reloading,
    Stoping,
}
