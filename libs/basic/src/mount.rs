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
use std::{
    fs::File,
    io::{self, BufRead, BufReader},
    os::unix::prelude::RawFd,
    path::{Path, PathBuf},
};

use crate::{
    error::*,
    fs::{chase_symlink, is_symlink},
};
use nix::{
    fcntl::AtFlags,
    sys::stat::{fstatat, SFlag},
};

///
pub fn mount_point_fd_valid(fd: RawFd, file_name: &str, flags: AtFlags) -> Result<bool> {
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

#[cfg(not(target_env = "musl"))]
/// check if the given path is a mount point, return true if yes, return false if no or fail.
pub fn is_mount_point(path: &Path) -> bool {
    use std::{ffi::CString, os::unix::prelude::AsRawFd};

    use libc::{statx, STATX_ATTR_MOUNT_ROOT};

    let file = match File::open(path) {
        Err(_) => {
            return false;
        }
        Ok(v) => v,
    };
    let fd = AsRawFd::as_raw_fd(&file);
    let path_name = CString::new(path.to_str().unwrap()).unwrap();
    let mut statxbuf: statx = unsafe { std::mem::zeroed() };
    unsafe {
        /* statx was added to linux in kernel 4.11 per `stat(2)`,
         * we can depend on it safely. So we only use statx to
         * check if the path is a mount point, and chase the
         * symlink unconditionally*/
        statx(fd, path_name.as_ptr(), 0, 0, &mut statxbuf);
        log::debug!(
            "{} attributes_mask {},stx_attributes{}",
            path.to_str().unwrap(),
            statxbuf.stx_attributes_mask & (STATX_ATTR_MOUNT_ROOT as u64),
            statxbuf.stx_attributes & (STATX_ATTR_MOUNT_ROOT as u64)
        );
        /* The mask is supported and is set */
        statxbuf.stx_attributes_mask & (STATX_ATTR_MOUNT_ROOT as u64) != 0
            && statxbuf.stx_attributes & (STATX_ATTR_MOUNT_ROOT as u64) != 0
    }
}

#[cfg(target_env = "musl")]
/// check if the given path is a mount point, return true if yes, return false if no or fail.
/* musl can't use statx, check /proc/self/mountinfo. */
pub fn is_mount_point(path: &Path) -> bool {
    use std::io::Read;

    let mut mount_data = String::new();
    let mut file = match File::open("/proc/self/mountinfo") {
        Err(_) => {
            return false;
        }
        Ok(v) => v,
    };
    if file.read_to_string(&mut mount_data).is_err() {
        return false;
    }
    let parser = MountInfoParser::new(mount_data);
    for mount in parser {
        if path.to_str().unwrap() == mount.mount_point {
            return true;
        }
    }
    false
}

/// check if the given path is a swap, return true if yes, return false if no or fail.
pub fn is_swap(path: &Path) -> bool {
    let device_path = if is_symlink(path) {
        match chase_symlink(path) {
            Err(_) => PathBuf::from(path),
            Ok(v) => v,
        }
    } else {
        PathBuf::from(path)
    };

    let file = match File::open("/proc/swaps") {
        Err(_) => return false,
        Ok(v) => v,
    };
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = match line {
            Err(_) => continue,
            Ok(v) => v,
        };
        if line.starts_with(device_path.to_str().unwrap()) {
            return true;
        }
    }
    false
}

/// MountParser is a parser used to parse /proc/PID/mountinfo
pub struct MountInfoParser {
    cur: usize,
    max_len: usize,
    s: String,
}

