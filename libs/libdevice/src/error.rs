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

//! Error definition of libdevice
//!
use nix::errno::Errno;
use snafu::prelude::Snafu;

/// libdevice error
#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
#[non_exhaustive]
pub enum Error {
    /// error from syscall
    #[snafu(display(
        "Error(libdevice): Got an error for syscall {} (errno={})",
        syscall,
        errno,
    ))]
    Syscall {
        /// syscall
        syscall: String,
        /// errno
        errno: Errno,
    },

    /// other error
    #[snafu(display("Error(libdevice): Got an error {}", msg,))]
    Other {
        /// message
        msg: String,
        /// errno
        errno: Option<Errno>,
    },
}
