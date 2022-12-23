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

/// create symlink link_name -> target
pub fn symlink(target: &str, link_name: &str, relative: bool) -> Result<(), Errno> {
    let link_name_path = Path::new(&link_name);
    let target_path = Path::new(&target);
    let (target_path, fd) = if relative {
        let link_name_path_parent = link_name_path.parent().unwrap();
        let rel_path = diff_paths(target_path, link_name_path_parent).unwrap();
        let fd = nix::fcntl::open(&rel_path, OFlag::O_DIRECT, Mode::from_bits(0).unwrap())?;
        (rel_path, Some(fd))
    } else {
        (target_path.to_path_buf(), None)
    };

    match nix::unistd::symlinkat(target_path.as_path(), fd, link_name_path) {
        Ok(()) => Ok(()),
        Err(e) => {
            log::debug!("Failed to create symlink: {} -> {}", link_name, target);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::fs_util::symlink;
    use nix::unistd;

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
}
