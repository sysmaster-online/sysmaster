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

//! utility for stat
//!

use super::{Error, Result};
use nix::sys::statfs::{self, FsType};

#[cfg(any(
    all(target_os = "linux", not(target_env = "musl")),
    target_os = "android"
))]
use nix::sys::statfs::PROC_SUPER_MAGIC;

/// check whether a path is specific file system type
pub fn path_is_fs_type(path: &str, magic: FsType) -> Result<bool> {
    let s = statfs::statfs(path).map_err(|e| Error::Nix { source: e })?;

    Ok(s.filesystem_type() == magic)
}

/// check whether /proc is mounted
pub fn proc_mounted() -> Result<bool> {
    #[cfg(any(
        all(target_os = "linux", not(target_env = "musl")),
        target_os = "android"
    ))]
    match path_is_fs_type("/proc/", PROC_SUPER_MAGIC) {
        Ok(r) => return Ok(r),
        Err(e) => match e {
            Error::Nix {
                source: nix::errno::Errno::ENOENT,
            } => {}
            _ => return Err(e),
        },
    }

    Ok(false)
}
