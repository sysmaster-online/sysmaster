//!
pub use manager::{Action, Manager, Mode, MANAGER_ARGS_SIZE_MAX};
pub use rentry::ReliLastFrame;
pub use unit::{
    DeserializeWith, ExecCmdError, ExecCommand, ExecContext, ExecFlags, ExecParameters,
    KillOperation, SubUnit, UmIf, Unit, UnitActionError, UnitActiveState, UnitDependencyMask,
    UnitManager, UnitManagerObj, UnitMngUtil, UnitNotifyFlags, UnitRef, UnitRelationAtom,
    UnitRelations, UnitType,
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
mod config;
mod manager;
mod pre_install;
mod rentry;
mod signals;
mod table;
mod unit;
