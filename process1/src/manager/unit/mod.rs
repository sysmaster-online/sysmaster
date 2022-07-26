pub use execute::{ExecCmdError, ExecCommand, ExecContext, ExecFlags, ExecParameters};
pub use unit_base::{KillOperation, UnitActionError, UnitType};
pub use unit_entry::{Unit, UnitObj, UnitRef};
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
// {job | execute} -> unit_manager

mod execute;
mod job;
mod uload_util;
mod unit_base;
mod unit_datastore;
mod unit_entry;
mod unit_manager;
mod unit_runtime;

pub use serde::{Deserialize, Deserializer};

pub trait DeserializeWith: Sized {
    fn deserialize_with<'de, D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>;
}

impl DeserializeWith for Vec<String> {
    fn deserialize_with<'de, D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(de)?;
        let mut vec = Vec::new();

        for l in s.split_whitespace() {
            vec.push(l.to_string());
        }

        Ok(vec)
    }
}
