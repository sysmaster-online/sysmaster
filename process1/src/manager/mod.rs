pub use data::{UnitActiveState, UnitNotifyFlags, UnitRelations};
pub(super) use manager::Manager;
pub use manager::{Action, ManagerX, Mode, Stats};
pub use unit::{
    ExecCmdError, ExecCommand, ExecContext, ExecFlags, ExecParameters, KillOperation, Unit,
    UnitActionError, UnitManager, UnitMngUtil, UnitObj, UnitRef, UnitSubClass, UnitType,
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
mod data;
#[allow(dead_code)]
mod manager;
#[allow(dead_code)]
mod reliability;
mod signals;
#[allow(dead_code)]
mod table;
#[allow(dead_code)]
mod unit;
