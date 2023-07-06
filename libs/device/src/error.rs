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
use nix::errno::Errno;
use snafu::prelude::Snafu;

/// libdevice error
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
#[non_exhaustive]
pub enum Error {
    /// other error
    #[snafu(context, display("Device error: {}", msg))]
    Nix {
        /// message
        msg: String,
        /// errno indicates the error kind
        source: nix::Error,
    },
}

impl Error {
    /// extract the errno from error
    pub fn get_errno(&self) -> Errno {
        match self {
            Error::Nix {
                msg: _,
                source: errno,
            } => *errno,
        }
    }
}

/// append current function and inherit the errno
#[macro_export]
macro_rules! err_wrapper {
    ($e:expr, $s:expr) => {
        $e.map_err(|e| Error::Nix {
            msg: format!("$s failed: {}", e),
            source: e.get_errno(),
        })
    };
}

impl Error {
    /// check whether the device error belongs to specific errno
    pub fn is_errno(&self, errno: nix::Error) -> bool {
        match self {
            Self::Nix { msg: _, source } => *source == errno,
        }
    }

    /// check whether the device error indicates the device is absent
    pub fn is_absent(&self) -> bool {
        match self {
            Self::Nix { msg: _, source } => {
                matches!(source, Errno::ENODEV | Errno::ENXIO | Errno::ENOENT)
            }
        }
    }
}
