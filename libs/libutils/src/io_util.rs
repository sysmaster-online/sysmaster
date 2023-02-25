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
    libc,
    poll::{self, PollFd, PollFlags},
    sys::{signal::SigSet, time::TimeSpec},
};
use std::os::unix::prelude::RawFd;

fn ppoll_timeout(fds: &mut [PollFd], timeout: Option<TimeSpec>) -> Result<libc::c_int> {
    if fds.is_empty() {
        return Ok(0);
    }

    let ret = poll::ppoll(fds, timeout, SigSet::empty()).context(NixSnafu)?;

    if ret == 0 {
        return Ok(0);
    }

    for item in fds {
        if item.revents().is_none() {
            continue;
        }

        if item.revents().unwrap().eq(&PollFlags::POLLNVAL) {
            return Err(Error::Nix {
                source: nix::errno::Errno::EBADF,
            });
        }
    }

    Ok(ret)
}

///
pub fn wait_for_events(fd: RawFd, event: PollFlags, time_out: i64) -> Result<libc::c_int> {
    let poll_fd = PollFd::new(fd, event);
    let time_spec = TimeSpec::from_timespec(libc::timespec {
        tv_sec: time_out,
        tv_nsec: 0,
    });
    let mut fds = [poll_fd];

    let ret = ppoll_timeout(&mut fds, Some(time_spec))?;

    Ok(ret)
}
