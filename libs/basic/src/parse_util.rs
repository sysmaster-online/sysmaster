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
pub fn device_path_parse_devnum(path: String) -> Result<(mode_t, u64)> {
    let mode = if path.starts_with("/dev/block/") {
        S_IFBLK
    } else if path.starts_with("/dev/char/") {
        S_IFCHR
    } else {
        return Err(Error::Nix {
            source: nix::errno::Errno::ENODEV,
        });
    };

    let filename = match Path::new(&path).file_name() {
        Some(s) => s.to_string_lossy().to_string(),
        None => {
            return Err(Error::Invalid {
                what: format!("invalid path {}", path),
            })
        }
    };

    Ok((mode, parse_devnum(filename)?))
}

/// parse the major:minor like string, and return the devnum
pub fn parse_devnum(s: String) -> Result<u64> {
    let tokens: Vec<&str> = s.split(':').collect();
    if tokens.len() != 2 {
        return Err(Error::Invalid {
            what: format!("incorrect number of tokens: {}", s),
        });
    }

    let (major, minor) = (
        tokens[0].parse::<u64>().map_err(|_| Error::Invalid {
            what: format!("invalid major: {}", tokens[0]),
        })?,
        tokens[1].parse::<u64>().map_err(|_| Error::Invalid {
            what: format!("invalid minor: {}", tokens[1]),
        })?,
    );

    Ok(makedev(major, minor))
}

/// parse the numeric like string into ifindex number
pub fn parse_ifindex(s: String) -> Result<u32> {
    s.parse::<u32>().map_err(|_| Error::Nix {
        source: nix::errno::Errno::EINVAL,
    })
}

/// parse the string into mode_t
pub fn parse_mode(mode: &str) -> Result<mode_t> {
    match mode.parse::<mode_t>() {
        Ok(v) => {
            if v > 7777 {
                return Err(Error::Nix {
                    source: nix::errno::Errno::ERANGE,
                });
            }

            Ok(v)
        }
        Err(e) => Err(Error::Parse {
            source: Box::new(e),
        }),
    }
}
