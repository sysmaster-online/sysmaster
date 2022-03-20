use crate::manager::data::UnitType;
use nix::sys::signal::Signal;

#[derive(PartialEq, Debug, Eq)]
pub enum UnitLoadState {
    UnitStub = 0,
    UnitLoaded,
    UnitNotFound,
    UnitError,
    UnitMerged,
    UnitMasked,
    UnitLoadStateMax,
    UnitLoadStateInvalid = -1,
}

enum UnitNameFlags {
    UnitNamePlain = 1,
    UnitNameInstance = 2,
    UnitNameTemplate = 4,
    UnitNameAny = 1 | 2 | 4,
}

enum UnitFileState {
    UnitFileEnabled,
    UnitFileEnabledRuntime,
    UnitFileLinked,
    UnitFileLinkedRuntime,
    UnitFileAlias,
    UnitFileMasked,
    UnitFileMaskedRuntime,
    UnitFileStatic,
    UnitFileDisabled,
    UnitFileIndirect,
    UnitFileGenerated,
    UnitFileTransient,
    UnitFileBad,
    UnitFileStateMax,
    UnitFileStateInvalid,
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
}

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum UnitActiveState {
    UnitActive,
    UnitReloading,
    UnitInActive,
    UnitFailed,
    UnitActivating,
    UnitDeActivating,
    UnitMaintenance,
}

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum UnitNotifyFlags {
    UnitNotifyReloadFailure = 1 << 0,
    UnitNotifyWillAutoRestart = 1 << 1,
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

pub(super) fn unit_name_to_type(unit_name: &str) -> UnitType {
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
