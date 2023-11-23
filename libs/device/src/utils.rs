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

//! utilities for device operation
use crate::{error::*, Device};
use basic::ResultExt;
use nix::errno::Errno;
use std::{cmp::Ordering, fmt::Debug, fs::DirEntry, path::Path};

#[cfg(feature = "loopdev")]
use loopdev::*;
#[cfg(feature = "loopdev")]
use std::path::PathBuf;

/// compare sound device
pub(crate) fn sound_device_compare(devpath_a: &str, devpath_b: &str) -> Ordering {
    let prefix_len = match devpath_a.find("/sound/card") {
        Some(i) => i,
        None => return Ordering::Equal,
    };

    let prefix_len = match devpath_a[prefix_len + "/sound/card".len()..].find('/') {
        Some(i) => prefix_len + i,
        None => return Ordering::Equal,
    };

    if devpath_b.len() < prefix_len || devpath_a[0..prefix_len] != devpath_b[0..prefix_len] {
        return Ordering::Equal;
    }

    let devpath_a_suffix = &devpath_a[prefix_len..];
    let devpath_b_suffix = &devpath_b[prefix_len..];

    devpath_a_suffix
        .contains("/controlC")
        .cmp(&devpath_b_suffix.contains("/controlC"))
}

/// whether the devpath is late block
pub(crate) fn devpath_is_late_block(devpath: &str) -> bool {
    devpath.contains("/block/md") || devpath.contains("/block/dm-")
}

/// compare device
pub(crate) fn device_compare(device_a: &Device, device_b: &Device) -> Ordering {
    let devpath_a = device_a.get_devpath().unwrap();
    let devpath_b = device_b.get_devpath().unwrap();

    match sound_device_compare(&devpath_a, &devpath_b) {
        Ordering::Greater => return Ordering::Greater,
        Ordering::Less => return Ordering::Less,
        Ordering::Equal => {}
    }

    // md and dm devices are enumerated after all other devices
    match devpath_is_late_block(&devpath_a).cmp(&devpath_is_late_block(&devpath_b)) {
        Ordering::Greater => return Ordering::Greater,
        Ordering::Less => return Ordering::Less,
        Ordering::Equal => {}
    }

    devpath_a.cmp(&devpath_b)
}

/// check whether directory entry is subdirectory under sysfs
pub(crate) fn relevant_sysfs_subdir(de: &DirEntry) -> bool {
    let abs_path = match de.path().canonicalize() {
        Ok(ret) => ret,
        Err(_) => {
            return false;
        }
    };
    if !abs_path.starts_with("/sys/") {
        return false;
    }

    let t = match de.file_type() {
        Ok(t) => t,
        Err(_) => return false,
    };
    if t.is_dir() || t.is_symlink() {
        return true;
    }
    false
}

/// chase the symlink and get the file name
pub(crate) fn readlink_value<P: AsRef<Path> + Debug>(path: P) -> Result<String, Error> {
    let _ = std::fs::symlink_metadata(&path).context(Io {
        msg: format!("readlink_value failed: invalid symlink {:?}", path),
    })?;

    let abs_path = match std::fs::canonicalize(path.as_ref()) {
        Ok(ret) => ret,
        Err(e) => {
            return Err(Error::Nix {
                msg: format!("readlink_value failed: canonicalize {:?} ({})", path, e),
                source: Errno::from_i32(e.raw_os_error().unwrap_or_default()),
            });
        }
    };

    Ok(abs_path.file_name().unwrap().to_str().unwrap().to_string())
}

/// loop device
#[cfg(feature = "loopdev")]
pub struct LoopDev {
    tmpfile: String,
    lodev: LoopDevice,
}

#[cfg(feature = "loopdev")]
impl LoopDev {
    /// create a temporate file with specific size
    #[allow(dead_code)]
    pub fn new(tmpfile: &str, size: u64) -> Result<Self, Error> {
        let file = std::fs::File::create(tmpfile).map_err(|e| Error::Nix {
            msg: format!("failed to create '{}'", tmpfile),
            source: e
                .raw_os_error()
                .map(nix::Error::from_i32)
                .unwrap_or(nix::Error::EIO),
        })?;
        file.set_len(size).map_err(|e| Error::Nix {
            msg: "failed to set length".to_string(),
            source: e
                .raw_os_error()
                .map(nix::Error::from_i32)
                .unwrap_or(nix::Error::EIO),
        })?;

        let lc = loopdev::LoopControl::open().map_err(|e| Error::Nix {
            msg: "failed to open lo-control".to_string(),
            source: e
                .raw_os_error()
                .map(nix::Error::from_i32)
                .unwrap_or(nix::Error::EIO),
        })?;
        let ld = lc.next_free().map_err(|e| Error::Nix {
            msg: "failed to find lo-device".to_string(),
            source: e
                .raw_os_error()
                .map(nix::Error::from_i32)
                .unwrap_or(nix::Error::EIO),
        })?;

        ld.with()
            .part_scan(true)
            .offset(0)
            .size_limit(size)
            .attach(tmpfile)
            .map_err(|e| Error::Nix {
                msg: "failed to attach lo-device".to_string(),
                source: e
                    .raw_os_error()
                    .map(nix::Error::from_i32)
                    .unwrap_or(nix::Error::EIO),
            })?;

        Ok(LoopDev {
            tmpfile: tmpfile.to_string(),
            lodev: ld,
        })
    }

    /// get the loop device path
    #[allow(dead_code)]
    pub fn get_device_path(&self) -> Option<PathBuf> {
        self.lodev.path()
    }

    /// create a loop device based on the temporary file and call the function to
    /// deal with the loop device
    pub fn inner_process<F>(tmpfile: &str, size: u64, f: F) -> Result<(), Error>
    where
        F: FnOnce(&mut Device) -> Result<(), Error>,
    {
        let lo = Self::new(tmpfile, size)?;

        let devpath = lo.get_device_path().ok_or(Error::Nix {
            msg: "invalid loop device path".to_string(),
            source: nix::Error::EINVAL,
        })?;

        let mut dev = Device::from_path(devpath.to_str().ok_or(Error::Nix {
            msg: "can't change path buffer to string".to_string(),
            source: nix::Error::EINVAL,
        })?)?;

        dev.set_base_path("/tmp/devmaster");

        f(&mut dev)
    }
}

#[cfg(feature = "loopdev")]
impl Drop for LoopDev {
    fn drop(&mut self) {
        let _ = self.lodev.detach();
        let _ = std::fs::remove_file(self.tmpfile.as_str());
    }
}
