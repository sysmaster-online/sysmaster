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

//! the utils of the path operation
use crate::Error;
use crate::{error::*, format_proc_fd_path};
use libc::{fchownat, mode_t, timespec, AT_EMPTY_PATH, S_IFLNK, S_IFMT};
use nix::unistd::mkdir;
use nix::{
    fcntl::{open, readlink, renameat, OFlag},
    sys::stat::{fstat, Mode},
    sys::statfs,
    unistd::{unlinkat, Gid, Uid, UnlinkatFlags},
};
use pathdiff::diff_paths;
use rand::Rng;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::{
    ffi::CString,
    fs::{create_dir_all, remove_dir, File},
    io::ErrorKind,
    os::unix::prelude::{AsRawFd, FromRawFd, PermissionsExt, RawFd},
};

const CHASE_SYMLINK_MAX: i32 = 32;

/// read first line from a file
pub fn read_first_line(path: &Path) -> Result<String> {
    let file = std::fs::File::open(path).context(IoSnafu)?;
    let mut buffer = BufReader::new(file);
    let mut first_line = String::with_capacity(1024);
    let _ = buffer.read_line(&mut first_line);
    Ok(first_line)
}

/// write string to file
pub fn write_string_file<P: AsRef<Path>>(path: P, value: String) -> std::io::Result<()> {
    let mut file = OpenOptions::new().write(true).open(&path)?;

    let _ = file.write(value.as_bytes())?;

    Ok(())
}

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
        log::error!("Failed to create symlink {} -> {}: {}", link, target, e);
        return Err(Error::Nix { source: e });
    }

    if let Err(e) = renameat(raw_fd, tmp_link.as_str(), raw_fd, link_path) {
        log::error!(
            "Failed to rename the temporary path of {:?}: {}",
            link_path,
            e
        );
        let _ = unlinkat(raw_fd, tmp_link.as_str(), UnlinkatFlags::NoRemoveDir);
        return Err(Error::Nix { source: e });
    }

    log::debug!("Successfully created symlink: {} -> {}", link, target);

    Ok(())
}

