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

//!
use crate::error::*;
use nix::{
    errno::Errno,
    fcntl::{openat, FcntlArg, FdFlag, OFlag},
    ioctl_read,
    sys::stat::{Mode, SFlag},
};
use std::os::unix::prelude::FromRawFd;
use std::{fs::File, os::unix::prelude::RawFd};

/// check if the given stat.st_mode is regular file
pub fn stat_is_reg(st_mode: u32) -> bool {
    st_mode & SFlag::S_IFMT.bits() & SFlag::S_IFREG.bits() > 0
}

/// check if the given stat.st_mode is char device
pub fn stat_is_char(st_mode: u32) -> bool {
    st_mode & SFlag::S_IFMT.bits() & SFlag::S_IFCHR.bits() > 0
}

///
pub fn fd_nonblock(fd: RawFd, nonblock: bool) -> Result<()> {
    assert!(fd >= 0);

    let flags = nix::fcntl::fcntl(fd, FcntlArg::F_GETFL).context(NixSnafu)?;
    let fd_flag = unsafe { OFlag::from_bits_unchecked(flags) };

    let nflag = match nonblock {
        true => fd_flag | OFlag::O_NONBLOCK,
        false => fd_flag & !OFlag::O_NONBLOCK,
    };

    if nflag == fd_flag {
        return Ok(());
    }

    nix::fcntl::fcntl(fd, FcntlArg::F_SETFL(nflag)).context(NixSnafu)?;

    Ok(())
}

///
pub fn fd_cloexec(fd: RawFd, cloexec: bool) -> Result<()> {
    assert!(fd >= 0);

    let flags = nix::fcntl::fcntl(fd, FcntlArg::F_GETFD).context(NixSnafu)?;

    let fd_flag = unsafe { FdFlag::from_bits_unchecked(flags) };

    let nflag = match cloexec {
        true => fd_flag | FdFlag::FD_CLOEXEC,
        false => fd_flag & !FdFlag::FD_CLOEXEC,
    };

    nix::fcntl::fcntl(fd, FcntlArg::F_SETFD(nflag)).context(NixSnafu)?;

    Ok(())
}

///
pub fn fd_is_cloexec(fd: RawFd) -> bool {
    assert!(fd >= 0);

    let flags = nix::fcntl::fcntl(fd, FcntlArg::F_GETFD).unwrap_or(0);
    let fd_flag = FdFlag::from_bits(flags).unwrap();
    fd_flag.contains(FdFlag::FD_CLOEXEC)
}

///
pub fn close(fd: RawFd) {
    if let Err(e) = nix::unistd::close(fd) {
        log::warn!("close fd {} failed, errno: {}", fd, e);
    }
}

/// reopen the specified fd with new flags to convert an O_PATH fd into
/// regular one, or to turn O_RDWR fds into O_RDONLY fds
///
/// this function can not work on sockets, as they can not be opened
///
/// note that this function implicitly reset the read index to zero
pub fn fd_reopen(fd: RawFd, oflags: OFlag) -> Result<File> {
    if oflags.intersects(OFlag::O_DIRECTORY) {
        let new_fd = nix::fcntl::openat(fd, ".", oflags, nix::sys::stat::Mode::empty())
            .map_err(|e| Error::Nix { source: e })?;

        return Ok(unsafe { File::from_raw_fd(new_fd) });
    }

    match nix::fcntl::open(
        format!("/proc/self/fd/{}", fd).as_str(),
        oflags,
        nix::sys::stat::Mode::empty(),
    ) {
        Ok(n) => Ok(unsafe { File::from_raw_fd(n) }),
        Err(e) => {
            if e != Errno::ENOENT {
                return Err(Error::Nix { source: e });
            }

            if !crate::stat_util::proc_mounted().map_err(|_| Error::Nix {
                source: Errno::ENOENT,
            })? {
                // if /proc/ is not mounted, this function can not work
                Err(Error::Nix {
                    source: Errno::ENOSYS,
                })
            } else {
                // if /proc/ is mounted, means this fd is not valid
                Err(Error::Nix {
                    source: Errno::EBADF,
                })
            }
        }
    }
}

