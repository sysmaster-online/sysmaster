pub use data::{UnitActiveState, UnitRelations, UnitType};
pub use manager::{Manager, ManagerX};
pub use unit::{JobAffect, JobConf, JobInfo, JobKind, JobManager, JobResult, JobStage};
pub use unit::{
    KillOperation, Unit, UnitActionError, UnitDb, UnitManager, UnitManagerX, UnitMngUtil, UnitObj,
    UnitSubClass, UnitX,
};

#[derive(Debug)]
pub enum MngErrno {
    MngErrInput,
    MngErrNotExisted,
    MngErrInternel,
    MngErrNotSupported,
}

pub mod commands;
#[allow(dead_code)]
pub mod manager;
pub mod signals;
#[allow(dead_code)]
mod unit;

#[allow(dead_code)]
mod data;
#[allow(dead_code)]
mod table;
