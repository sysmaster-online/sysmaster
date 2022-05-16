pub use data::{UnitActiveState, UnitNotifyFlags, UnitRelations};
pub(super) use manager::Manager;
pub use manager::{Action, ManagerX, Mode, Stats};
pub use unit::{
    KillOperation, Unit, UnitActionError, UnitManager, UnitMngUtil, UnitObj, UnitSubClass, UnitType,
};

#[derive(Debug)]
pub enum MngErrno {
    MngErrInput,
    MngErrNotExisted,
    MngErrInternel,
    MngErrNotSupported,
}

mod commands;
#[allow(dead_code)]
mod manager;
mod signals;
#[allow(dead_code)]
mod unit;

#[allow(dead_code)]
mod data;
#[allow(dead_code)]
mod table;
