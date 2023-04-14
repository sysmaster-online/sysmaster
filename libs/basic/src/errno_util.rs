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

//! utility for checking errno
//!

use nix::errno::Errno;

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
