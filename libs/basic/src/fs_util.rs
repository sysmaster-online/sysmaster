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
    fcntl::{open, readlink, renameat, OFlag},
    sys::stat::{fstat, Mode},
    unistd::{unlinkat, Gid, Uid, UnlinkatFlags},
};
use pathdiff::diff_paths;
use rand::Rng;
use std::{
    ffi::CString,
    fs::{create_dir_all, remove_dir, File},
    io::ErrorKind,
    os::unix::prelude::{AsRawFd, FromRawFd, PermissionsExt, RawFd},
    path::Path,
};

/// open the parent directory of path
pub fn open_parent(path: &Path, flags: OFlag, mode: Mode) -> Result<File> {
    let parent = path.parent().ok_or(Error::Nix {
        source: nix::errno::Errno::EINVAL,
    })?;

    Ok(unsafe { File::from_raw_fd(nix::fcntl::open(parent, flags, mode).context(NixSnafu)?) })
}

/// create symlink link -> target
/* Please don't use "from/to", use "symlink/target" to name path.
 * Take "A -> B" for example, A is "link", B is "target". */
pub fn symlink(target: &str, link: &str, relative: bool) -> Result<()> {
    let link_path = Path::new(&link);
    let target_path = Path::new(&target);

    let (target_path, dir) = if relative {
        let link_path_parent = link_path.parent().ok_or(Error::NotExisted {
            what: format!("{}'s parent", link_path.to_string_lossy()),
        })?;

        let relative_path = diff_paths(target_path, link_path_parent).unwrap();
        let fd = nix::fcntl::open(
            link_path_parent,
            OFlag::O_DIRECTORY | OFlag::O_CLOEXEC | OFlag::O_NOFOLLOW,
            Mode::from_bits(0).unwrap(),
        )
        .context(NixSnafu)?;
        (relative_path, Some(unsafe { File::from_raw_fd(fd) }))
    } else {
        (target_path.to_path_buf(), None)
    };

    let mut rng = rand::thread_rng();
    let tmp_link = format!("{}.{}", link, rng.gen::<u32>());
    let raw_fd = dir.map(|f| f.as_raw_fd());

    if let Err(e) = nix::unistd::symlinkat(target_path.as_path(), raw_fd, tmp_link.as_str()) {
        log::error!("Failed to create symlink {link} -> {target}: {e}");
        return Err(Error::Nix { source: e });
    }

    if let Err(e) = renameat(raw_fd, tmp_link.as_str(), raw_fd, link_path) {
        log::error!("Failed to rename the temporary path of {link_path:?}: {e}");
        let _ = unlinkat(raw_fd, tmp_link.as_str(), UnlinkatFlags::NoRemoveDir);
        return Err(Error::Nix { source: e });
    }

    log::debug!("Successfully created symlink: {link} -> {target}");

    Ok(())
}

/// chmod based on fd opened with O_PATH
pub fn fchmod_opath(fd: RawFd, mode: mode_t) -> Result<()> {
    let fd_path = format_proc_fd_path!(fd);

    let mut perms = std::fs::metadata(&fd_path).context(IoSnafu)?.permissions();
    perms.set_mode(mode);
    std::fs::set_permissions(&fd_path, perms).context(IoSnafu)
}

/// chmod based on path
pub fn chmod(path: &str, mode: mode_t) -> Result<()> {
    let mut perms = std::fs::metadata(path).context(IoSnafu)?.permissions();
    perms.set_mode(mode);
    std::fs::set_permissions(path, perms).context(IoSnafu)
}

