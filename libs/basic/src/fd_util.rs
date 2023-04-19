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
    fcntl::{FcntlArg, FdFlag, OFlag},
    ioctl_read,
    sys::stat::SFlag,
};

/// check if the given stat.st_mode is regular file
pub fn stat_is_reg(st_mode: u32) -> bool {
    st_mode & SFlag::S_IFMT.bits() & SFlag::S_IFREG.bits() > 0
}

/// check if the given stat.st_mode is char device
pub fn stat_is_char(st_mode: u32) -> bool {
    st_mode & SFlag::S_IFMT.bits() & SFlag::S_IFCHR.bits() > 0
}

///
pub fn fd_nonblock(fd: i32, nonblock: bool) -> Result<()> {
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
pub fn fd_cloexec(fd: i32, cloexec: bool) -> Result<()> {
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
pub fn fd_is_cloexec(fd: i32) -> bool {
    assert!(fd >= 0);

    let flags = nix::fcntl::fcntl(fd, FcntlArg::F_GETFD).unwrap_or(0);
    let fd_flag = FdFlag::from_bits(flags).unwrap();
    fd_flag.contains(FdFlag::FD_CLOEXEC)
}

///
pub fn close(fd: i32) {
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
pub fn fd_reopen(fd: i32, oflags: OFlag) -> Result<i32> {
    if oflags.intersects(OFlag::O_DIRECTORY) {
        let new_fd = nix::fcntl::openat(fd, ".", oflags, nix::sys::stat::Mode::empty())
            .map_err(|e| Error::Nix { source: e })?;

        return Ok(new_fd);
    }

    match nix::fcntl::open(
        format!("/proc/self/fd/{}", fd).as_str(),
        oflags,
        nix::sys::stat::Mode::empty(),
    ) {
        Ok(n) => Ok(n),
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
pub fn fd_get_diskseq(fd: i32) -> Result<u64> {
    let mut diskseq: u64 = 0;
    let ptr: *mut u64 = &mut diskseq;
    unsafe {
        match blk_get_diskseq(fd, ptr) {
            Ok(_) => {}
            Err(e) => {
                if !crate::errno_util::errno_is_not_supported(e) && e != Errno::EINVAL {
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

#[cfg(test)]
mod tests {
    use crate::fd_util::{stat_is_char, stat_is_reg};
    use nix::sys::stat::fstat;
    use std::{fs::File, os::fd::AsRawFd, path::Path};

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
}
