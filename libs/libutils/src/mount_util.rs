use nix::{
    errno::Errno,
    fcntl::AtFlags,
    sys::stat::{fstatat, SFlag},
};

pub fn mount_point_fd_valid(fd: i32, file_name: &str, flags: AtFlags) -> Result<bool, Errno> {
    assert!(fd >= 0);

    let flags = if flags.contains(AtFlags::AT_SYMLINK_FOLLOW) {
        flags & !AtFlags::AT_SYMLINK_FOLLOW
    } else {
        flags | AtFlags::AT_SYMLINK_FOLLOW
    };

    let f_stat = fstatat(fd, file_name, flags)?;
    if SFlag::S_IFLNK.bits() & f_stat.st_mode == SFlag::S_IFLNK.bits() {
        return Ok(false);
    }

    let d_stat = fstatat(fd, "", AtFlags::AT_EMPTY_PATH)?;

    if f_stat.st_dev == d_stat.st_dev && f_stat.st_ino == d_stat.st_ino {
        return Ok(true);
    }

    Ok(f_stat.st_dev != d_stat.st_dev)
}
