//!  Unit is the main module for process 1 to manage and abstract system services
//!  The module contains:
//!  [execute]: unit Object data structure definition to be executed.
//!  [job]: The scheduling execution entity corresponding to the unit. After each unit is started, it will be driven by the job.
//!  [uload_util]: Attribute definitions related to each unit configuration file.
//!  [unit_base]: Definition of basic attributes of unit related objects, such as enumeration of unit type and definition of unit dependency
//!  [unit_datastore]: the unit object storage module is responsible for storing the unit module status.
//!  [unit_entry]: Definition of unit related objects
//!  [unit_manager]: Manager all Unit Instances in sysmaster
//!  [um_interface]: Share api of unit_manager for subunit

pub(in super) use data::{UnitActiveState, UnitNotifyFlags};
pub(in super) use unit_entry::{SubUnit, Unit, UnitX};
pub(in super) use unit_manager::UnitManagerX;
pub(in super) use unit_rentry::unit_name_to_type;
pub(in super) use unit_rentry::{JobMode};
#[cfg(test)]
pub(in super) use test::test_utils;
pub(in super) use unit_datastore::UnitDb;
pub use execute::{ExecCmdError, ExecContext, ExecFlags, ExecParameters};
pub use um_interface::UmIf;
pub use unit_base::{
    DeserializeWith, KillOperation, UnitActionError, UnitDependencyMask, UnitRef, UnitRelationAtom,
};
pub use unit_entry::{KillContext, KillMode};
pub use unit_manager::{UnitManager, UnitManagerObj, UnitMngUtil};

pub use unit_rentry::UeConfigInstall;
pub use unit_rentry::{ExecCommand, UnitRelations, UnitType};




///
#[allow(dead_code)]
#[derive(Debug)]
pub enum UnitErrno {
    ///
    InputErr,
    ///
    NotExisted,
    ///
    InternalErr,
    ///
    NotSupported,
}

// dependency:
// unit_rentry -> data -> unit_base -> {uload_util} ->
// unit_entry -> {unit_datastore -> unit_runtime} -> job ->
// {execute | sigchld | notify} -> unit_manager -> um_interface

mod data;
mod execute;
mod notify;
mod sigchld;
#[cfg(test)]
mod test;
mod uload_util;
mod um_interface;
mod unit_base;
mod unit_datastore;
mod unit_entry;
mod unit_manager;
mod unit_rentry;
mod unit_runtime;
