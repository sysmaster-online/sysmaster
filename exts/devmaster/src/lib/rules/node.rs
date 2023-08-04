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

use crate::{error::*, log_dev, log_dev_option};
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
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::{create_dir_all, File};
use std::os::unix::prelude::{AsRawFd, FromRawFd, RawFd};
use std::path::Path;
use std::rc::Rc;

pub(crate) fn node_apply_permissions(
    dev: Rc<RefCell<Device>>,
    apply_mac: bool,
    mode: Option<mode_t>,
    uid: Option<Uid>,
    gid: Option<Gid>,
    seclabel_list: &HashMap<String, String>,
) -> Result<()> {
    let devnode = dev
        .borrow()
        .get_devname()
        .context(DeviceSnafu)
        .log_error("failed to apply node permissions")?;

    let file = match dev.borrow().open(OFlag::O_PATH | OFlag::O_CLOEXEC) {
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
    dev: Option<Rc<RefCell<Device>>>,
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
        .log_error("fstat failed")?;

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
            log_dev_option!(
                debug,
                dev.clone(),
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
                .log_dev_error_option(dev.clone(), "chmod and chown failed")?;
        }
    } else {
        log_dev_option!(debug, dev.clone(), "preserve devnode permission");
    }

    /*
     * todo: apply SECLABEL
     */

    if let Err(e) = futimens_opath(file.as_raw_fd(), None).context(BasicSnafu) {
        log_dev_option!(debug, dev, format!("failed to update timestamp: {}", e));
    }

    Ok(())
}

pub(crate) fn update_node(
    dev_new: Rc<RefCell<Device>>,
    dev_old: Rc<RefCell<Device>>,
) -> Result<()> {
    for devlink in &dev_old.borrow().devlink_iter() {
        if dev_new.borrow().has_devlink(devlink) {
            continue;
        }

        log_dev!(
            debug,
            dev_new.borrow(),
            format!("removing old devlink '{}'", devlink)
        );

        let _ = update_symlink(dev_new.clone(), devlink, false).log_dev_error(
            &dev_new.borrow(),
            &format!("failed to remove old symlink '{}'", devlink),
        );
    }

    for devlink in &dev_new.borrow().devlink_iter() {
        log_dev!(
            debug,
            dev_new.borrow(),
            format!("updating new devlink '{}'", devlink)
        );

        let _ = update_symlink(dev_new.clone(), devlink, true).log_dev_error(
            &dev_new.borrow(),
            &format!("failed to add new symlink '{}'", devlink),
        );
    }

    /* create '/dev/{block, char}/$major:$minor' symlink */
    let target = device_get_symlink_by_devnum(dev_new.clone())
        .log_dev_error(&dev_new.borrow(), "failed to get devnum symlink")?;

    if let Err(e) = node_symlink(dev_new.clone(), "", &target) {
        log_dev!(debug, dev_new.borrow(), e);
    }

    Ok(())
}

pub(crate) fn cleanup_node(dev: Rc<RefCell<Device>>) -> Result<()> {
    for link in &dev.borrow().devlink_iter() {
        if let Err(e) = update_symlink(dev.clone(), link.as_str(), false) {
            log_dev!(
                error,
                dev.borrow(),
                format!("failed to remove symlink '{}': {}", link, e)
            );
        }
    }

    let filename = device_get_symlink_by_devnum(dev.clone())
        .log_dev_error(&dev.borrow(), "failed to get devnum symlink")?;

    match unlink(filename.as_str()) {
        Ok(_) => log_dev!(debug, dev.borrow(), format!("unlinked '{}'", filename)),
        Err(e) => {
            log_dev!(
                error,
                dev.borrow(),
                format!("failed to unlink '{}': {}", filename, e)
            );
        }
    }

    Ok(())
}

