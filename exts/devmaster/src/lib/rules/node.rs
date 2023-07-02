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

//! Tools to control static device nodes under devtmpfs.
//!
//! There may be multiple devices having symlink with the same name, but only the device with the highest
//! symlink priority can create the symlink actually. We use 'link priority' directory to record the symlink
//! priorities of different devices on symlink with the same name.
//!
//! The layout of 'priority directory' is as following:
//! /run/
//!   |- devmaster/
//!       |- link_priority/
//!           |- 'escaped symlink name 1'/
//!           |   |- <device_id_1> -> <priority_1:devnode_1>
//!           |    \ <device_id_2> -> <priority_2:devnode_2>
//!           |- 'escaped symlink name 2'/
//!                \ <device_id> -> <priority:devnode>
//!
//! Each symlink name has a corresponding 'link priority' directory under '/run/devmaster/'. If some devices
//! has the symlink, they will create a dangling symbolic linkage under the 'priority directory', which uses
//! device id as the linkage name and danglely points to <priority:devnode>. When adding or updating the symlink,
//! we will use the device with the highest priority. When there is no linkage under the 'link priority'
//! directory, the directory will be removed.

use crate::{error::*, log_dev_lock, log_dev_lock_option};
use basic::fs_util::{fchmod_and_chown, futimens_opath, symlink};
use basic::path_util::path_simplify;
use basic::{fd_util::opendirat, fs_util::remove_dir_until};
use cluFlock::ExclusiveFlock;
use device::Device;
use libc::{mode_t, S_IFBLK, S_IFCHR, S_IFLNK, S_IFMT};
use nix::dir::Dir;
use nix::fcntl::{open, readlinkat, OFlag};
use nix::sys::stat::{self, fstat, lstat, major, minor, Mode};
use nix::unistd::unlink;
use nix::unistd::{symlinkat, unlinkat, Gid, Uid, UnlinkatFlags};
use snafu::ResultExt;
use std::collections::HashMap;
use std::fs::{create_dir_all, File};
use std::os::unix::prelude::{AsRawFd, FromRawFd, RawFd};
use std::path::Path;
use std::sync::{Arc, Mutex};

pub(crate) fn node_apply_permissions(
    dev: Arc<Mutex<Device>>,
    apply_mac: bool,
    mode: Option<mode_t>,
    uid: Option<Uid>,
    gid: Option<Gid>,
    seclabel_list: &HashMap<String, String>,
) -> Result<()> {
    let devnode = dev
        .lock()
        .unwrap()
        .get_devname()
        .context(DeviceSnafu)
        .log_error("failed to apply node permissions")?;

    let file = match dev.lock().unwrap().open(OFlag::O_PATH | OFlag::O_CLOEXEC) {
        Ok(r) => r,
        Err(e) => {
            if e.is_absent() {
                return Ok(());
            }

            log::error!("failed to open device: {}", e);
            return Err(Error::Device { source: e });
        }
    };

    apply_permission_impl(
        Some(dev),
        file,
        &devnode,
        apply_mac,
        mode,
        uid,
        gid,
        seclabel_list,
    )
}

