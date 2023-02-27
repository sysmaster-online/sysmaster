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

//! Error define
use snafu::prelude::*;

/// Event Error
#[derive(Debug, Snafu)]
pub enum Error {
    /// An error from syscall
    #[snafu(display(
        "Error(event): : Got an error: (ret={}, errno={}) for syscall: {}",
        ret,
        errno,
        syscall
    ))]
    Syscall {
        /// string representation
        syscall: &'static str,
        /// return value
        ret: i32,
        /// errno
        errno: i32,
    },

    /// Other
    #[snafu(display("Error(event): '{}'.", word))]
    Other {
        /// some words
        word: &'static str,
    },
}

/// new Result
pub type Result<T, E = Error> = std::result::Result<T, E>;
