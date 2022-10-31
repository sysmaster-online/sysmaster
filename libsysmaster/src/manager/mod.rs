//!
pub(super) use manager::Manager;
pub use manager::{Action, ManagerX, Mode, MANAGER_ARGS_SIZE_MAX};
pub use rentry::ReliLastFrame;
pub use unit::{
    DeserializeWith, ExecCmdError, ExecCommand, ExecContext, ExecFlags, ExecParameters,
    KillOperation, Unit, UnitActionError, UnitActiveState, UnitDependencyMask, UnitManager,
    UnitManagerObj, UnitMngUtil, UnitNotifyFlags, UnitObj, UnitRef, UnitRelationAtom,
    UnitRelations, UnitSubClass, UnitType,
};

/// error number of manager
#[derive(Debug)]
pub enum MngErrno {
    /// invalid input
    Input,
    /// not existed
    NotExisted,
    /// Internal error
    Internal,
    /// not supported
    NotSupported,
}

mod commands;
#[allow(dead_code)]
mod manager;
mod pre_install;
#[allow(dead_code)]
mod rentry;
mod signals;
#[allow(dead_code)]
mod table;
#[allow(dead_code)]
mod unit;
