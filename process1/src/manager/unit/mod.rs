//!  Unit is the main module for process 1 to manage and abstract system services
//!  The module contains:
//!  [execute]: unit Object data structure definition to be executed.
//!  [job]: The scheduling execution entity corresponding to the unit. After each unit is started, it will be driven by the job.
//!  [uload_util]: Attribute definitions related to each unit configuration file.
//!  [unit_base]: Definition of basic attributes of unit related objects, such as enumeration of unit type and definition of unit dependency
//!  [unit_datastore]: the unit object storage module is responsible for storing the unit module status.
//!  [unit_entry]: Definition of unit related objects
//!
pub use data::{UnitActiveState, UnitNotifyFlags};
pub use execute::{ExecCmdError, ExecContext, ExecFlags, ExecParameters};
pub use unit_base::{
    DeserializeWith, KillOperation, UnitActionError, UnitDependencyMask, UnitRef, UnitRelationAtom,
};
pub use unit_entry::{Unit, UnitObj};
pub(super) use unit_manager::UnitManagerX;
pub use unit_manager::{UnitManager, UnitManagerObj, UnitMngUtil, UnitSubClass};
pub use unit_rentry::{ExecCommand, UnitRelations, UnitType};

///
#[allow(clippy::enum_variant_names)]
#[derive(Debug)]
pub enum UnitErrno {
    ///
    UnitErrInput,
    ///
    UnitErrNotExisted,
    ///
    UnitErrInternel,
    ///
    UnitErrNotSupported,
}

// dependency:
// unit_rentry -> data -> unit_base -> {uload_util} ->
// unit_entry -> {unit_datastore -> unit_runtime} -> job ->
// {execute | sigchld | notify} -> unit_manager

mod data;
mod execute;
mod job;
mod notify;
mod sigchld;
mod uload_util;
mod unit_base;
mod unit_datastore;
mod unit_entry;
mod unit_manager;
mod unit_rentry;
mod unit_runtime;
