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
use nix::{
    errno::Errno,
    libc::{mode_t, S_IFBLK, S_IFCHR},
    sys::stat::makedev,
};
use std::path::Path;

/// given a device path, extract its mode and devnum
/// e.g. input /dev/block/8:0, output (S_IFBLK, makedev(8,0))
pub fn device_path_parse_major_minor(path: String) -> Result<(mode_t, u64), Errno> {
    let mode = if path.starts_with("/dev/block/") {
        S_IFBLK
    } else if path.starts_with("/dev/char/") {
        S_IFCHR
    } else {
        return Err(Errno::ENODEV);
    };

    let filename = match Path::new(&path).file_name() {
        Some(name) => match name.to_str() {
            Some(s) => s.to_string(),
            None => {
                return Err(Errno::EINVAL);
            }
        },
        None => {
            return Err(Errno::EINVAL);
        }
    };

    let tokens: Vec<&str> = filename.split(':').collect();

    let (major, minor) = (
        match tokens[0].parse::<u64>() {
            Ok(n) => n,
            Err(_) => {
                return Err(Errno::EINVAL);
            }
        },
        match tokens[1].parse::<u64>() {
            Ok(n) => n,
            Err(_) => {
                return Err(Errno::EINVAL);
            }
        },
    );

    Ok((mode, makedev(major, minor)))
}