/// if 'add' is true, add or update the target device symlink under '/run/devmaster/links/<escaped symlink>',
/// otherwise delete the old symlink.
pub(crate) fn update_symlink(dev: Rc<RefCell<Device>>, symlink: &str, add: bool) -> Result<()> {
    /*
     * Create link priority directory if it does not exist.
     * The directory is locked until finishing updating device symlink.
     */
    let (dir, lock_file) = open_prior_dir(symlink)?;

    if let Err(e) = ExclusiveFlock::wait_lock(&lock_file) {
        log_dev!(
            error,
            dev.borrow(),
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

    log_dev!(
        debug,
        dev.borrow(),
        format!("removing symlink '{}'", symlink)
    );

    match unlink(symlink).context(NixSnafu) {
        Ok(_) => log_dev!(
            debug,
            dev.borrow(),
            format!("unlinked symlink '{}'", symlink)
        ),
        Err(e) => {
            if e.get_errno() != nix::Error::ENOENT {
                log_dev!(
                    error,
                    dev.borrow(),
                    format!("failed to unlink '{}': {}", symlink, e)
                );
            }
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
    let cano_link = match path_simplify(symlink) {
        None => {
            return Err(Error::Nix {
                source: nix::errno::Errno::EINVAL,
            })
        }
        Some(v) => v,
    };

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
pub(crate) fn update_prior_dir(dev: Rc<RefCell<Device>>, dirfd: RawFd, add: bool) -> Result<bool> {
    let id = dev
        .borrow()
        .get_device_id()
        .context(DeviceSnafu)
        .log_error("failed to get device id")?;

    if add {
        let devname = dev
            .borrow()
            .get_devname()
            .context(DeviceSnafu)
            .log_error("failed to get devname")?;
        let priority = dev
            .borrow()
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
        match unlinkat(Some(dirfd), id.as_str(), UnlinkatFlags::NoRemoveDir) {
            Ok(_) => log_dev!(debug, dev.borrow(), format!("unlinked '{}'", id)),
            Err(e) => log_dev!(
                error,
                dev.borrow(),
                format!("failed to unlink '{}': {}", id, e)
            ),
        }
        symlinkat(dangle_link.as_str(), Some(dirfd), id.as_str())
            .context(NixSnafu)
            .log_error("symlinkat failed")?;
    } else {
        match unlinkat(Some(dirfd), id.as_str(), UnlinkatFlags::NoRemoveDir).context(NixSnafu) {
            Ok(_) => log_dev!(debug, dev.borrow(), format!("unlinked '{}'", id)),
            Err(e) => {
                if e.get_errno() == nix::Error::ENOENT {
                    /* unchange */
                    return Ok(false);
                }
                log_dev!(
                    error,
                    dev.borrow(),
                    format!("failed to unlink '{}': {}", id, e)
                );
                return Err(e);
            }
        }
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

pub(crate) fn device_get_symlink_by_devnum(dev: Rc<RefCell<Device>>) -> Result<String> {
    let subsystem = dev.borrow().get_subsystem().context(DeviceSnafu)?;

    let devnum = dev.borrow().get_devnum().context(DeviceSnafu)?;

    Ok(match subsystem.as_str() {
        "block" => {
            format!("/dev/block/{}:{}", major(devnum), minor(devnum))
        }
        _ => {
            format!("/dev/char/{}:{}", major(devnum), minor(devnum))
        }
    })
}

pub(crate) fn node_symlink(dev: Rc<RefCell<Device>>, devnode: &str, target: &str) -> Result<()> {
    let devnode = if devnode.is_empty() {
        dev.borrow()
            .get_devname()
            .context(DeviceSnafu)
            .log_error("failed to get devname")?
    } else {
        devnode.to_string()
    };

    match lstat(target) {
        Ok(stat) => {
            if stat.st_mode & S_IFMT != S_IFLNK {
                log_dev!(
                    error,
                    dev.borrow(),
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
                log_dev!(error, dev.borrow(), format!("failed to lstat '{}'", target));
                return Err(Error::Nix { source: e });
            }
        }
    }

    if let Some(p) = Path::new(target).parent() {
        std::fs::create_dir_all(p)
            .context(IoSnafu {
                filename: target.to_string(),
            })
            .log_dev_error(&dev.borrow(), "failed to create directory all")?;
    }

    symlink(&devnode, target, true)
        .context(BasicSnafu)
        .log_dev_error(
            &dev.borrow(),
            &format!("failed to create symlink '{}'->'{}'", devnode, target),
        )?;

    log_dev!(
        debug,
        dev.borrow(),
        format!("successfully created symlink '{}' to '{}'", target, devnode)
    );
    Ok(())
}

pub(crate) fn find_prioritized_devnode(
    dev: Rc<RefCell<Device>>,
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
                        log_dev!(error, dev.borrow(), format!("{}", e));
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
    use basic::fs_util::is_symlink;
    use device::utils::LoopDev;
    use nix::unistd::unlink;
    use std::fs::{self, read_link, remove_dir, remove_dir_all};

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

                let dev_new = Device::from_path(&dev_path).unwrap();
                let dev_old = Device::from_path(&dev_path).unwrap();

                dev_new.add_devlink("/dev/test/sss").unwrap();

                let devnum = dev_new.get_devnum().unwrap();
                let sysname = dev_new.get_sysname().unwrap();
                let symlink = format!("/dev/block/{}:{}", major(devnum), minor(devnum));

                let link_path = Path::new(&symlink);

                if is_symlink(link_path) {
                    unlink(link_path).unwrap();
                }

                let dev_new_arc = Rc::new(RefCell::new(dev_new));
                let dev_old_arc = Rc::new(RefCell::new(dev_old));

                update_node(dev_new_arc.clone(), dev_old_arc.clone()).unwrap();

                assert!(is_symlink(link_path));
                assert_eq!(
                    fs::read_link(link_path).unwrap().as_path(),
                    Path::new(&format!("../{}", sysname))
                );

                assert!(is_symlink(Path::new("/dev/test/sss")));

                update_node(dev_old_arc, dev_new_arc).unwrap();

                /*
                 * If some other devices hold the same symlink, the symlink will not
                 * be removed but be linked to the other devices. Thus, if the symlink
                 * still exists, check whether it points to different devices.
                 */
                assert!(
                    !Path::new("/dev/test/sss").exists() || {
                        match read_link(Path::new("/dev/test/sss")) {
                            Ok(target) => !target.ends_with(&sysname),
                            Err(_) => false,
                        }
                    }
                );
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
                let dev =
                    Device::from_path(lodev.get_device_path().unwrap().to_str().unwrap()).unwrap();

                dev.add_devlink("test/update_prior_dir").unwrap();
                let dev_rc = Rc::new(RefCell::new(dev));

                {
                    match open_prior_dir("/dev/test/update_prior_dir") {
                        Ok((dir, _)) => {
                            /* create priority link in first time */
                            assert!(
                                update_prior_dir(dev_rc.clone(), dir.as_raw_fd(), true).unwrap()
                            );
                            /* priority link already exists, didn't update anything */
                            assert!(
                                !update_prior_dir(dev_rc.clone(), dir.as_raw_fd(), true).unwrap()
                            );
                            /* remove priority link */
                            assert!(update_prior_dir(dev_rc, dir.as_raw_fd(), false).unwrap());
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
