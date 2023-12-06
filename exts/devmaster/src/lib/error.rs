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
use crate::log_dev;
use device::Device;
use snafu::prelude::*;
use std::{ffi::OsString, rc::Rc, str::Utf8Error};

pub(crate) type Result<T, E = Error> = std::result::Result<T, E>;

/// devmaster error
#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
#[non_exhaustive]
#[allow(missing_docs)]
pub enum Error {
    /// Error from device
    #[snafu(display("Device error: {}", source))]
    Device { source: device::error::Error },
    #[snafu(display("Failed to IO on '{}': {:?}", filename, source))]
    Io {
        filename: String,
        source: std::io::Error,
    },
    #[snafu(context)]
    ReadTooShort { filename: String },
    #[snafu(display("Failed to parse integer: {}", source))]
    ParseInt { source: std::num::ParseIntError },
    #[snafu(display("Failed to parse float: {}", source))]
    ParseFloat { source: std::num::ParseFloatError },
    #[snafu(display("Failed to parse boolean: {}", source))]
    ParseBool { source: std::str::ParseBoolError },
    #[snafu(display("Failed to parse shell words: {}", source))]
    ParseShellWords { source: shell_words::ParseError },

    #[snafu(display("Nix error: {}", source))]
    Nix { source: nix::Error },

    #[snafu(display("libbasic error: {}", source))]
    Basic { source: basic::Error },

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

    #[snafu(display("Invalid utf8 string: {}", source))]
    Utf8Error { source: Utf8Error },

    #[snafu(display("Rtnetlink error: {}", source))]
    Rtnetlink { source: rtnetlink::Error },

    #[snafu(display("Failed to deserialize .toml: {}", source))]
    Toml { source: toml::de::Error },

    #[snafu(display("Invalid OsString: {:?}", s))]
    InvalidOsString { s: OsString },

    #[snafu(display("Invalid subsystem kernel string: {}", s))]
    InvalidSubsystemKernel { s: String },
}

impl Error {
    pub(crate) fn get_errno(&self) -> nix::errno::Errno {
        match self {
            Self::RulesExecuteError { msg: _, errno: n } => *n,
            Self::Io {
                filename: _,
                source,
            } => nix::errno::from_i32(source.raw_os_error().unwrap_or_default()),
            Self::Device { source } => source.get_errno(),
            Self::Nix { source } => *source,
            Self::Other { msg: _, errno } => *errno,
            Self::Rtnetlink {
                source: rtnetlink::Error::NetlinkError(msg),
            } => nix::errno::Errno::from_i32(i32::abs(msg.code)),
            _ => nix::errno::Errno::EINVAL,
        }
    }
}

pub(crate) trait Log {
    fn log_error(self, msg: &str) -> Self;
    fn log_debug(self, msg: &str) -> Self;
    fn log_info(self, msg: &str) -> Self;
    fn log_dev_error(self, dev: &Device, msg: &str) -> Self;
    fn log_dev_debug(self, dev: &Device, msg: &str) -> Self;
    fn log_dev_info(self, dev: &Device, msg: &str) -> Self;
    fn log_dev_error_option(self, dev: Option<Rc<Device>>, msg: &str) -> Self;
    fn log_dev_debug_option(self, dev: Option<Rc<Device>>, msg: &str) -> Self;
    fn log_dev_info_option(self, dev: Option<Rc<Device>>, msg: &str) -> Self;
}

impl<T> Log for std::result::Result<T, Error> {
    fn log_info(self, msg: &str) -> Self {
        self.map_err(|e| {
            log::info!("{}: {}", msg, e);
            e
        })
    }
    fn log_debug(self, msg: &str) -> Self {
        self.map_err(|e| {
            log::debug!("{}: {}", msg, e);
            e
        })
    }
    fn log_error(self, msg: &str) -> Self {
        self.map_err(|e| {
            log::error!("{}: {}", msg, e);
            e
        })
    }
    fn log_dev_error(self, dev: &Device, msg: &str) -> Self {
        self.map_err(|e| {
            log_dev!(error, dev, format!("{}: {}", msg, e));
            e
        })
    }
    fn log_dev_debug(self, dev: &Device, msg: &str) -> Self {
        self.map_err(|e| {
            log_dev!(debug, dev, format!("{}: {}", msg, e));
            e
        })
    }
    fn log_dev_info(self, dev: &Device, msg: &str) -> Self {
        self.map_err(|e| {
            log_dev!(info, dev, format!("{}: {}", msg, e));
            e
        })
    }
    fn log_dev_error_option(self, dev: Option<Rc<Device>>, msg: &str) -> Self {
        self.map_err(|e| {
            match dev {
                Some(d) => log_dev!(error, d, format!("{}: {}", msg, e)),
                None => log::error!("{}: {}", msg, e),
            }
            e
        })
    }
    fn log_dev_debug_option(self, dev: Option<Rc<Device>>, msg: &str) -> Self {
        self.map_err(|e| {
            match dev {
                Some(d) => log_dev!(debug, d, format!("{}: {}", msg, e)),
                None => log::error!("{}: {}", msg, e),
            }
            e
        })
    }
    fn log_dev_info_option(self, dev: Option<Rc<Device>>, msg: &str) -> Self {
        self.map_err(|e| {
            match dev {
                Some(d) => log_dev!(info, d, format!("{}: {}", msg, e)),
                None => log::error!("{}: {}", msg, e),
            }
            e
        })
    }
}