/// chase the given symlink, and return the final target.
pub fn chase_symlink(link_path: &Path) -> Result<PathBuf> {
    let mut current_path = PathBuf::from(link_path);
    let mut max_follows = CHASE_SYMLINK_MAX;
    loop {
        let mut current_dir = match current_path.parent() {
            None => {
                return Err(Error::NotExisted {
                    what: "couldn't determine parent directory".to_string(),
                })
            }
            Some(v) => v.to_string_lossy().to_string(),
        };

        /* empty current_dir joined with "/target_path" will generate root directory mistakenly. */
        if current_dir.is_empty() {
            current_dir = ".".to_string();
        }

        let mut target_path = match std::fs::read_link(&current_path) {
            Err(e) => return Err(Error::Io { source: e }),
            Ok(v) => v,
        };

        if target_path.is_relative() {
            let current_path_str = current_dir + "/" + &target_path.to_string_lossy().to_string();
            let simplified_path = match path_simplify(&current_path_str) {
                None => {
                    return Err(Error::Invalid {
                        what: format!("invalid file path: {}", current_path_str),
                    })
                }
                Some(v) => v,
            };
            target_path = match PathBuf::from_str(&simplified_path) {
                Err(_) => {
                    return Err(Error::Invalid {
                        what: format!("invalid file path: {}", current_path_str),
                    })
                }
                Ok(v) => v,
            };
        }

        if !target_path.exists() {
            return Err(Error::Nix {
                source: nix::errno::Errno::ENOENT,
            });
        }

        if !is_symlink(&target_path) {
            return Ok(target_path);
        }

        max_follows -= 1;
        if max_follows <= 0 {
            break;
        }
        current_path = target_path;
    }
    Err(Error::Nix {
        source: nix::errno::Errno::ELOOP,
    })
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
             *
             * By the way, this code region is not covered in unit test cases. This region is expected
             * to run in devmaster threads.
             */
            let target = readlink(fd_path.as_str()).context(NixSnafu)?;
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

/// mkdir -p with the given directory mode
pub fn mkdir_p_label(path: &Path, mode: u32) -> Result<()> {
    let path_str = path.to_string_lossy();
    if path.exists() && path.is_dir() {
        if let Err(e) = chmod(&path_str, mode) {
            log::error!("Failed to chmod of {}: {}", path_str, e);
            return Err(e);
        }
    }

    let simplified_path = match path_simplify(&path_str) {
        None => {
            return Err(Error::Invalid {
                what: format!("Invalid path: {}", path_str),
            });
        }
        Some(v) => v,
    };

    if !path_is_abosolute(&simplified_path) {
        return Err(Error::Invalid {
            what: format!(
                "Invalid Path: {}, only abosolute path is allowed.",
                path_str
            ),
        });
    }

    let mode = Mode::from_bits_truncate(mode);
    let mut cur_path = PathBuf::from("/");
    // mkdir -p up to down
    for e in simplified_path.split('/') {
        cur_path = cur_path.join(e);
        if cur_path.exists() {
            continue;
        }
        if let Err(e) = mkdir(&cur_path, mode) {
            return Err(Error::Nix { source: e });
        }
    }
    Ok(())
}

/// mkdir parents with the given label
pub fn mkdir_parents_label() {}

/// check if the given directory is not empty
pub fn directory_is_not_empty(path: &Path) -> Result<bool> {
    if path.is_file() {
        return Ok(false);
    }
    let mut iter = match path.read_dir() {
        Err(err) => {
            return Err(Error::Nix {
                source: nix::Error::from_i32(err.raw_os_error().unwrap_or_default()),
            })
        }
        Ok(v) => v,
    };
    Ok(iter.next().is_some())
}

/// check if the given directory is empty
pub fn directory_is_empty(path: &Path) -> bool {
    if path.is_file() {
        return false;
    }
    let mut iter = match path.read_dir() {
        Err(_) => return false,
        Ok(v) => v,
    };
    iter.next().is_none()
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

            let empty = Mode::empty();
            open(
                p,
                OFlag::O_WRONLY | OFlag::O_CREAT | OFlag::O_EXCL | OFlag::O_CLOEXEC,
                mode.map(|v| Mode::from_bits(v).unwrap_or(empty))
                    .unwrap_or(empty),
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
                log::error!("Failed to {} {:?}: {}", $action, $entry, e);
                return Err(e).context(IoSnafu);
            }
            Ok(_) => log::debug!("{} {:?} succeeded", $action, $entry),
        }
    };
}

/// Replace unstable is_symlink method in std
pub fn is_symlink(path: &Path) -> bool {
    let md = match path.symlink_metadata() {
        Ok(md) => md,
        Err(_) => return false,
    };

    md.file_type().is_symlink()
}

/// do some operations and log message out, skip the error
#[macro_export]
macro_rules! do_entry_log {
    ($function:expr, $entry:ident, $action:literal) => {
        match $function(&$entry) {
            Err(e) => log::error!("Failed to {} {:?}: {}", $action, $entry, e),
            Ok(_) => log::debug!("{} {:?} succeeded", $action, $entry),
        }
    };
}

/// The maximum length of a linux path
pub const PATH_LENGTH_MAX: usize = 4096;

/// The maximum length of a linux file name
pub const FILE_LENGTH_MAX: usize = 255;

/// return true if the path of a and b equaled.
pub fn path_equal(a: &str, b: &str) -> bool {
    let p_a = Path::new(a);
    let p_b = Path::new(b);
    p_a == p_b
}

/// check if the path name contains unsafe character
///
/// return true if it doesn't contain unsafe character
pub fn path_name_is_safe(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    for c in s.chars() {
        if c > 0 as char && c < ' ' {
            return false;
        }
        if c.is_ascii_control() {
            return false;
        }
    }
    true
}

