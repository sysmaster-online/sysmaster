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

/// libcore Error:
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

    #[snafu(display("Confique error"))]
    Confique {
        source: confique::Error,
    },

    #[snafu(display("cgroup error:{}", source))]
    Cgroup {
        source: cgroup::error::Error,
    },

    #[snafu(display("VarError(libcore)"))]
    Var {
        source: std::env::VarError,
    },

    #[snafu(display("UtilError(libcore)"))]
    Util {
        source: basic::error::Error,
    },

    #[snafu(display("IoError(libcore)"))]
    Io {
        source: std::io::Error,
    },

    #[snafu(display("FmtError(libcore)"))]
    Fmt {
        source: std::fmt::Error,
    },

    #[snafu(display("NixError(libcore)"))]
    Nix {
        source: nix::Error,
    },

    #[snafu(display("HeedError(libcore)"))]
    Heed {
        source: heed::Error,
    },

    #[snafu(display("InvalidData(libcore)"))]
    InvalidData,

    #[snafu(display("NotFound(libcore): '{}'.", what))]
    NotFound {
        what: String,
    },

    #[snafu(display("OtherError(libcore): '{}'.", msg))]
    Other {
        msg: String,
    },

    #[snafu(display("Shared"))]
    Shared {
        source: Arc<Error>,
    },

    #[snafu(display("Invalid Name: {}", what))]
    InvalidName {
        what: String,
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

    /// events error
    #[snafu(display("event error; '{}'.", msg))]
    EventError {
        msg: String,
    },

    /// Error for exec command
    #[snafu(display("Timeout(ExecCmdError)"))]
    Timeout,
    #[snafu(display("NoCmdFound(ExecCmdError)"))]
    NoCmdFound,
    #[snafu(display("SpawnError(ExecCmdError)"))]
    SpawnError,
    #[snafu(display("load unit error '{}'.", msg))]
    LoadError {
        msg: String,
    },

    #[snafu(display("unit configuration error: '{}'.", msg))]
    ConfigureError {
        msg: String,
    },

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

/// Convert to the standard linux error code
impl From<Error> for nix::Error {
    fn from(e: Error) -> Self {
        match e {
            Error::EOF => nix::Error::EIO,
            Error::Parse { source: _ } => nix::Error::EINVAL,
            Error::PluginLoad { msg: _ } => nix::Error::EIO,
            Error::Confique { source: _ } => nix::Error::EINVAL,
            Error::Cgroup { source: _ } => nix::Error::EIO,
            Error::Var { source: _ } => nix::Error::EINVAL,
            Error::Util { source: _ } => nix::Error::EINVAL,
            Error::Io { source: _ } => nix::Error::EIO,
            Error::Fmt { source: _ } => nix::Error::EIO,
            Error::Nix { source } => source,
            Error::Heed { source: _ } => nix::Error::EIO,
            Error::InvalidData => nix::Error::EINVAL,
            Error::NotFound { what: _ } => nix::Error::ENOENT,
            Error::Other { msg: _ } => nix::Error::EIO,
            Error::Shared { source: _ } => nix::Error::EIO,
            Error::InvalidName { what: _ } => nix::Error::EINVAL,
            Error::ConvertToSysmaster => nix::Error::EIO,
            Error::Input => nix::Error::EIO,
            Error::Conflict => nix::Error::EBADR,
            Error::NotExisted => nix::Error::ENOENT,
            Error::Internal => nix::Error::EIO,
            Error::NotSupported => nix::Error::ENOTSUP,
            Error::BadRequest => nix::Error::EBADR,
            Error::Timeout => nix::Error::ETIMEDOUT,
            Error::NoCmdFound => nix::Error::ENOENT,
            Error::SpawnError => nix::Error::EIO,
            Error::UnitActionEAgain => nix::Error::EAGAIN,
            Error::UnitActionEAlready => nix::Error::EALREADY,
            Error::UnitActionEComm => nix::Error::ECOMM,
            Error::UnitActionEBadR => nix::Error::EBADR,
            Error::UnitActionENoExec => nix::Error::ENOEXEC,
            Error::UnitActionEProto => nix::Error::EPROTO,
            Error::UnitActionEOpNotSupp => nix::Error::ENOTSUP,
            Error::UnitActionENolink => nix::Error::ENOLINK,
            Error::UnitActionEStale => nix::Error::ESTALE,
            Error::UnitActionEFailed => nix::Error::EIO,
            Error::UnitActionEInval => nix::Error::EINVAL,
            Error::UnitActionEBusy => nix::Error::EBUSY,
            Error::UnitActionENoent => nix::Error::ENOENT,
            Error::UnitActionECanceled => nix::Error::ECANCELED,
            Error::UnitActionERefuseManualStart => nix::Error::EINVAL,
            Error::UnitActionERefuseManualStop => nix::Error::EINVAL,

            Error::ConfigureError { msg: _ } => nix::Error::EINVAL,
            Error::LoadError { msg: _ } => nix::Error::EIO,
            Error::EventError { msg: _ } => nix::Error::EIO,
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

impl From<event::Error> for Error {
    fn from(source: event::Error) -> Error {
        Error::EventError {
            msg: format!("{:?}", source),
        }
    }
}

/// new Result
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// check if the error is disconnect
pub fn error_is_disconnect(e: &Errno) -> bool {
    [
        Errno::ECONNABORTED,
        Errno::ECONNREFUSED,
        Errno::ECONNRESET,
        Errno::EHOSTDOWN,
        Errno::EHOSTUNREACH,
        Errno::ENETDOWN,
        Errno::ENETRESET,
        Errno::ENONET,
        Errno::ENOPROTOOPT,
        Errno::ENOTCONN,
        Errno::EPIPE,
        Errno::EPROTO,
        Errno::ESHUTDOWN,
        Errno::ETIMEDOUT,
    ]
    .contains(e)
}

/// check if the error is transient
pub fn error_is_transient(e: &Errno) -> bool {
    [Errno::EAGAIN, Errno::EINTR].contains(e)
}

/// check if the error is accept or again
pub fn error_is_accept_again(e: &Errno) -> bool {
    error_is_disconnect(e) || error_is_transient(e) || e == &Errno::EOPNOTSUPP
}