/// Safely chmod and chown based on a file description. If ownership
/// and access mode are both changed, ensuring there is no point when
/// the access mode is above the old mode under old owner or the new
/// mode under new owner.
pub fn fchmod_and_chown(
    fd: RawFd,
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

/// Update the timestamp of a file with fd. If 'ts' is not provided, use the current timestamp by default.
pub fn futimens_opath(fd: RawFd, ts: Option<[timespec; 2]>) -> Result<()> {
    let fd_path = format_proc_fd_path!(fd);
    let c_string =
        CString::new(fd_path.clone()).map_err(|e| crate::Error::NulError { source: e })?;
    let times = ts.unwrap_or([
        timespec {
            tv_sec: 0,
            tv_nsec: libc::UTIME_NOW,
        },
        timespec {
            tv_sec: 0,
            tv_nsec: libc::UTIME_NOW,
        },
    ])[0];

    if unsafe { libc::utimensat(libc::AT_FDCWD, c_string.as_ptr(), &times, 0) } < 0 {
        let errno = nix::Error::from_i32(
            std::io::Error::last_os_error()
                .raw_os_error()
                .unwrap_or_default(),
        );

        if errno == nix::Error::ENOENT {
            /*
             * In devmaster threads, utimensat will fail with errno of ENOENT, which means
             * the file path does not exist, if the fd path is used. The fd path is a symlink to
             * the opened real file. It is weird because the fd path really exists but utimesat
             * can not find it. To avoid the failure, we try to follow the fd path symlink to the
             * real file and update the timestamp directly on it.
             */
            let target = readlink(fd_path.as_str()).unwrap();
            let c_string = target
                .to_str()
                .ok_or(Error::Nix { source: errno })
                .map(|s| unsafe { libc::strdup(s.as_ptr() as *const libc::c_char) })?;

            if unsafe { libc::utimensat(libc::AT_FDCWD, c_string, &times, 0) } < 0 {
                return Err(Error::Nix {
                    source: nix::Error::from_i32(
                        std::io::Error::last_os_error().raw_os_error().unwrap(),
                    ),
                });
            }
        } else {
            return Err(Error::Nix { source: errno });
        }
    }

    Ok(())
}

/// recursively remove parent directories until specific directory
pub fn remove_dir_until(path: &str, stop: &str) -> Result<()> {
    let path = Path::new(path);

    let mut dir = if path.is_dir() {
        Path::new(path)
    } else {
        match path.parent() {
            Some(p) => p,
            None => {
                return Ok(());
            }
        }
    };

    loop {
        if let Err(e) = remove_dir(dir) {
            match e.kind() {
                ErrorKind::NotFound => break,
                _ => {
                    return Err(Error::Io { source: e });
                }
            }
        }

        match dir.parent() {
            Some(p) => {
                if p.ends_with(stop) {
                    break;
                }
                dir = p;
            }
            None => break,
        };
    }

    Ok(())
}

/// create and open a temporary file
pub fn open_temporary(path: &str) -> Result<(File, String)> {
    let tmp_file = format!("{}.{}", path, rand::thread_rng().gen::<u32>());

    let f = open(
        tmp_file.as_str(),
        OFlag::O_CLOEXEC | OFlag::O_NOCTTY | OFlag::O_RDWR | OFlag::O_CREAT | OFlag::O_EXCL,
        Mode::from_bits(0o644).unwrap(),
    )
    .context(NixSnafu)?;

    let file = unsafe { File::from_raw_fd(f) };

    Ok((file, tmp_file))
}

/// Create a new file based on absolute path.
pub fn touch_file(
    path: &str,
    parents: bool,
    mode: Option<mode_t>,
    uid: Option<Uid>,
    gid: Option<Gid>,
) -> Result<bool> {
    let p = Path::new(path);

    if parents && p.parent().is_some() {
        let _ = create_dir_all(p.parent().unwrap());
    }

    let fd = match open(
        p,
        OFlag::O_PATH | OFlag::O_CLOEXEC | OFlag::O_NOFOLLOW,
        Mode::empty(),
    ) {
        Ok(n) => n,
        Err(e) => {
            if e != nix::Error::ENOENT {
                return Err(Error::Nix { source: e });
            }

            open(
                p,
                OFlag::O_WRONLY | OFlag::O_CREAT | OFlag::O_EXCL | OFlag::O_CLOEXEC,
                mode.map(|v| Mode::from_bits(v).unwrap_or(Mode::empty()))
                    .unwrap_or(Mode::empty()),
            )
            .context(NixSnafu)?
        }
    };

    /* The returned value should be explicitly declared, otherwise the file will be closed at once */
    let _f = unsafe { File::from_raw_fd(fd) };

    fchmod_and_chown(fd, path, mode, uid, gid)
}

/// do some operations and log message out, returns Io error when fail
#[macro_export]
macro_rules! do_entry_or_return_io_error {
    ($function:expr, $entry:ident, $action:literal) => {
        match $function(&$entry) {
            Err(e) => {
                log::error!("Failed to {} {:?}: {e}", $action, $entry);
                return Err(e).context(IoSnafu);
            }
            Ok(_) => log::debug!("{} {:?} succeeded", $action, $entry),
        }
    };
}

/// do some operations and log message out, skip the error
#[macro_export]
macro_rules! do_entry_log {
    ($function:expr, $entry:ident, $action:literal) => {
        match $function(&$entry) {
            Err(e) => log::error!("Failed to {} {:?}: {e}", $action, $entry),
            Ok(_) => log::debug!("{} {:?} succeeded", $action, $entry),
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs_util::symlink;
    use nix::unistd::{self, unlink};
    use std::{
        fs::{create_dir_all, remove_file, File},
        os::unix::prelude::MetadataExt,
        time::SystemTime,
    };

    #[test]
    fn test_symlink() {
        // use a complicated long name to make sure we don't have this file
        // before running this testcase.
        let link_name_path = std::path::Path::new("/tmp/test_link_name_39285b");
        if link_name_path.exists() {
            return;
        }

        symlink("/dev/null", "/tmp/test_link_name_39285b", false).unwrap();

        unistd::unlinkat(
            None,
            link_name_path.to_str().unwrap(),
            unistd::UnlinkatFlags::NoRemoveDir,
        )
        .unwrap();

        symlink("/dev/null", "/tmp/test_link_name_39285c", true).unwrap();

        unistd::unlinkat(
            None,
            "/tmp/test_link_name_39285c",
            unistd::UnlinkatFlags::NoRemoveDir,
        )
        .unwrap();
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

    #[test]
    fn test_remove_dir_until() {
        create_dir_all("/tmp/test_remove_dir_until_1/test1").unwrap();
        assert!(Path::new("/tmp/test_remove_dir_until_1/test1").exists());
        remove_dir_until("/tmp/test_remove_dir_until_1/test1", "/tmp").unwrap();
        assert!(!Path::new("/tmp/test_remove_dir_until_1/test1").exists());
        assert!(!Path::new("/tmp/test_remove_dir_until_1").exists());

        create_dir_all("/tmp/test_remove_dir_until_2/test2").unwrap();
        File::create("/tmp/test_remove_dir_until_2/test_file").unwrap();
        assert!(Path::new("/tmp/test_remove_dir_until_2/test2").exists());
        assert!(Path::new("/tmp/test_remove_dir_until_2/test_file").exists());
        assert!(remove_dir_until("/tmp/test_remove_dir_until_2/test2", "/tmp").is_err());
        assert!(!Path::new("/tmp/test_remove_dir_until_2/test2").exists());
        assert!(Path::new("/tmp/test_remove_dir_until_2/test_file").exists());
    }

    #[test]
    fn test_open_temporary() {
        let (file, name) = open_temporary("test_open_temporary").unwrap();
        let p = Path::new(&name);
        println!("{:?}", p.canonicalize().unwrap());
        drop(file);
        remove_file(name).unwrap();
    }

    #[test]
    fn test_touch_file() {
        let _ = unlink("/tmp/test_touch_file/f1");
        touch_file("/tmp/test_touch_file/f1", true, Some(0o444), None, None).unwrap();
        let p = Path::new("/tmp/test_touch_file/f1");
        assert!(p.exists());
        let md = p.metadata().unwrap();
        assert_eq!(md.mode() & 0o777, 0o444);
        touch_file("/tmp/test_touch_file/f1", true, Some(0o666), None, None).unwrap();
        let md = p.metadata().unwrap();
        assert_eq!(md.mode() & 0o777, 0o666);
        let _ = unlink("/tmp/test_touch_file/f1");
    }
}
