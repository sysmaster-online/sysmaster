#![warn(unused_imports)]
use nix::sys::signal::Signal;
use serde::{Deserialize, Deserializer};

#[allow(missing_docs)]
#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum UnitDependencyMask {
    UnitDependencyDefault = 1 << 2,
}

#[allow(missing_docs)]
#[macro_export]
macro_rules! null_str {
    ($name:expr) => {
        String::from($name)
    };
}

#[allow(missing_docs)]
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
    UnitActionEBusy,
    UnitActionENoent,
}

#[allow(missing_docs)]
pub enum KillOperation {
    KillTerminate,
    KillTerminateAndLog,
    KillRestart,
    KillKill,
    KillWatchdog,
    KillInvalid,
}

impl KillOperation {
    ///
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

///
pub trait DeserializeWith: Sized {
    ///
    fn deserialize_with<'de, D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>;
}

impl DeserializeWith for Vec<String> {
    fn deserialize_with<'de, D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(de)?;
        let mut vec = Vec::new();

        for l in s.split_terminator(';') {
            vec.push(l.trim().to_string());
        }

        Ok(vec)
    }
}

///
#[derive(Default)]
pub struct UnitRef {
    source: Option<String>,
    target: Option<String>,
}

impl UnitRef {
    ///
    pub fn new() -> Self {
        UnitRef {
            source: None,
            target: None,
        }
    }

    ///
    pub fn set_ref(&mut self, source: String, target: String) {
        self.source = Some(source);
        self.target = Some(target);
    }

    ///
    pub fn unset_ref(&mut self) {
        self.source = None;
        self.target = None;
    }

    ///
    pub fn target(&self) -> Option<&String> {
        self.target.as_ref()
    }
}
