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

//! the utils of the file operation
//!
use crate::{error::*, format_proc_fd_path};
use libc::{fchownat, mode_t, timespec, AT_EMPTY_PATH, S_IFLNK, S_IFMT};
use nix::{
    fcntl::OFlag,
    sys::stat::{fstat, Mode},
    unistd::{Gid, Uid},
};
use pathdiff::diff_paths;
use std::{os::unix::prelude::PermissionsExt, path::Path};

/// open the parent directory of path
pub fn open_parent(path: &Path, flags: OFlag, mode: Mode) -> Result<i32> {
    let parent = path.parent().ok_or(Error::Nix {
        source: nix::errno::Errno::EINVAL,
    })?;

    nix::fcntl::open(parent, flags, mode).context(NixSnafu)
}

/// create symlink link_name -> target
pub fn symlink(target: &str, link: &str, relative: bool) -> Result<()> {
    let link_path = Path::new(&link);
    let target_path = Path::new(&target);

    let (target_path, fd) = if relative {
        let link_path_parent = link_path.parent().ok_or(Error::NotExisted {
            what: format!("{}'s parent", link_path.to_string_lossy()),
        })?;

        let rel_path = diff_paths(target_path, link_path_parent).unwrap();
        let fd = nix::fcntl::open(&rel_path, OFlag::O_DIRECT, Mode::from_bits(0).unwrap())
            .context(NixSnafu)?;
        (rel_path, Some(fd))
    } else {
        (target_path.to_path_buf(), None)
    };

    nix::unistd::symlinkat(target_path.as_path(), fd, link_path).map_err(|e| {
        log::debug!("Failed to create symlink: {} -> {}", link, target);
        Error::Nix { source: e }
    })
}

/// chmod based on fd opened with O_PATH
pub fn fchmod_opath(fd: i32, mode: mode_t) -> Result<()> {
    let fd_path = format_proc_fd_path!(fd);

    let mut perms = std::fs::metadata(&fd_path).context(IoSnafu)?.permissions();
    perms.set_mode(mode);
    std::fs::set_permissions(&fd_path, perms).context(IoSnafu)?;

    Ok(())
}

/// chmod based on path
pub fn chmod(path: &str, mode: mode_t) -> Result<()> {
    let mut perms = std::fs::metadata(path).context(IoSnafu)?.permissions();
    perms.set_mode(mode);
    std::fs::set_permissions(path, perms).context(IoSnafu)?;

    Ok(())
}

/// Safely chmod and chown based on a file description. If ownership
/// and access mode are both changed, ensuring there is no point when
/// the access mode is above the old mode under old owner or the new
/// mode under new owner.
pub fn fchmod_and_chown(
    fd: i32,
    path: &str,
    mode: Option<mode_t>,
    uid: Option<Uid>,
    gid: Option<Gid>,
) -> Result<bool> {
    let st = fstat(fd).context(NixSnafu)?;

    let do_chown = (uid.is_some() && st.st_uid != uid.as_ref().unwrap().as_raw())
        || (gid.is_some() && st.st_gid != gid.as_ref().unwrap().as_raw());
    let do_chmod = ((st.st_mode & S_IFMT) != S_IFLNK)
        && (mode.is_some() && ((st.st_mode ^ mode.as_ref().unwrap()) & 0o7777 != 0))
        || do_chown;

    if do_chmod
        && (mode.as_ref().unwrap() & S_IFMT > 0)
        && ((mode.as_ref().unwrap() ^ st.st_mode) & S_IFMT > 0)
    {
        return Err(Error::Invalid {
            what: "file types are inconsistent".to_string(),
        });
    }

    if do_chown & do_chmod {
        let intersection = st.st_mode & mode.as_ref().unwrap();

        if (intersection ^ st.st_mode) & 0o7777 != 0
            && fchmod_opath(fd, intersection & 0o7777).is_err()
        {
            chmod(path, intersection & 0o7777)?;
        }
    }

    if do_chown {
        let r = unsafe {
            fchownat(
                fd,
                "\0".as_ptr() as *const libc::c_char,
                uid.as_ref().map_or_else(|| u32::MAX, |v| v.as_raw()),
                gid.as_ref().map_or_else(|| u32::MAX, |v| v.as_raw()),
                AT_EMPTY_PATH,
            )
        };
        if r < 0 {
            return Err(Error::Nix {
                source: nix::Error::from_i32(
                    std::io::Error::last_os_error().raw_os_error().unwrap(),
                ),
            });
        }
    }

    if do_chmod && fchmod_opath(fd, mode.as_ref().unwrap() & 0o7777).is_err() {
        chmod(path, mode.as_ref().unwrap() & 0o7777)?;
    }

    Ok(do_chown || do_chmod)
}