const BLK_DISKSEQ_MAGIC: u8 = 18;
const BLK_GET_DISKSEQ: u8 = 128;
ioctl_read!(
    /// get the diskseq from block
    blk_get_diskseq,
    BLK_DISKSEQ_MAGIC,
    BLK_GET_DISKSEQ,
    u64
);

/// get the diskseq according to fd
pub fn fd_get_diskseq(fd: RawFd) -> Result<u64> {
    let mut diskseq: u64 = 0;
    let ptr: *mut u64 = &mut diskseq;
    unsafe {
        match blk_get_diskseq(fd, ptr) {
            Ok(_) => {}
            Err(e) => {
                if !crate::error::errno_is_not_supported(e) && e != Errno::EINVAL {
                    return Err(Error::Nix { source: e });
                }

                return Err(Error::Nix {
                    source: Errno::EOPNOTSUPP,
                });
            }
        }
    }
    Ok(diskseq)
}

/// open the directory at fd
pub fn opendirat(dirfd: i32, flags: OFlag) -> Result<nix::dir::Dir> {
    let nfd = openat(
        dirfd,
        ".",
        OFlag::O_RDONLY | OFlag::O_NONBLOCK | OFlag::O_DIRECTORY | OFlag::O_CLOEXEC | flags,
        Mode::empty(),
    )
    .context(NixSnafu)?;

    nix::dir::Dir::from_fd(nfd).context(NixSnafu)
}

#[cfg(test)]
mod tests {
    use crate::fd_util::{stat_is_char, stat_is_reg};
    use nix::{
        fcntl::{open, OFlag},
        sys::stat::{fstat, Mode},
    };
    use std::{
        fs::{remove_dir_all, File},
        os::unix::prelude::AsRawFd,
        path::Path,
    };

    use super::opendirat;

    #[test]
    fn test_stats() {
        let fd_reg_file = File::open(Path::new("/bin/true")).unwrap();
        assert!(fd_reg_file.as_raw_fd() >= 0);
        let st = fstat(fd_reg_file.as_raw_fd()).unwrap();
        assert!(stat_is_reg(st.st_mode));

        let fd_non_reg_file = File::open(Path::new("/proc/1")).unwrap();
        assert!(fd_non_reg_file.as_raw_fd() >= 0);
        let st = fstat(fd_non_reg_file.as_raw_fd()).unwrap();
        assert!(!stat_is_reg(st.st_mode));

        let fd_char_file = File::open(Path::new("/dev/zero")).unwrap();
        assert!(fd_char_file.as_raw_fd() >= 0);
        let st = fstat(fd_char_file.as_raw_fd()).unwrap();
        assert!(stat_is_char(st.st_mode));

        let fd_non_char_file = File::open(Path::new("/proc/1")).unwrap();
        assert!(fd_non_char_file.as_raw_fd() >= 0);
        let st = fstat(fd_non_char_file.as_raw_fd()).unwrap();
        assert!(!stat_is_char(st.st_mode));
    }

    #[test]
    fn test_opendirat() {
        std::fs::create_dir_all("/tmp/test_opendirat").unwrap();
        File::create("/tmp/test_opendirat/entry0").unwrap();
        File::create("/tmp/test_opendirat/entry1").unwrap();

        let dirfd = open("/tmp/test_opendirat", OFlag::O_DIRECTORY, Mode::empty()).unwrap();
        let mut dir = opendirat(dirfd, OFlag::O_NOFOLLOW).unwrap();

        for e in dir.iter() {
            let _ = e.unwrap();
        }

        remove_dir_all("/tmp/test_opendirat").unwrap();
    }
}
