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

//! initrd utils

use nix::sys::statfs::{self, FsType};
use std::path::Path;

#[cfg(target_env = "musl")]
type FsTypeT = libc::c_ulong;

#[cfg(not(target_env = "musl"))]
type FsTypeT = libc::c_long;

/// Whether in initrd
pub fn in_initrd(path: Option<&str>) -> bool {
    let path = path.map_or("/", |path| path);
    let is_tmpfs = statfs::statfs(path).map_or(false, |s| {
        s.filesystem_type() == FsType(libc::TMPFS_MAGIC as FsTypeT)
    });

    let has_initrd_release = Path::new("/etc/initrd-release").exists();

    is_tmpfs && has_initrd_release
}
