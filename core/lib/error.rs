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

//! Error define, There is no globally defined error library, and each crate defines its own error.rs.
//! Within a crate, only this unified Error can be used, and attention should be paid to avoiding semantic duplication.
//! In sysmaster, the unit  components and sysmaster-core share one Error in terms of logic and functionality to avoid frequent conversions.
//! todo: We can re-split it in the future to make it at a reasonable granularity, such as defining an error for each crate.

/// Reuse the Errno from the nix library:
/// Errno is an enumeration type that defines error codes that may be returned by Linux system calls
/// and other system interfaces. The nix library provides an implementation of this type and its associated error messages.
pub use nix::errno::Errno;
use snafu::prelude::*;
#[allow(unused_imports)]
pub use snafu::ResultExt;
/// Reuse the Errorkind from the std::io library:
/// Errorkind is an enumeration type defined in the std::io library that represents different types of errors
/// that can occur during input and output operations. Reusing Errorkind can provide a consistent error handling
/// mechanism across different parts of the codebase.
pub use std::io::ErrorKind;
use std::sync::Arc;

/// Libsysmaster Error:
/// Here, errors inherited from underlying crates (such as nix/io, etc.) are utilized, and some new unique error codes
/// related to the sysmaster project are defined.
#[allow(missing_docs)]
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
#[non_exhaustive]
pub enum Error {
    #[snafu(display("Unexpected end of file"))]
    EOF,

    #[snafu(display("Error parsing from string: {}", source))]
    Parse {
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[snafu(display("plugin load error"))]
    PluginLoad {
        msg: String,
    },

    #[snafu(display("Confique  error"))]
    Confique {
        source: confique::Error,
    },

    #[snafu(display("cgroup error:{}", source))]
    Cgroup {
        source: cgroup::error::Error,
    },

    #[snafu(display("VarError(libsysmaster)"))]
    Var {
        source: std::env::VarError,
    },

    #[snafu(display("UtilError(libsysmaster)"))]
    Util {
        source: basic::error::Error,
    },

    #[snafu(display("IoError(libsysmaster)"))]
    Io {
        source: std::io::Error,
    },

    #[snafu(display("NixError(libsysmaster)"))]
    Nix {
        source: nix::Error,
    },

    #[snafu(display("HeedError(libsysmaster)"))]
    Heed {
        source: heed::Error,
    },

    #[snafu(display("InvalidData(libsysmaster)"))]
    InvalidData,

    #[snafu(display("NotFound(libsysmaster): '{}'.", what))]
    NotFound {
        what: String,
    },

    #[snafu(display("OtherError(libsysmaster): '{}'.", msg))]
    Other {
        msg: String,
    },

    #[snafu(display("Shared"))]
    Shared {
        source: Arc<Error>,
    },

    #[snafu(display("ConvertError"))]
    ConvertToSysmaster,

    /// Job errno
    Input,
    Conflict,
    NotExisted,
    Internal,
    NotSupported,
    BadRequest,

    /// Error for exec command
    #[snafu(display("Timeout(ExecCmdError)"))]
    Timeout,
    #[snafu(display("NoCmdFound(ExecCmdError)"))]
    NoCmdFound,
    #[snafu(display("SpawnError(ExecCmdError)"))]
    SpawnError,

    /// UnitAction Error
    #[snafu(display("EAgain(UnitActionError)"))]
    UnitActionEAgain,
    #[snafu(display("EAlready(UnitActionError)"))]
    UnitActionEAlready,
    #[snafu(display("EComm(UnitActionError)"))]
    UnitActionEComm,
    #[snafu(display("EBadR(UnitActionError)"))]
    UnitActionEBadR,
    #[snafu(display("ENoExec(UnitActionError)"))]
    UnitActionENoExec,
    #[snafu(display("EProto(UnitActionError)"))]
    UnitActionEProto,
    #[snafu(display("EOpNotSupp(UnitActionError)"))]
    UnitActionEOpNotSupp,
    #[snafu(display("ENolink(UnitActionError)"))]
    UnitActionENolink,
    #[snafu(display("EStale(UnitActionError)"))]
    UnitActionEStale,
    #[snafu(display("EFailed(UnitActionError)"))]
    UnitActionEFailed,
    #[snafu(display("EInval(UnitActionError)"))]
    UnitActionEInval,
    #[snafu(display("EBusy(UnitActionError)"))]
    UnitActionEBusy,
    #[snafu(display("ENoent(UnitActionError)"))]
    UnitActionENoent,
    #[snafu(display("ECanceled(UnitActionError)"))]
    UnitActionECanceled,
    #[snafu(display("unit can not be started manually"))]
    UnitActionERefuseManualStart,
    #[snafu(display("unit can not be stopped manually"))]
    UnitActionERefuseManualStop,
}

impl From<Error> for nix::errno::Errno {
    fn from(e: Error) -> Self {
        match e {
            Error::Nix { source } => source,
            _ => nix::errno::Errno::ENOTSUP,
        }
    }
}

impl From<Error> for std::io::Error {
    fn from(e: Error) -> Self {
        match e {
            Error::Io { source } => source,
            _ => std::io::ErrorKind::Other.into(),
        }
    }
}

#[allow(unused_macros)]
macro_rules! errfrom {
    ($($st:ty),* => $variant:ident) => (
        $(
            impl From<$st> for Error {
                fn from(e: $st) -> Error {
                    Error::$variant { source: e.into() }
                }
            }
        )*
    )
}

errfrom!(std::num::ParseIntError, std::string::ParseError => Parse);
errfrom!(nix::errno::Errno => Nix);

impl From<basic::error::Error> for Error {
    fn from(e: basic::Error) -> Error {
        match e {
            basic::Error::Io { source } => Error::Io { source },
            basic::Error::Nix { source } => Error::Nix { source },
            _ => Error::Other {
                msg: "unspport".to_string(),
            },
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(source: std::io::Error) -> Error {
        Error::Io { source }
    }
}

impl From<String> for Error {
    fn from(msg: String) -> Error {
        Error::Other { msg }
    }
}

impl From<Arc<Error>> for Error {
    fn from(source: Arc<Error>) -> Error {
        Error::Shared { source }
    }
}

/// new Result
pub type Result<T, E = Error> = std::result::Result<T, E>;