impl MountInfoParser {
    /// Create a new MountParser
    pub fn new(s: String) -> MountInfoParser {
        Self {
            cur: 0,
            max_len: s.as_bytes().len(),
            s,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
#[allow(dead_code)]
/// The mount point
pub struct MountInfo {
    /// unique identifier of the mount
    pub mount_id: u64,
    /// ID of parent
    pub parent_id: u64,
    /// value of st_dev for files on filesystem
    pub major: u64,
    /// value of st_dev for files on filesystem
    pub minor: u64,
    /// root of the mount within the filesystem
    pub root: String,
    /// mount point relative to the process's root
    pub mount_point: String,
    /// per mount options
    pub mount_options: String,
    /// zero or more fields of the form `tag[:value]`
    pub optional_fields: String,
    /// name of filesystem of the form `type[.subtype]`
    pub fstype: String,
    /// filesystem specific information or `none`
    pub mount_source: String,
    /// per super block options
    pub super_options: String,
}

impl Iterator for MountInfoParser {
    type Item = MountInfo;

    fn next(&mut self) -> Option<Self::Item> {
        let mbytes = self.s.as_bytes();
        if self.cur >= self.max_len {
            return None;
        }

        let mut cur = self.cur;
        let mut index = 0;
        let mut pre = cur;

        let mut mount_id = 0_u64;
        let mut parent_id = 0_u64;
        let mut major = 0_u64;
        let mut minor = 0_u64;
        let mut root = String::new();
        let mut mount_point = String::new();
        let mut mount_options = String::new();
        let mut optional_fields = String::new();
        let mut fstype = String::new();
        let mut mount_source = String::new();

        while mbytes[cur] != b'\n' {
            let cur_byte = mbytes[cur];
            if cur_byte != b' ' {
                cur += 1;
                continue;
            }

            let s = match std::str::from_utf8(&mbytes[pre..cur]) {
                Err(_) => return None,
                Ok(v) => v,
            };

            /* https://metacpan.org/pod/Linux::Proc::Mountinfo#Excerpt-from-Linux-documentation */
            match index {
                0 => {
                    mount_id = match s.parse::<u64>() {
                        Err(_) => return None,
                        Ok(v) => v,
                    };
                }
                1 => {
                    parent_id = match s.parse::<u64>() {
                        Err(_) => return None,
                        Ok(v) => v,
                    };
                }
                2 => {
                    let (major_str, minor_str) = match s.split_once(':') {
                        None => return None,
                        Some(v) => v,
                    };
                    major = match major_str.parse::<u64>() {
                        Err(_) => return None,
                        Ok(v) => v,
                    };
                    minor = match minor_str.parse::<u64>() {
                        Err(_) => return None,
                        Ok(v) => v,
                    };
                }
                3 => {
                    root = s.to_string();
                }
                4 => {
                    mount_point = s.to_string();
                }
                5 => {
                    mount_options = s.to_string();
                }
                6 => {
                    if s == "-" {
                        // skip 7
                        index += 1;
                    } else {
                        if !optional_fields.is_empty() {
                            optional_fields += " "
                        }
                        optional_fields += s;
                        // don't know if there are more optional field, try again.
                        index = 5;
                    }
                }
                7 => {
                    // "-" marks the end of the optional fields, we should have skipped 7 when
                    // (index == 6 && s == "-"), so it's impossible to match this arm.
                    return None;
                }
                8 => {
                    fstype = s.to_string();
                }
                9 => {
                    mount_source = s.to_string();
                }
                _ => {}
            }
            index += 1;
            cur += 1;
            pre = cur;
        }

        if index != 10 {
            return None;
        }
        let super_options = match std::str::from_utf8(&mbytes[pre..cur]) {
            Err(_) => return None,
            Ok(s) => s.to_string(),
        };

        self.cur = cur + 1;

        Some(MountInfo {
            mount_id,
            parent_id,
            major,
            minor,
            root,
            mount_point,
            mount_options,
            optional_fields,
            fstype,
            mount_source,
            super_options,
        })
    }
}

/// return the unit name of a mount point
pub fn mount_point_to_unit_name(mount_point: &str) -> String {
    let mut res = String::from(mount_point).replace('/', "-") + ".mount";
    if res != "-.mount" {
        res = String::from(&res[1..])
    }
    res
}

/// filter options we don't need, and return the rest
pub fn filter_options(options: &str, filter_names: Vec<&str>) -> String {
    let mut res = String::new();
    for option in options.split(',') {
        if filter_names.contains(&option.trim()) {
            continue;
        }
        if !res.is_empty() {
            res += ","
        }
        res += option
    }
    res
}

///Read the file of filename into the BufReader for later processing
pub fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

#[cfg(test)]
mod tests {
    use super::{MountInfo, MountInfoParser};

    #[test]
    fn test_mount_parser() {
        let mountinfo = "22 29 0:20 / /sys rw,nosuid,nodev,noexec,relatime shared:7 - sysfs sysfs rw
                                 91 29 8:19 / /boot/efi rw,relatime shared:46 - vfat /dev/sdb3 rw,fmask=0077,dmask=0077,codepage=437,iocharset=ascii,shortname=mixed,utf8,errors=remount-ro
                                 22 29 0:20 / /sys rw,nosuid,nodev,noexec,relatime shared:7 - sysfs sysfs".to_string();

        let mount_results = vec![
            MountInfo {
                mount_id: 22,
                parent_id: 29,
                major: 0,
                minor: 20,
                root: "/".to_string(),
                mount_point: "/sys".to_string(),
                mount_options: "rw,nosuid,nodev,noexec,relatime".to_string(),
                optional_fields: "shared:7".to_string(),
                fstype: "sysfs".to_string(),
                mount_source: "sysfs".to_string(),
                super_options: "rw".to_string(),
            },
            MountInfo {
                mount_id: 91,
                parent_id: 29,
                major: 8,
                minor: 19,
                root: "/".to_string(),
                mount_point: "/boot/efi".to_string(),
                mount_options: "rw,relatime".to_string(),
                optional_fields: "shared:46".to_string(),
                fstype: "vfat".to_string(),
                mount_source: "/dev/sdb3".to_string(),
                super_options: "rw,fmask=0077,dmask=0077,codepage=437,iocharset=ascii,shortname=mixed,utf8,errors=remount-ro".to_string(),
            }
        ];
        let mut i = 0;
        for mount in MountInfoParser::new(mountinfo) {
            assert_eq!(mount, mount_results[i]);
            i += 1;
        }
        /* The last testcase is invalid, MountParser returns None to break the loop. */
        assert_eq!(i, 1);
    }
}
