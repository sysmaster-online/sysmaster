pub use unit_base::{KillOperation, UnitActionError, UnitType};
pub use unit_entry::{Unit, UnitObj};
pub(super) use unit_manager::UnitManagerX;
pub use unit_manager::{UnitManager, UnitMngUtil, UnitSubClass};

#[derive(Debug)]
pub enum UnitErrno {
    UnitErrInput,
    UnitErrNotExisted,
    UnitErrInternel,
    UnitErrNotSupported,
}

// dependency:
// unit_base -> {uload_util} ->
// unit_entry ->
// {unit_datastore -> unit_runtime} ->
// {job} -> unit_manager

mod job;
mod uload_util;
mod unit_base;
mod unit_datastore;
mod unit_entry;
mod unit_manager;
mod unit_runtime;