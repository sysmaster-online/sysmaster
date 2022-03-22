pub use job::{JobAffect, JobConf, JobInfo, JobKind, JobManager, JobResult, JobStage};
pub use unit_base::KillOperation;
pub use unit_datastore::UnitDb;
pub use unit_entry::{Unit, UnitObj, UnitX};
pub use unit_manager::{UnitManager, UnitManagerX, UnitMngUtil, UnitSubClass};


#[derive(Debug)]
pub enum UnitErrno {
    UnitErrInput,
    UnitErrNotExisted,
    UnitErrInternel,
    UnitErrNotSupported,
}

// dependency:
// {unit_base | unit_relation | unit_relation_atom} -> unit_file ->
// unit_entry ->
// {unit_datastore | unit_runtime} ->
// {unit_load | job} -> unit_manager
mod job;
mod unit_base;
mod unit_datastore;
mod unit_entry;
mod unit_file;
mod unit_load;
mod unit_manager;
mod unit_parser_mgr;
mod unit_relation;
mod unit_relation_atom;
mod unit_runtime;