/// if ts are not provided, use the current timestamp by default.
pub fn futimens_opath(fd: i32, ts: Option<[timespec; 2]>) -> Result<()> {
    let r = unsafe {
        libc::utimensat(
            libc::AT_FDCWD,
            format_proc_fd_path!(fd).as_ptr() as *const libc::c_char,
            &ts.unwrap_or([
                timespec {
                    tv_sec: 0,
                    tv_nsec: libc::UTIME_NOW,
                },
                timespec {
                    tv_sec: 0,
                    tv_nsec: libc::UTIME_NOW,
                },
            ])[0],
            0,
        )
    };
    if r < 0 {
        Err(Error::Nix {
            source: nix::Error::from_i32(std::io::Error::last_os_error().raw_os_error().unwrap()),
        })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs_util::symlink;
    use nix::unistd::{self};
    use std::{fs::File, os::unix::prelude::MetadataExt, time::SystemTime};

    #[test]
    fn test_symlink() {
        // use a complicated long name to make sure we don't have this file
        // before running this testcase.
        let link_name_path = std::path::Path::new("/tmp/test_link_name_39285b");
        if link_name_path.exists() {
            return;
        }

        let ret = symlink("/dev/null", "/tmp/test_link_name_39285b", false);
        assert!(ret.is_ok());

        let ret = unistd::unlinkat(
            None,
            link_name_path.to_str().unwrap(),
            unistd::UnlinkatFlags::NoRemoveDir,
        );
        assert!(ret.is_ok());
    }

    /// test changing the mode of a file by file descriptor with O_PATH
    #[test]
    fn test_fchmod_opath() {
        let file = File::create("/tmp/test_fchmod_opath").unwrap();
        let fd = nix::fcntl::open("/tmp/test_fchmod_opath", OFlag::O_PATH, Mode::empty()).unwrap();

        fchmod_opath(fd, 0o444).unwrap();

        assert_eq!(file.metadata().unwrap().mode() & 0o777, 0o444);

        std::fs::remove_file("/tmp/test_fchmod_opath").unwrap();
    }

    /// test changing the mode of a file by file path
    #[test]
    fn test_chmod() {
        let file = File::create("/tmp/test_chmod").unwrap();

        chmod("/tmp/test_chmod", 0o444).unwrap();

        assert_eq!(file.metadata().unwrap().mode() & 0o777, 0o444);

        std::fs::remove_file("/tmp/test_chmod").unwrap();
    }

    /// test changing the mode or owner of a file by file descriptor with O_PATH
    #[test]
    fn test_fchmod_and_chown() {
        let _ = File::create("/tmp/test_fchmod_and_chown").unwrap();

        let fd =
            nix::fcntl::open("/tmp/test_fchmod_and_chown", OFlag::O_PATH, Mode::empty()).unwrap();

        fchmod_and_chown(fd, "", Some(0o664), None, None).unwrap();
        let stat = fstat(fd).unwrap();
        assert_eq!(stat.st_mode & 0o777, 0o664);

        std::fs::remove_file("/tmp/test_fchmod_and_chown").unwrap();
    }

    /// test updating the timestamp of a file by file descriptor with O_PATH
    #[test]
    fn test_futimens_opath() {
        let _ = File::create("/tmp/test_futimens_opath").unwrap();

        let fd =
            nix::fcntl::open("/tmp/test_futimens_opath", OFlag::O_PATH, Mode::empty()).unwrap();

        // check point
        let point = SystemTime::now();

        futimens_opath(fd, None).unwrap();

        let metadata = File::open(format_proc_fd_path!(fd))
            .expect("failed to open file")
            .metadata()
            .expect("failed to get metadata");

        let access = metadata.accessed().unwrap();
        let modify = metadata.modified().unwrap();

        // considering time cost over instructions, allow slender timestamp gap
        // but not greater than 10ms
        assert!(point.duration_since(access).unwrap().as_nanos() < 10000000);
        assert!(point.duration_since(modify).unwrap().as_nanos() < 10000000);

        std::fs::remove_file("/tmp/test_futimens_opath").unwrap();
    }
}
