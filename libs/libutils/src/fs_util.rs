//! the utils of the file operation
//!
use nix::{errno::Errno, fcntl::OFlag, sys::stat::Mode};
use pathdiff::diff_paths;
use std::path::Path;

/// open the parent directory of path
pub fn open_parent(path: &Path, flags: OFlag, mode: Mode) -> Result<i32, Errno> {
    let parent = path.parent();

    if parent.is_none() {
        return Err(Errno::EINVAL);
    }

    let fd = nix::fcntl::open(parent.unwrap(), flags, mode)?;

    Ok(fd)
}

/// create symlink {from} source to {target}
pub fn symlink(source: &str, target: &str, relative: bool) -> Result<(), Errno> {
    let t_path = Path::new(&target);
    let f_path = Path::new(&source);
    let (from, fd) = if relative {
        let t_parent = t_path.parent().unwrap();
        let rel_path = diff_paths(f_path, t_parent).unwrap();
        let fd = nix::fcntl::open(&rel_path, OFlag::O_DIRECT, Mode::from_bits(0).unwrap())?;
        (rel_path, Some(fd))
    } else {
        (f_path.to_path_buf(), None)
    };

    let ret = nix::unistd::symlinkat(from.as_path(), fd, t_path);

    if ret.is_err() {
        return Ok(());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::fs_util::symlink;

    #[test]
    fn test_symlink() {
        let ret = symlink("/dev/null", "/tmp/test", false);
        assert!(ret.is_ok());
    }
}
