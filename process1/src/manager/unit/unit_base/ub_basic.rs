#![warn(unused_imports)]
use crate::null_str;
use core::fmt::{Display, Formatter, Result as FmtResult};
use nix::sys::signal::Signal;
use std::{num::ParseIntError, str::FromStr};

#[derive(PartialEq, Eq, Hash, Copy, Clone)]
pub enum UnitType {
    UnitService = 0,
    UnitTarget,
    UnitSocket,
    UnitTypeMax,
    UnitTypeInvalid,
    UnitTypeErrnoMax,
}

impl FromStr for UnitType {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ret = match s {
            "Service" => UnitType::UnitService,
            "Target" => UnitType::UnitTarget,
            "Socket" => UnitType::UnitSocket,
            _ => UnitType::UnitTypeInvalid,
        };
        Ok(ret)
    }
}
impl From<UnitType> for String {
    fn from(u_t: UnitType) -> Self {
        match u_t {
            UnitType::UnitService => "service".into(),
            UnitType::UnitTarget => "target".into(),
            UnitType::UnitSocket => "socket".into(),
            UnitType::UnitTypeMax => null_str!("").into(),
            UnitType::UnitTypeInvalid => null_str!("").into(),
            UnitType::UnitTypeErrnoMax => null_str!("").into(),
        }
    }
}
impl Display for UnitType {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            UnitType::UnitService => write!(f, "Service"),
            UnitType::UnitTarget => write!(f, "Target"),
            UnitType::UnitSocket => write!(f, "Socket"),
            UnitType::UnitTypeMax => write!(f, "Max"),
            UnitType::UnitTypeInvalid => write!(f, ""),
            UnitType::UnitTypeErrnoMax => write!(f, ""),
        }
    }
}

// #[macro_export]
// macro_rules! unit_name_to_type{
//     ($name:expr) => {
//         match $name{
//             "*.service" => UnitType::UnitService,
//             "*.target" => UnitType::UnitTarget,
//             _ => UnitType::UnitTypeInvalid,
//         }
//     };
// }

pub(in crate::manager::unit) fn unit_name_to_type(unit_name: &str) -> UnitType {
    let words: Vec<&str> = unit_name.split(".").collect();
    match words[words.len() - 1] {
        "service" => UnitType::UnitService,
        "target" => UnitType::UnitTarget,
        _ => UnitType::UnitTypeInvalid,
    }
}

#[macro_export]
macro_rules! null_str {
    ($name:expr) => {
        String::from($name)
    };
}

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum UnitActionError {
    UnitActionEAgain,
    UnitActionEAlready,
    UnitActionEComm,
    UnitActionEBadR,
    UnitActionENoExec,
    UnitActionEProto,
    UnitActionEOpNotSupp,
    UnitActionENolink,
    UnitActionEStale,
    UnitActionEFailed,
    UnitActionEInval,
}

pub enum KillOperation {
    KillTerminate,
    KillTerminateAndLog,
    KillRestart,
    KillKill,
    KillWatchdog,
    KillInvalid,
}

impl KillOperation {
    pub fn to_signal(&self) -> Signal {
        match *self {
            KillOperation::KillTerminate
            | KillOperation::KillTerminateAndLog
            | KillOperation::KillRestart => Signal::SIGTERM,
            KillOperation::KillKill => Signal::SIGKILL,
            KillOperation::KillWatchdog => Signal::SIGABRT,
            _ => Signal::SIGTERM,
        }
    }
}
