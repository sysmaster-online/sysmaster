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

//!
use crate::error::*;
use nix::{
    fcntl::AtFlags,
    sys::stat::{fstatat, SFlag},
};

///
pub fn mount_point_fd_valid(fd: i32, file_name: &str, flags: AtFlags) -> Result<bool> {
    assert!(fd >= 0);

    let flags = if flags.contains(AtFlags::AT_SYMLINK_FOLLOW) {
        flags & !AtFlags::AT_SYMLINK_FOLLOW
    } else {
        flags | AtFlags::AT_SYMLINK_FOLLOW
    };

    let f_stat = fstatat(fd, file_name, flags).context(NixSnafu)?;
    if SFlag::S_IFLNK.bits() & f_stat.st_mode == SFlag::S_IFLNK.bits() {
        return Ok(false);
    }

    let d_stat = fstatat(fd, "", AtFlags::AT_EMPTY_PATH).context(NixSnafu)?;

    if f_stat.st_dev == d_stat.st_dev && f_stat.st_ino == d_stat.st_ino {
        return Ok(true);
    }

    Ok(f_stat.st_dev != d_stat.st_dev)
}
