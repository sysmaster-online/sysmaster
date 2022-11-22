use nix::sys::signal::Signal;
use serde::{Deserialize, Deserializer};
use libutils::serialize::DeserializeWith;
#[allow(missing_docs)]
#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum UnitDependencyMask {
    UnitDependencyDefault = 1 << 2,
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
    UnitActionECanceled,
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
