//!
pub use manager::{Manager,Action, Mode, MANAGER_ARGS_SIZE_MAX};
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
    /// internel error
    Internel,
    /// not supported
    NotSupported,
}

mod commands;
#[allow(dead_code)]
mod manager;
#[allow(dead_code)]
mod rentry;
mod signals;
#[allow(dead_code)]
mod table;
#[allow(dead_code)]
mod unit;
