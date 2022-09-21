//! unit是process1对系统服务进行管理抽象的主要模块
//! 该模块包含:
//! [execute]：unit要执行的对象数据结构定义。
//! [job]：unit对应的调度执行实体，每个unit在启动后，会通过job来驱动。
//! [uload_util]：每个unit配置文件相关的属性定义。
//! [unit_base]：unit相关对象基本属性的定义，如unit类型的枚举，unit依赖关系的定义
//! [unit_datastore]: unit对象存储模块，负责对unit模块状态进行存储。
//! [unit_entry]：unit相关对象的定义
//!
use std::path::Path;

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