pub(crate) fn static_node_apply_permissions(
    name: String,
    mode: Option<mode_t>,
    uid: Option<Uid>,
    gid: Option<Gid>,
    _tags: &[String],
) -> Result<()> {
    if mode.is_none() && uid.is_none() && gid.is_none() && _tags.is_empty() {
        return Ok(());
    }

    let devnode = format!("/dev/{}", name);

    let file = match open(
        devnode.as_str(),
        OFlag::O_PATH | OFlag::O_CLOEXEC,
        stat::Mode::empty(),
    ) {
        Ok(fd) => unsafe { File::from_raw_fd(fd) },
        Err(e) => {
            if e == nix::errno::Errno::ENOENT {
                return Ok(());
            }
            return Err(crate::rules::Error::Nix { source: e });
        }
    };

    let stat = fstat(file.as_raw_fd()).context(NixSnafu)?;

    if (stat.st_mode & S_IFMT) != S_IFBLK && (stat.st_mode & S_IFMT) != S_IFCHR {
        log::warn!("'{}' is neither block nor character, ignoring", devnode);
        return Ok(());
    }

    /*
     * todo: export tags to a directory as symlinks
     */

    let seclabel_list = HashMap::new();

    apply_permission_impl(None, file, &devnode, false, mode, uid, gid, &seclabel_list)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_permission_impl(
    dev: Option<Arc<Mutex<Device>>>,
    file: File,
    devnode: &str,
    apply_mac: bool,
    mode: Option<mode_t>,
    uid: Option<Uid>,
    gid: Option<Gid>,
    _seclabel_list: &HashMap<String, String>,
) -> Result<()> {
    let stat = fstat(file.as_raw_fd())
        .context(NixSnafu)
        .log_dev_lock_option_error(dev.clone(), "fstat failed")?;

    /* if group is set, but mode is not set, "upgrade" group mode */
    let mode = if mode.is_none() && gid.is_some() && gid.as_ref().unwrap().as_raw() > 0 {
        Some(0o660)
    } else {
        mode
    };

    let apply_mode = mode.is_some() && (stat.st_mode & 0o777) != (mode.as_ref().unwrap() & 0o777);
    let apply_uid = uid.is_some() && stat.st_uid != uid.as_ref().unwrap().as_raw();
    let apply_gid = gid.is_some() && stat.st_gid != gid.as_ref().unwrap().as_raw();

    if apply_mode || apply_uid || apply_gid || apply_mac {
        if apply_mode || apply_uid || apply_gid {
            log_dev_lock_option!(
                debug,
                dev,
                format!(
                    "setting permission for '{}', uid='{}', gid='{}', mode='{:o}'",
                    devnode,
                    uid.as_ref()
                        .map(|i| i.as_raw())
                        .unwrap_or_else(|| stat.st_uid),
                    gid.as_ref()
                        .map(|i| i.as_raw())
                        .unwrap_or_else(|| stat.st_gid),
                    mode.as_ref().copied().unwrap_or(stat.st_mode),
                )
            );

            fchmod_and_chown(file.as_raw_fd(), devnode, mode, uid, gid)
                .context(BasicSnafu)
                .log_dev_lock_option_error(dev.clone(), "chmod and chown failed")?;
        }
    } else {
        log_dev_lock_option!(debug, dev, "preserve devnode permission");
    }

    /*
     * todo: apply SECLABEL
     */

    if let Err(e) = futimens_opath(file.as_raw_fd(), None).context(BasicSnafu) {
        log_dev_lock_option!(debug, dev, format!("failed to update timestamp: {}", e));
    }

    Ok(())
}

pub(crate) fn update_node(dev_new: Arc<Mutex<Device>>, dev_old: Arc<Mutex<Device>>) -> Result<()> {
    let old_links = dev_old.as_ref().lock().unwrap().devlinks.clone();

    for devlink in old_links.iter() {
        if dev_new.as_ref().lock().unwrap().has_devlink(devlink) {
            continue;
        }

        log_dev_lock!(
            debug,
            dev_new,
            format!("removing old devlink '{}'", devlink)
        );

        let _ = update_symlink(dev_new.clone(), devlink, false).log_dev_lock_error(
            dev_new.clone(),
            &format!("failed to remove old symlink '{}'", devlink),
        );
    }

    let new_links = dev_new.as_ref().lock().unwrap().devlinks.clone();

    for devlink in new_links.iter() {
        log_dev_lock!(
            debug,
            dev_new,
            format!("updating new devlink '{}'", devlink)
        );

        let _ = update_symlink(dev_new.clone(), devlink, true).log_dev_lock_error(
            dev_new.clone(),
            &format!("failed to add new symlink '{}'", devlink),
        );
    }

    /* create '/dev/{block, char}/$major:$minor' symlink */
    let target = device_get_symlink_by_devnum(dev_new.clone())
        .log_dev_lock_error(dev_new.clone(), "failed to get devnum symlink")?;

    if let Err(e) = node_symlink(dev_new.clone(), "", &target) {
        log_dev_lock!(debug, dev_new, e);
    }

    Ok(())
}

/// if 'add' is true, add or update the target device symlink under '/run/devmaster/links/<escaped symlink>',
/// otherwise delete the old symlink.
pub(crate) fn update_symlink(dev: Arc<Mutex<Device>>, symlink: &str, add: bool) -> Result<()> {
    /*
     * Create link priority directory if it does not exist.
     * The directory is locked until finishing updating device symlink.
     */
    let (dir, lock_file) = open_prior_dir(symlink)?;

    if let Err(e) = ExclusiveFlock::wait_lock(&lock_file) {
        log_dev_lock!(
            error,
            dev,
            format!("failed to lock priority directory for '{}': {}", symlink, e)
        );
    } else {
        /* update or remove old device symlink under '/run/devmaster/links/<escaped symlink>/' */
        update_prior_dir(dev.clone(), dir.as_raw_fd(), add)?;

        /*
         * find available devnode with the highest priority, if found, create a dangling symlink,
         * otherwise remove old symlink.
         */
        match find_prioritized_devnode(dev.clone(), dir.as_raw_fd()) {
            Err(_) => {
                format!(
                    "failed to determine device node with highest priority for {}",
                    symlink
                );
            }
            Ok(devnode) => {
                if let Some(s) = devnode {
                    return node_symlink(dev, s.as_str(), symlink);
                }
            }
        };
    }

    log_dev_lock!(debug, dev, format!("removing symlink '{}'", symlink));

    if let Err(e) = unlink(symlink).context(NixSnafu) {
        if e.get_errno() != nix::Error::ENOENT {
            log_dev_lock!(
                debug,
                dev,
                format!("failed to remove symlink '{}': {}", symlink, e)
            );
        }
    }

    /* remove empty parent directories */
    let _ = remove_dir_until(symlink, "/dev");

    Ok(())
}

/// create a link priority directory using the escaped symlink name
pub(crate) fn open_prior_dir(symlink: &str) -> Result<(Dir, File)> {
    let dirname = get_prior_dir(symlink).map_err(|e| {
        log::error!("failed to get link priority directory: {}", e);
        e
    })?;

    create_dir_all(dirname.as_str())
        .context(IoSnafu {
            filename: dirname.clone(),
        })
        .log_error(&format!("failed to create directory all '{}'", dirname))?;

    let dir = nix::dir::Dir::from_fd(
        nix::fcntl::open(
            dirname.as_str(),
            OFlag::O_CLOEXEC | OFlag::O_DIRECTORY | OFlag::O_NOFOLLOW | OFlag::O_RDONLY,
            Mode::from_bits(0o755).unwrap(),
        )
        .context(NixSnafu)
        .log_error(&format!("failed to open directory '{}'", dirname))?,
    )
    .context(NixSnafu)?;

    let lock_path = format!("{}/.lock", dirname);
    let lock_file = File::create(lock_path.as_str()).context(IoSnafu {
        filename: lock_path,
    })?;

    Ok((dir, lock_file))
}

/// get link priority path based on symlink name
pub(crate) fn get_prior_dir(symlink: &str) -> Result<String> {
    let cano_link = path_simplify(symlink);

    let name = match cano_link.strip_prefix("/dev") {
        Some(s) => s,
        None => {
            return Err(Error::Nix {
                source: nix::errno::Errno::EINVAL,
            });
        }
    };

    let escaped_link = escape_prior_dir(name);

    Ok(format!("/run/devmaster/links/{}", escaped_link))
}

pub(crate) fn escape_prior_dir(symlink: &str) -> String {
    let mut ret = String::new();

    for i in symlink.chars() {
        match i {
            '/' => ret.push_str("\\x2f"),
            '\\' => ret.push_str("\\x5c"),
            _ => ret.push(i),
        }
    }

    ret
}

/// return true if the link priority directory is updated
pub(crate) fn update_prior_dir(dev: Arc<Mutex<Device>>, dirfd: RawFd, add: bool) -> Result<bool> {
    let id = dev
        .as_ref()
        .lock()
        .unwrap()
        .get_device_id()
        .context(DeviceSnafu)
        .log_error("failed to get device id")?;

    if add {
        let devname = dev
            .as_ref()
            .lock()
            .unwrap()
            .get_devname()
            .context(DeviceSnafu)
            .log_error("failed to get devname")?;
        let priority = dev
            .as_ref()
            .lock()
            .unwrap()
            .get_devlink_priority()
            .context(DeviceSnafu)
            .log_error("failed to get devlink priority")?;
        let dangle_link = format!("{}:{}", priority, devname);
        if let Ok(pointee) = readlinkat(dirfd, id.as_str()) {
            if pointee.to_str().unwrap_or_default() == dangle_link {
                /* unchange */
                return Ok(false);
            }
        }
        if let Err(e) = unlinkat(Some(dirfd), id.as_str(), UnlinkatFlags::NoRemoveDir) {
            log::debug!("failed to unlink '{}': {}", id, e);
        }
        symlinkat(dangle_link.as_str(), Some(dirfd), id.as_str())
            .context(NixSnafu)
            .log_error("symlinkat failed")?;
    } else if let Err(e) =
        unlinkat(Some(dirfd), id.as_str(), UnlinkatFlags::NoRemoveDir).context(NixSnafu)
    {
        if e.get_errno() == nix::Error::ENOENT {
            /* unchange */
            return Ok(false);
        }
        log::error!("{}", e);
        return Err(e);
    }

    Ok(true)
}

/// read a symlink under the link priority directory and get the devnode and pirority
/// return a tuple: (priority, devnode)
fn prior_dir_read_one(dirfd: RawFd, name: &str) -> Result<(i32, String)> {
    let pointee = readlinkat(dirfd, name)
        .context(NixSnafu)
        .log_error("readlinkat failed")?;

    let pointee_str = pointee.to_str().ok_or(Error::Other {
        msg: "invalid dangling symlink".to_string(),
        errno: nix::Error::EINVAL,
    })?;

    let tokens: Vec<&str> = pointee_str.split(':').collect();

    if tokens.len() != 2 {
        return Err(Error::Other {
            msg: "invalid dangling symlink".to_string(),
            errno: nix::Error::EINVAL,
        });
    }

    if !Path::new(tokens[1]).exists() {
        return Err(Error::Nix {
            source: nix::Error::ENODEV,
        });
    }

    let priority = tokens[0].parse::<i32>().context(ParseIntSnafu)?;

    Ok((priority, tokens[1].to_string()))
}

pub(crate) fn device_get_symlink_by_devnum(dev: Arc<Mutex<Device>>) -> Result<String> {
    let subsystem = dev
        .as_ref()
        .lock()
        .unwrap()
        .get_subsystem()
        .context(DeviceSnafu)?;

    let devnum = dev
        .as_ref()
        .lock()
        .unwrap()
        .get_devnum()
        .context(DeviceSnafu)?;

    Ok(match subsystem.as_str() {
        "block" => {
            format!("/dev/block/{}:{}", major(devnum), minor(devnum))
        }
        _ => {
            format!("/dev/char/{}:{}", major(devnum), minor(devnum))
        }
    })
}

pub(crate) fn node_symlink(dev: Arc<Mutex<Device>>, devnode: &str, target: &str) -> Result<()> {
    let devnode = if devnode.is_empty() {
        dev.as_ref()
            .lock()
            .unwrap()
            .get_devname()
            .context(DeviceSnafu)
            .log_error("failed to get devname")?
    } else {
        devnode.to_string()
    };

    match lstat(target) {
        Ok(stat) => {
            if stat.st_mode & S_IFMT != S_IFLNK {
                log_dev_lock!(
                    error,
                    dev,
                    format!(
                        "conflicting inode '{}' found, symlink to '{}' will not be created",
                        target, devnode
                    )
                );
                return Err(Error::Other {
                    msg: "conflicting inode".to_string(),
                    errno: nix::Error::EINVAL,
                });
            }
        }
        Err(e) => {
            if e != nix::Error::ENOENT {
                log_dev_lock!(error, dev, format!("failed to lstat '{}'", target));
                return Err(Error::Nix { source: e });
            }
        }
    }

    if let Some(p) = Path::new(target).parent() {
        std::fs::create_dir_all(p)
            .context(IoSnafu {
                filename: target.to_string(),
            })
            .log_dev_lock_error(dev.clone(), "failed to create directory all")?;
    }

    symlink(&devnode, target, true)
        .context(BasicSnafu)
        .log_dev_lock_error(
            dev.clone(),
            &format!("failed to create symlink '{}'->'{}'", devnode, target),
        )?;

    log_dev_lock!(
        debug,
        dev,
        format!("successfully created symlink '{}' to '{}'", target, devnode)
    );
    Ok(())
}

pub(crate) fn find_prioritized_devnode(
    dev: Arc<Mutex<Device>>,
    dirfd: i32,
) -> Result<Option<String>> {
    let mut dir = opendirat(dirfd, OFlag::O_NOFOLLOW)
        .context(BasicSnafu)
        .log_error(&format!("failed to opendirat '{}'", dirfd))?;

    let mut devnode: Option<String> = None;
    let mut priority = i32::MIN;

    for e in dir.iter().flatten() {
        if let Ok(name) = e.file_name().to_str() {
            if [".", ".."].contains(&name) {
                continue;
            }

            match prior_dir_read_one(dirfd, name) {
                Ok((n, p)) => {
                    if n > priority {
                        priority = n;
                        devnode = Some(p);
                    }
                }
                Err(e) => {
                    if e.get_errno() != nix::Error::ENODEV {
                        log_dev_lock!(error, dev.clone(), format!("{}", e));
                    }
                }
            }
        }
    }

    Ok(devnode)
}

#[cfg(test)]
mod test {
    use super::*;
    use device::utils::LoopDev;
    use nix::unistd::unlink;
    use std::fs::{self, remove_dir, remove_dir_all};

    #[test]
    fn test_update_node() {
        match LoopDev::new("/tmp/test_update_node", 1024 * 1024 * 10) {
            Ok(lodev) => {
                let dev_path = lodev
                    .get_device_path()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string();

                let mut dev_new = Device::from_path(dev_path.clone()).unwrap();
                let dev_old = Device::from_path(dev_path).unwrap();

                dev_new.add_devlink("/dev/test/sss".to_string()).unwrap();

                let devnum = dev_new.get_devnum().unwrap();
                let sysname = dev_new.get_sysname().unwrap().to_string();
                let symlink = format!("/dev/block/{}:{}", major(devnum), minor(devnum));

                let link_path = Path::new(&symlink);

                if link_path.is_symlink() {
                    unlink(link_path).unwrap();
                }

                let dev_new_arc = Arc::new(Mutex::new(dev_new));
                let dev_old_arc = Arc::new(Mutex::new(dev_old));

                update_node(dev_new_arc.clone(), dev_old_arc.clone()).unwrap();

                assert!(link_path.is_symlink());
                assert_eq!(
                    fs::read_link(link_path).unwrap().as_path(),
                    Path::new(&format!("../{}", sysname))
                );

                assert!(Path::new("/dev/test/sss").is_symlink());

                update_node(dev_old_arc, dev_new_arc).unwrap();

                assert!(!Path::new("/dev/test/sss").exists());
                /* If /dev/test/ is not empty, the directory will not be removed. */
                let _ = remove_dir("/dev/test");
                unlink(symlink.as_str()).unwrap();
            }
            Err(e) => {
                assert_eq!(e.get_errno(), nix::Error::EACCES);
            }
        }
    }

    #[test]
    fn test_open_prior_dir() {
        if let Err(e) = open_prior_dir("/dev/test_symlink") {
            assert!(e.get_errno() == nix::Error::EACCES)
        }
        /* acquired exclusive lock on link priority directory */
    }

    #[test]
    fn test_update_prior_dir() {
        match LoopDev::new("/tmp/test_update_prior_dir", 1024 * 1024 * 10) {
            Ok(lodev) => {
                let mut dev = Device::from_path(
                    lodev
                        .get_device_path()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_string(),
                )
                .unwrap();

                dev.add_devlink("test/update_prior_dir".to_string())
                    .unwrap();
                let arc = Arc::new(Mutex::new(dev));

                {
                    match open_prior_dir("/dev/test/update_prior_dir") {
                        Ok((dir, _)) => {
                            /* create priority link in first time */
                            assert!(update_prior_dir(arc.clone(), dir.as_raw_fd(), true).unwrap());
                            /* priority link already exists, didn't update anything */
                            assert!(!update_prior_dir(arc.clone(), dir.as_raw_fd(), true).unwrap());
                            /* remove priority link */
                            assert!(update_prior_dir(arc, dir.as_raw_fd(), false).unwrap());
                        }
                        Err(e) => {
                            assert!(e.get_errno() == nix::Error::EACCES);
                        }
                    }
                }

                remove_dir_all("/run/devmaster/links/\\x2ftest\\x2fupdate_prior_dir/").unwrap();
            }
            Err(e) => {
                assert!(e.get_errno() == nix::Error::EACCES);
            }
        }
    }
}
