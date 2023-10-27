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
use nix::{
    errno::Errno,
    fcntl::{FcntlArg, FdFlag, OFlag},
};

///
pub fn fd_nonblock(fd: i32, nonblock: bool) -> Result<(), Errno> {
    assert!(fd >= 0);

    let flags = nix::fcntl::fcntl(fd, FcntlArg::F_GETFL)?;
    let fd_flag = unsafe { OFlag::from_bits_unchecked(flags) };

    let n_block = match nonblock {
        true => OFlag::O_NONBLOCK,
        false => !OFlag::O_NONBLOCK,
    };
    let nflag = fd_flag & n_block;

    nix::fcntl::fcntl(fd, FcntlArg::F_SETFL(nflag))?;

    Ok(())
}

///
pub fn fd_cloexec(fd: i32, cloexec: bool) -> Result<(), Errno> {
    assert!(fd >= 0);

    let flags = nix::fcntl::fcntl(fd, FcntlArg::F_GETFD)?;

    let fd_flag = unsafe { FdFlag::from_bits_unchecked(flags) };

    let nflag = match cloexec {
        true => fd_flag | FdFlag::FD_CLOEXEC,
        false => fd_flag & !FdFlag::FD_CLOEXEC,
    };

    nix::fcntl::fcntl(fd, FcntlArg::F_SETFD(nflag))?;

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