/// check if the path length is valid
pub fn path_length_is_valid(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    if s.len() > PATH_LENGTH_MAX {
        return false;
    }
    let mut de_len = 0;
    let mut last_c = '/';
    for c in s.chars() {
        match c {
            '/' => {
                de_len = 0;
            }
            '.' => {
                if last_c == '/' {
                    de_len = 1;
                } else {
                    de_len += 1;
                }
            }
            _ => {
                de_len += 1;
            }
        }
        if de_len > FILE_LENGTH_MAX {
            return false;
        }
        last_c = c;
    }
    true
}

/// Remove redundant inner and trailing slashes and unnecessary dots to simplify path.
/// e.g., //foo//.//bar/ becomes /foo/bar
/// /foo/foo1/../bar becomes /foo/bar
pub fn path_simplify(p: &str) -> Option<String> {
    let mut res = String::new();
    let mut stack: Vec<&str> = Vec::new();
    for f in p.split('/') {
        if f.is_empty() || f == "." {
            continue;
        }
        if f == ".." {
            if let Some(v) = stack.last() {
                if *v != ".." {
                    stack.pop();
                    continue;
                }
            }
            if !p.starts_with('/') {
                stack.push(f);
                continue;
            }
            return None;
        }
        stack.push(f);
    }

    if stack.is_empty() {
        if p.starts_with('/') {
            return Some("/".to_string());
        } else {
            return Some(".".to_string());
        }
    }

    if p.starts_with('/') {
        res += "/";
    }
    res += stack.remove(0);

    for f in stack {
        res += "/";
        res += f;
    }

    Some(res)
}

/// check if the given string is a path
pub fn is_path(s: &str) -> bool {
    s.contains('/')
}

/// check if the given path is abololute path
pub fn path_is_abosolute(s: &str) -> bool {
    s.starts_with('/')
}

/// check if the absolute path is valid, return the simplified path String if it's valid.
pub fn parse_absolute_path(s: &str) -> Result<String, Error> {
    if !path_name_is_safe(s) {
        return Err(Error::Invalid {
            what: "path contains unsafe character".to_string(),
        });
    }

    if !path_length_is_valid(s) {
        return Err(Error::Invalid {
            what: "path is too long or empty".to_string(),
        });
    }

    if !path_is_abosolute(s) {
        return Err(Error::Invalid {
            what: "path is not abosolute".to_string(),
        });
    }

    let path = match path_simplify(s) {
        None => {
            return Err(Error::Invalid {
                what: "path can't be simplified".to_string(),
            });
        }
        Some(v) => v,
    };

    Ok(path)
}

///
pub fn parse_pathbuf(s: &str) -> Result<PathBuf, Error> {
    let path = parse_absolute_path(s).map_err(|_| Error::Invalid {
        what: "Invalid PathBuf".to_string(),
    })?;
    Ok(PathBuf::from(path))
}

/// unit transient path in /run
pub const RUN_TRANSIENT_PATH: &str = "/run/sysmaster/transient";
/// unit lookup path in /etc
pub const ETC_SYSTEM_PATH: &str = "/etc/sysmaster/system";
/// unit lookup path in /run
pub const RUN_SYSTEM_PATH: &str = "/run/sysmaster/system";
/// unit lookup path in /usr/lib
pub const LIB_SYSTEM_PATH: &str = "/usr/lib/sysmaster/system";

/// struct LookupPaths
#[derive(Debug, Clone)]
pub struct LookupPaths {
    /// Used to search fragment, dropin, updated
    pub search_path: Vec<String>,
    /// Used to search preset file
    pub preset_path: Vec<String>,
    /// generator paths
    pub generator: String,
    /// generator early paths
    pub generator_early: String,
    /// generator late paths
    pub generator_late: String,
    /// transient paths
    pub transient: String,
    /// transient paths
    pub persistent_path: String,
}

impl LookupPaths {
    /// new
    pub fn new() -> Self {
        LookupPaths {
            generator: String::from(""),
            generator_early: String::from(""),
            generator_late: String::from(""),
            transient: String::from(""),
            search_path: Vec::new(),
            persistent_path: String::from(""),
            preset_path: Vec::new(),
        }
    }

