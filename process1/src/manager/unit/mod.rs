pub use job::{JobAffect, JobConf, JobInfo, JobKind, JobManager, JobResult, JobStage};
pub use unit_base::{KillOperation, UnitActiveState};
pub use unit_datastore::UnitDb;
pub use unit_entry::{Unit, UnitObj, UnitX};
pub use unit_manager::UnitManager;

#[derive(Debug)]
pub enum UnitErrno {
    UnitErrInput,
    UnitErrNotExisted,
    UnitErrInternel,
    UnitErrNotSupported,
}

// dependency: {unit_base | unit_relation | unit_relation_atom} -> unit_entry -> {unit_dep | unit_sets | unit_datastore} -> {unit_configs | job} -> unit_manager
mod job;
mod unit_base;
mod unit_configs;
mod unit_datastore;
mod unit_dep;
mod unit_entry;
mod unit_manager;
mod unit_parser_mgr;
mod unit_relation;
mod unit_relation_atom;
mod unit_sets;

//后续考虑和plugin结合
use crate::manager::data::{DataManager, UnitType};
use std::error::Error;
use std::rc::Rc;

fn unit_new(
    dm: Rc<DataManager>,
    unit_db: Rc<UnitDb>,
    unit_type: UnitType,
    name: &str,
) -> Result<Rc<UnitX>, Box<dyn Error>> {
    UnitX::new(dm, unit_db, unit_type, name)
}
