// Copyright (c) 2022 Huawei Technologies Co.,Ltd. All rights reserved.
//
// sysMaster is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan
// PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY
// KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

//! utils of libdevmaster
//!
use snafu::prelude::*;

pub(crate) type Result<T, E = Error> = std::result::Result<T, E>;

/// devmaster error
#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
#[non_exhaustive]
#[allow(missing_docs)]
pub enum Error {
    /// Error from device
    #[snafu(context)]
    Device { source: device::error::Error },

    #[snafu(display("filename: {}, error: {}", filename, source))]
    Io {
        filename: String,
        source: std::io::Error,
    },

    #[snafu(context)]
    ReadTooShort { filename: String },

    #[snafu(display("Fail to access : {}, error: {}", filename, source))]
    FailToAccess {
        filename: String,
        source: device::error::Error,
    },

    #[snafu(display("Fail to get devtype : {}", source))]
    FailToGetDevType { source: device::error::Error },

    #[snafu(display("Fail to get sysattr : {}", source))]
    GetSysAttr { source: device::error::Error },

    #[snafu(display("Fail to sscanf : {}", source))]
    FailToSscanf { source: sscanf::Error },

    #[snafu(display("Fail to get sysattr : {}", source))]
    ParseInt { source: std::num::ParseIntError },

    #[snafu(context)]
    CorruptData { filename: String },

    #[snafu(display("sys_path not found"))]
    SysPathNotFound,

    #[snafu(display("sys_name not found"))]
    SysNameNotFound,

    /// Error in worker manager
    #[snafu(display("Worker Manager: {}", msg))]
    WorkerManagerError { msg: &'static str },

    /// Error in job queue
    #[snafu(display("Job Queue: {}", msg))]
    JobQueueError { msg: &'static str },

    /// Error in control manager
    #[snafu(display("Control Manager: {}", msg))]
    ControlManagerError { msg: &'static str },

    /// Error encountered in builtin commands
    #[snafu(display("Builtin: {}", msg))]
    BuiltinCommandError { msg: String },

    /// Error encountered in rules loader
    #[snafu(display("Failed to load rule: {}", msg))]
    RulesLoadError { msg: String },

    /// Error encountered in rules loader
    #[snafu(display("Failed to execute rule: {}", msg))]
    RulesExecuteError {
        msg: String,
        /// error number
        errno: nix::errno::Errno,
    },

    /// Errors that can be ignored
    #[snafu(display("Ignore error: {}", msg))]
    IgnoreError { msg: String },

    /// Other errors
    #[snafu(display("Other error: {}", msg))]
    Other {
        /// error message
        msg: String,
        /// error number
        errno: nix::errno::Errno,
    },
}

impl Error {
    pub(crate) fn get_errno(&self) -> nix::errno::Errno {
        match self {
            Self::RulesExecuteError { msg: _, errno: n } => *n,
            Self::Other { msg: _, errno: n } => *n,
            _ => nix::errno::Errno::EINVAL,
        }
    }
}
