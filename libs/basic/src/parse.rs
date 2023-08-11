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
pub fn device_path_parse_devnum(path: &str) -> Result<(mode_t, u64)> {
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
                what: format!("invalid path '{}'", path),
            })
        }
    };

    Ok((mode, parse_devnum(&filename)?))
}

/// parse the major:minor like string, and return the devnum
pub fn parse_devnum(s: &str) -> Result<u64> {
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
pub fn parse_ifindex(s: &str) -> Result<u32> {
    s.parse::<u32>().map_err(|_| Error::Nix {
        source: nix::errno::Errno::EINVAL,
    })
}

/// parse the string into mode_t
pub fn parse_mode(mode: &str) -> Result<mode_t> {
    match mode_t::from_str_radix(mode, 8) {
        Ok(v) => {
            if v > 0o7777 {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_path_parse_devnum() {
        assert_eq!(
            (S_IFBLK, makedev(8, 100)),
            device_path_parse_devnum("/dev/block/8:100").unwrap()
        );
        assert_eq!(
            (S_IFCHR, makedev(9, 100)),
            device_path_parse_devnum("/dev/char/9:100").unwrap()
        );
        assert!(device_path_parse_devnum("invalid").is_err());
        assert!(device_path_parse_devnum("/dev/block/invalid").is_err());
        assert!(device_path_parse_devnum("/dev/char/invalid").is_err());
    }

    #[test]
    fn test_parse_ifindex() {
        assert_eq!(1, parse_ifindex("1").unwrap());
        assert!(parse_ifindex("a").is_err());
    }

    #[test]
    fn test_parse_mode() {
        assert_eq!(0o777, parse_mode("777").unwrap());
        assert_eq!(0o7777, parse_mode("7777").unwrap());
        assert!(parse_mode("7778").is_err());
        assert!(parse_mode("invalid").is_err());
    }
}