    /// init lookup paths
    pub fn init_lookup_paths(&mut self) {
        self.search_path.push(RUN_TRANSIENT_PATH.to_string());
        self.search_path.push(ETC_SYSTEM_PATH.to_string());
        self.search_path.push(RUN_SYSTEM_PATH.to_string());
        self.search_path.push(LIB_SYSTEM_PATH.to_string());

        self.preset_path
            .push(format!("{}/{}", ETC_SYSTEM_PATH, "system-preset"));
        self.preset_path
            .push(format!("{}/{}", LIB_SYSTEM_PATH, "system-preset"));

        self.persistent_path = ETC_SYSTEM_PATH.to_string();
    }
}

impl Default for LookupPaths {
    fn default() -> Self {
        Self::new()
    }
}

/// Check whether $p belongs to $fstype fs
pub fn check_filesystem(p: &Path, fstype: statfs::FsType) -> bool {
    let fstp = match statfs::statfs(p) {
        Ok(s) => s.filesystem_type(),
        Err(_) => {
            return false;
        }
    };
    fstp == fstype
}

#[cfg(test)]
mod tests {
    use super::LookupPaths;
    use super::*;
    use nix::unistd::{self, unlink};
    use std::{
        fs::{create_dir_all, remove_dir_all, remove_file, File},
        io::BufWriter,
        os::unix::prelude::MetadataExt,
        time::SystemTime,
    };
    use tempfile::NamedTempFile;

    #[test]
    fn test_read_first_line() {
        let file = NamedTempFile::new().unwrap();
        let mut buffer = BufWriter::new(&file);
        buffer.write_all(b"Hello, world!\n").unwrap();
        buffer.flush().unwrap();
        let path = file.path();
        let first_line: Result<String, crate::Error> = read_first_line(path);
        assert_eq!(first_line.unwrap(), "Hello, world!\n");
    }

