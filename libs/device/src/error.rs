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

//! Error definition of device
//!
use basic::IN_SET;
use nix::errno::Errno;
use snafu::prelude::Snafu;

/// libdevice error
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
#[non_exhaustive]
#[allow(missing_docs)]
pub enum Error {
    #[snafu(context, display("Device error: {}", msg))]
    Nix { msg: String, source: nix::Error },

    #[snafu(context, display("IO error: {}", msg))]
    Io { msg: String, source: std::io::Error },

    #[snafu(context, display("Basic error: {}", msg))]
    Basic { msg: String, source: basic::Error },

    #[snafu(context, display("Failed to parse boolean: {}", msg))]
    ParseBool {
        msg: String,
        source: std::str::ParseBoolError,
    },

    #[snafu(context, display("Failed to parse integer: {}", msg))]
    ParseInt {
        msg: String,
        source: std::num::ParseIntError,
    },

    #[snafu(context, display("Failed to parse utf-8: {}", msg))]
    FromUtf8 {
        msg: String,
        source: std::string::FromUtf8Error,
    },
}

impl Error {
    /// extract the errno from error
    pub fn get_errno(&self) -> Errno {
        match self {
            Self::Nix {
                msg: _,
                source: errno,
            } => *errno,
            Self::Io {
                msg: _,
                source: errno,
            } => Errno::from_i32(errno.raw_os_error().unwrap_or_default()),
            Self::Basic { msg: _, source } => Errno::from_i32(source.get_errno()),
            Self::ParseBool { msg: _, source: _ } => nix::Error::EINVAL,
            Self::ParseInt { msg: _, source: _ } => nix::Error::EINVAL,
            Self::FromUtf8 { msg: _, source: _ } => nix::Error::EINVAL,
        }
    }
}

impl Error {
    /// check whether the device error belongs to specific errno
    pub fn is_errno(&self, errno: nix::Error) -> bool {
        self.get_errno() == errno
    }

    /// check whether the device error indicates the device is absent
    pub fn is_absent(&self) -> bool {
        IN_SET!(self.get_errno(), Errno::ENODEV, Errno::ENXIO, Errno::ENOENT)
    }

    pub(crate) fn replace_errno(self, from: Errno, to: Errno) -> Self {
        let n = self.get_errno();

        if n == from {
            Self::Nix {
                msg: self.to_string(),
                source: to,
            }
        } else {
            self
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use nix::errno::Errno;

    #[test]
    fn test_replace_errno() {
        let e = Error::Nix {
            msg: "test".to_string(),
            source: Errno::ENOENT,
        };

        assert_eq!(
            Errno::ENOEXEC,
            e.replace_errno(Errno::ENOENT, Errno::ENOEXEC).get_errno(),
        );
    }
}
