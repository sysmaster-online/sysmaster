use std::{collections::HashMap, path::Path};

pub use execute::{ExecCmdError, ExecCommand, ExecContext, ExecFlags, ExecParameters};
pub use unit_base::{
    KillOperation, UnitActionError, UnitDependencyMask, UnitRelationAtom, UnitType,
};
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
// unit_rentry -> unit_entry ->
// {unit_datastore -> unit_runtime} ->
// {job | execute} -> unit_manager

mod execute;
mod job;
mod uload_util;
mod unit_base;
mod unit_datastore;
mod unit_entry;
mod unit_manager;
mod unit_rentry;
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

        for l in s.split_terminator(';') {
            vec.push(l.trim().to_string());
        }

        Ok(vec)
    }
}

impl DeserializeWith for Vec<ExecCommand> {
    fn deserialize_with<'de, D>(de: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(de)?;

        let mut vec = vec![];

        for cmd in s.trim().split_terminator(';') {
            if cmd.is_empty() {
                continue;
            }

            let mut command: Vec<String> = cmd
                .trim()
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();

            // get the command and leave the command args
            let exec_cmd = command.remove(0);
            let path = Path::new(&exec_cmd);

            if path.is_absolute() && !path.exists() {
                log::debug!("{:?} is not exist in parse!", path);
                continue;
            }

            let cmd = path.to_str().unwrap().to_string();
            let new_command = ExecCommand::new(cmd, command);
            vec.push(new_command);
        }

        Ok(vec)
    }
}