    #[test]
    fn test_read_first_line_empty_file() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path();
        let result = read_first_line(path);
        assert_eq!(result.unwrap(), "");
    }

    #[test]
    fn test_read_first_line_nonexistent_file() {
        let path = Path::new("nonexistent_file.txt");
        let result: Result<String, crate::Error> = read_first_line(path);
        assert!(result.is_err());
    }

    #[test]
    fn test_write_string_file() {
        let file = NamedTempFile::new().unwrap();
        let path = file.path();
        let result = write_string_file(path, String::from("Hello, world!\n"));
        let first_line = read_first_line(path);
        assert!(result.is_ok());
        assert_eq!(first_line.unwrap(), "Hello, world!\n");
    }

    #[test]
    fn test_write_string_noneexistent_file() {
        let path = Path::new("nonexistent_file.txt");
        let result = write_string_file(path, String::from("Hello, world!\n"));
        assert!(result.is_err())
    }

    #[test]
    fn test_path_equal() {
        assert!(path_equal("/etc", "/etc"));
        assert!(path_equal("//etc", "/etc"));
        assert!(path_equal("/etc//", "/etc"));
        assert!(!path_equal("/etc", "./etc"));
        assert!(path_equal("/x/./y", "/x/y"));
        assert!(path_equal("/x/././y", "/x/y/./."));
        assert!(!path_equal("/etc", "/var"));
    }

    #[test]
    fn test_path_name_is_safe() {
        assert!(!path_name_is_safe(""));
        assert!(path_name_is_safe("/abc"));
        assert!(!path_name_is_safe("/abc\x7f/a"));
        assert!(!path_name_is_safe("/abc\x1f/a"));
        assert!(!path_name_is_safe("/\x0a/a"));
    }

    #[test]
    fn test_path_length_is_valid() {
        assert!(!path_length_is_valid(""));

        let path = "/a/".to_string() + &String::from_iter(vec!['1'; 255]);
        assert!(path_length_is_valid(&path));

        let path = "/a/".to_string() + &String::from_iter(vec!['1'; 256]);
        assert!(!path_length_is_valid(&path));

        let path = "/a/".to_string() + &String::from_iter(vec!['/'; 256]);
        assert!(path_length_is_valid(&path));

        let path = "/a/".to_string() + &String::from_iter(vec!['.'; 255]);
        assert!(path_length_is_valid(&path));

        let mut path = "".to_string();
        for _ in 0..40 {
            path += "/";
            path += &String::from_iter(vec!['1'; 100]);
        }
        assert!(path_length_is_valid(&path));

        let mut path = "".to_string();
        for _ in 0..41 {
            path += "/";
            path += &String::from_iter(vec!['1'; 100]);
        }
        assert!(!path_length_is_valid(&path));
    }

    #[test]
    fn test_path_simplify() {
        assert_eq!(path_simplify("//foo//.//bar/").unwrap(), "/foo/bar");
        assert_eq!(path_simplify(".//foo//.//bar/").unwrap(), "foo/bar");
        assert_eq!(path_simplify("foo//.//bar/").unwrap(), "foo/bar");
        assert_eq!(path_simplify("/a///b/////././././c").unwrap(), "/a/b/c");
        assert_eq!(path_simplify(".//././///././././a").unwrap(), "a");
        assert_eq!(path_simplify("/////////////////").unwrap(), "/");
        assert_eq!(path_simplify(".//////////////////").unwrap(), ".");
        assert_eq!(
            path_simplify("a/b/c../..d/e/f//g").unwrap(),
            "a/b/c../..d/e/f/g"
        );
        assert_eq!(
            path_simplify("aaa/bbbb/.//.//.....").unwrap(),
            "aaa/bbbb/....."
        );

        assert_eq!(path_simplify("a/b/c/../../d").unwrap(), "a/d");
        assert_eq!(path_simplify("a/b/c/../../..").unwrap(), ".");
        assert_eq!(path_simplify("a/b/../../../c/d").unwrap(), "../c/d");
        assert_eq!(path_simplify("../../../a/../").unwrap(), "../../..");
        assert!(path_simplify("/../../../a/../").is_none());
    }

    #[test]
    fn test_path_is_abosolute() {
        assert!(path_is_abosolute("/a"));
        assert!(path_is_abosolute("//"));
        assert!(!path_is_abosolute("a"));
    }

    #[test]
    fn test_init_lookup_paths() {
        let mut lp = LookupPaths::default();
        lp.init_lookup_paths();
        assert_eq!(
            lp.search_path,
            vec![
                "/run/sysmaster/transient",
                "/etc/sysmaster/system",
                "/run/sysmaster/system",
                "/usr/lib/sysmaster/system"
            ]
        );
        assert_eq!(
            lp.preset_path,
            vec![
                "/etc/sysmaster/system/system-preset",
                "/usr/lib/sysmaster/system/system-preset"
            ]
        );
        assert_eq!(lp.persistent_path, "/etc/sysmaster/system")
    }

    #[test]
    fn test_symlink() {
        // use a complicated long name to make sure we don't have this file
        // before running this testcase.
        let link_name_path = std::path::Path::new("/tmp/test_link_name_39285b");
        if link_name_path.exists() {
            let _ = unlink(link_name_path);
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

        let _ = unlink("/tmp/test_not_exist");
        assert!(symlink("/dev/not_exist", "/tmp/test_not_exist", false).is_ok());
        let _ = unlink("/tmp/test_not_exist");
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

        match fchmod_and_chown(
            fd,
            "",
            Some(0o664),
            Some(Uid::from_raw(10)),
            Some(Gid::from_raw(10)),
        ) {
            Ok(_) => {
                let stat = fstat(fd).unwrap();
                assert_eq!(stat.st_uid, 10);
                assert_eq!(stat.st_gid, 10);
            }
            Err(e) => match e {
                Error::Nix { source } => {
                    assert_eq!(source, nix::Error::EPERM);
                }
                _ => {
                    panic!("{}", e);
                }
            },
        }

        match fchmod_and_chown(
            fd,
            "",
            Some(0o555),
            Some(Uid::from_raw(20)),
            Some(Gid::from_raw(20)),
        ) {
            Ok(_) => {
                let stat = fstat(fd).unwrap();
                assert_eq!(stat.st_mode & 0o777, 0o555);
                assert_eq!(stat.st_uid, 20);
                assert_eq!(stat.st_gid, 20);
            }
            Err(e) => match e {
                Error::Nix { source } => {
                    assert_eq!(source, nix::Error::EPERM);
                }
                _ => {
                    panic!("{}", e);
                }
            },
        }

        fchmod_and_chown(fd, "", Some(0o7555), None, None).unwrap();

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

        /* Test invalid raw fd. */
        assert!(futimens_opath(1000, None).is_err());
    }

    #[test]
    fn test_futimens_opath_in_thread() {
        let _ = File::create("/tmp/test_futimens_opath_in_thread").unwrap();
        let fd = nix::fcntl::open(
            "/tmp/test_futimens_opath_in_thread",
            OFlag::O_PATH | OFlag::O_CLOEXEC,
            Mode::empty(),
        )
        .unwrap();
        let handle = std::thread::spawn(move || {
            futimens_opath(fd, None).unwrap();
        });
        handle.join().unwrap();
        std::fs::remove_file("/tmp/test_futimens_opath_in_thread").unwrap();
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

    #[test]
    fn test_open_parent() {
        let _ = unlink("/tmp/test_open_parent/dir");
        touch_file("/tmp/test_open_parent/dir", true, Some(0o444), None, None).unwrap();
        {
            let f = open_parent(
                Path::new("/tmp/test_open_parent/dir"),
                OFlag::O_RDONLY,
                Mode::empty(),
            )
            .unwrap();
            let _md = f.metadata().unwrap();

            /* permission denied */
            assert!(open_parent(
                Path::new("/tmp/test_open_parent/dir"),
                OFlag::O_RDWR,
                Mode::from_bits_truncate(0o777),
            )
            .is_err());
        }
        remove_dir_all("/tmp/test_open_parent").unwrap();

        assert!(open_parent(
            Path::new("/tmp/test_open_parent/not_exist"),
            OFlag::O_RDONLY,
            Mode::empty(),
        )
        .is_err());
    }

    #[test]
    fn test_macro() {
        fn inner_function() -> Result<()> {
            let p = Path::new("/tmp/test_macro/all");
            do_entry_or_return_io_error!(std::fs::create_dir_all, p, "create");
            Ok(())
        }

        let _ = remove_dir_all("/tmp/test_macro");
        assert!(inner_function().is_ok());

        let p = Path::new("/tmp/test_macro/all_2");
        do_entry_log!(std::fs::create_dir_all, p, "create");
        assert!(remove_dir_all("/tmp/test_macro").is_ok());
    }

    #[test]
    fn test_is_symlink() {
        let link_name_path = std::path::Path::new("/tmp/test_is_symlink");
        if link_name_path.exists() {
            let _ = unlink(link_name_path);
        }

        symlink("/dev/null", "/tmp/test_is_symlink", false).unwrap();

        assert!(is_symlink(link_name_path));

        let _ = unlink("/tmp/test_is_symlink");
    }

    #[test]
    fn test_chase_symlink() {
        let _ = std::fs::File::create("final_target").unwrap();
        symlink("./final_target", "./link1", false).unwrap();
        symlink("./link1", "./link2", false).unwrap();
        assert_eq!(
            chase_symlink(Path::new("./link2")).unwrap(),
            PathBuf::from("final_target")
        );
        let mut prev = "./link2".to_string();
        for i in 3..33 {
            let cur = format!("./link{}", i);
            symlink(&prev, &cur, false).unwrap();
            prev = cur;
        }
        assert_eq!(
            chase_symlink(Path::new("./link32")).unwrap(),
            PathBuf::from("final_target")
        );
        symlink("./link32", "./link33", false).unwrap();
        assert!(chase_symlink(Path::new("./link33")).is_err());

        let _ = unlink("final_target");
        assert!(chase_symlink(Path::new("./link1")).is_err());

        for i in 1..34 {
            let path = format!("./link{}", i);
            let _ = unlink(Path::new(&path));
        }
    }
}
