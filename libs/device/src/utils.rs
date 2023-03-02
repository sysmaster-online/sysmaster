use nix::errno::Errno;

use crate::{error::*, *};
use std::{cmp::Ordering, fmt::Debug, fs::DirEntry, path::Path};

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

    match sound_device_compare(devpath_a, devpath_b) {
        Ordering::Greater => return Ordering::Greater,
        Ordering::Less => return Ordering::Less,
        Ordering::Equal => {}
    }

    // md and dm devices are enumerated after all other devices
    match devpath_is_late_block(devpath_a).cmp(&devpath_is_late_block(devpath_b)) {
        Ordering::Greater => return Ordering::Greater,
        Ordering::Less => return Ordering::Less,
        Ordering::Equal => {}
    }

    devpath_a.cmp(devpath_b)
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
