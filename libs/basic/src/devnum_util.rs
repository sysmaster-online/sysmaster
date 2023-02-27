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

//! the utils can be used to deal with devnum
use crate::error::*;
use nix::{
    libc::{mode_t, S_IFBLK, S_IFCHR},
    sys::stat::makedev,
};
use std::path::Path;

/// given a device path, extract its mode and devnum
/// e.g. input /dev/block/8:0, output (S_IFBLK, makedev(8,0))
pub fn device_path_parse_major_minor(path: String) -> Result<(mode_t, u64)> {
    let mode = if path.starts_with("/dev/block/") {
        S_IFBLK
    } else if path.starts_with("/dev/char/") {
        S_IFCHR
    } else {
        return Err(Error::Nix {
            source: nix::errno::Errno::ENODEV,
        });
    };

    let filename = Path::new(&path).to_string_lossy().to_string();
    let tokens: Vec<&str> = filename.split(':').collect();

    let (major, minor) = (
        tokens[0].parse::<u64>().map_err(|_| Error::Nix {
            source: nix::errno::Errno::EINVAL,
        })?,
        tokens[1].parse::<u64>().map_err(|_| Error::Nix {
            source: nix::errno::Errno::EINVAL,
        })?,
    );

    Ok((mode, makedev(major, minor)))
}
