use process1::manager::DeserializeWith;
use serde::{Deserialize, Deserializer, Serialize};

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

impl DeserializeWith for ServiceType {
    fn deserialize_with<'de, D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(de)?;

        match s.as_ref() {
            "simple" => Ok(ServiceType::Simple),
            "forking" => Ok(ServiceType::Forking),
            "oneshot" => Ok(ServiceType::Oneshot),
            "notify" => Ok(ServiceType::Notify),
            &_ => Ok(ServiceType::Simple),
        }
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
    Stopping,
}
