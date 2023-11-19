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

//! error definitions
use nix::errno::Errno;
use snafu::prelude::*;
#[allow(unused_imports)]
pub use snafu::ResultExt;

#[allow(missing_docs)]
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
#[non_exhaustive]
pub enum Error {
    #[snafu(display(
        "Got an error: (ret={}, errno={}) for syscall: {}",
        ret,
        errno,
        syscall
    ))]
    Syscall {
        syscall: &'static str,
        ret: i32,
        errno: i32,
    },

    #[snafu(display("Io: {}", source))]
    Io { source: std::io::Error },

    #[snafu(display("Caps: {}", what))]
    Caps { what: String },

    #[snafu(display("Errno: {}", source))]
    Nix { source: nix::Error },

    #[snafu(display("Var: {}", source))]
    Var { source: std::env::VarError },

    #[cfg(feature = "process")]
    #[snafu(display("procfs: {}", source))]
    Proc { source: procfs::ProcError },

    #[snafu(display("NulError: '{}'", source))]
    NulError { source: std::ffi::NulError },

    #[snafu(display("Error parsing from string: {}", source))]
    Parse {
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[snafu(display("Invalid naming scheme string: {}", what))]
    ParseNamingScheme { what: String },

    #[snafu(display("Not exist: '{}'.", what))]
    NotExisted { what: String },

    #[snafu(display("Invalid: '{}'.", what))]
    Invalid { what: String },

    #[snafu(display("OtherError: '{}'.", msg))]
    Other { msg: String },
}

impl Error {
    /// Translate the basic error to error number.
    pub fn get_errno(&self) -> i32 {
        match self {
            Self::Syscall {
                syscall: _,
                ret: _,
                errno,
            } => *errno,
            Error::Io { source } => source.raw_os_error().unwrap_or_default(),
            Error::Caps { what: _ } => nix::errno::Errno::EINVAL as i32,
            Error::Nix { source } => *source as i32,
            Error::Var { source } => {
                (match source {
                    std::env::VarError::NotPresent => nix::errno::Errno::ENOENT,
                    std::env::VarError::NotUnicode(_) => nix::errno::Errno::EINVAL,
                }) as i32
            }
            #[cfg(feature = "process")]
            Error::Proc { source } => match source {
                procfs::ProcError::Incomplete(_) => nix::errno::Errno::EINVAL as i32,
                procfs::ProcError::PermissionDenied(_) => nix::errno::Errno::EPERM as i32,
                procfs::ProcError::NotFound(_) => nix::errno::Errno::ENOENT as i32,
                procfs::ProcError::Io(_, _) => nix::errno::Errno::EIO as i32,
                procfs::ProcError::Other(_) => nix::errno::Errno::EINVAL as i32,
                procfs::ProcError::InternalError(_) => nix::errno::Errno::EINVAL as i32,
            },
            Error::NulError { source: _ } => nix::errno::Errno::EINVAL as i32,
            Error::Parse { source: _ } => nix::errno::Errno::EINVAL as i32,
            Error::ParseNamingScheme { what: _ } => nix::errno::Errno::EINVAL as i32,
            Error::NotExisted { what: _ } => nix::errno::Errno::ENOENT as i32,
            Error::Invalid { what: _ } => nix::errno::Errno::EINVAL as i32,
            Error::Other { msg: _ } => nix::errno::Errno::EINVAL as i32,
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

errfrom!(std::num::ParseIntError, std::string::ParseError, std::num::ParseFloatError, std::str::ParseBoolError, std::string::FromUtf8Error => Parse);

///
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// seven errno for "operation, system call, ioctl or socket feature not supported"
pub fn errno_is_not_supported(source: Errno) -> bool {
    matches!(
        source,
        Errno::EOPNOTSUPP
            | Errno::ENOTTY
            | Errno::ENOSYS
            | Errno::EAFNOSUPPORT
            | Errno::EPFNOSUPPORT
            | Errno::EPROTONOSUPPORT
            | Errno::ESOCKTNOSUPPORT
    )
}

/// two errno for access problems
pub fn errno_is_privilege(source: Errno) -> bool {
    matches!(source, Errno::EACCES | Errno::EPERM)
}

/// two errno to try again
pub fn errno_is_transient(source: Errno) -> bool {
    matches!(source, Errno::EAGAIN | Errno::EINTR)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_errno_is_not_supported() {
        assert!(errno_is_not_supported(nix::Error::EOPNOTSUPP));
        assert!(errno_is_not_supported(nix::Error::ENOTTY));
        assert!(errno_is_not_supported(nix::Error::ENOSYS));
        assert!(errno_is_not_supported(nix::Error::EAFNOSUPPORT));
        assert!(errno_is_not_supported(nix::Error::EPFNOSUPPORT));
        assert!(errno_is_not_supported(nix::Error::EPROTONOSUPPORT));
        assert!(errno_is_not_supported(nix::Error::ESOCKTNOSUPPORT));
    }

    #[test]
    fn test_errno_is_privilege() {
        assert!(errno_is_privilege(nix::Error::EACCES));
        assert!(errno_is_privilege(nix::Error::EPERM));
    }

    #[test]
    fn test_errno_is_transient() {
        assert!(errno_is_transient(nix::Error::EAGAIN));
        assert!(errno_is_transient(nix::Error::EINTR));
        assert!(!errno_is_transient(nix::Error::EACCES));
    }
}
