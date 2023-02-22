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
pub(super) use datastore::UnitDb;
pub(super) use entry::UnitX;
pub(super) use manager::UnitManagerX;
pub use rentry::UeConfigInstall;
pub(super) use rentry::{unit_name_to_type, JobMode};
pub(crate) use sysmaster::unit::UnitRelationAtom;
pub(crate) use sysmaster::unit::{UnitRelations, UnitType};

#[cfg(test)]
pub(super) use data::DataManager;

#[cfg(test)]
pub(super) use rentry::UnitRe;
#[cfg(test)]
pub(super) use test::test_utils;

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

mod base;
mod data;
mod datastore;
mod entry;
mod execute;
mod manager;
mod notify;
mod rentry;
mod runtime;
mod sigchld;
#[cfg(test)]
mod test;
mod uload;
mod util;
