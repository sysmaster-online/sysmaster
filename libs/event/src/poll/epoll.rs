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

use crate::Result;
use libc::{epoll_event, EPOLL_CLOEXEC, EPOLL_CTL_ADD, EPOLL_CTL_DEL, EPOLL_CTL_MOD};
use std::cmp::max;
use std::os::unix::io::{AsRawFd, RawFd};
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};

/// syscall
#[macro_export]
macro_rules! syscall {
    ($fn: ident ( $($arg: expr),* $(,)* ) ) => {{
        let res = unsafe { libc::$fn($($arg, )*) };
        if res < 0 {
            $crate::Result::Err($crate::Error::Syscall { syscall: stringify!($fn), errno: unsafe { *libc::__errno_location() }, ret: res })
        } else {
            $crate::Result::Ok(res)
        }
    }};
}

const LOWEST_FD: libc::c_int = 3;

#[derive(Debug)]
pub(crate) struct Epoll {
    epoll_fd: RawFd,
    n_sources: AtomicUsize,
}

impl Epoll {
    pub(crate) fn new() -> Result<Epoll> {
        syscall!(epoll_create1(EPOLL_CLOEXEC)).map(|ep| Epoll {
            epoll_fd: ep,
            n_sources: AtomicUsize::new(0),
        })
    }

    pub(crate) fn try_clone(&self) -> Result<Epoll> {
        syscall!(fcntl(self.epoll_fd, libc::F_DUPFD_CLOEXEC, LOWEST_FD)).map(|ep| Epoll {
            epoll_fd: ep,
            n_sources: AtomicUsize::new(0),
        })
    }

    pub(crate) fn poll(&self, timeout: i32) -> Result<Vec<epoll_event>> {
        let size = max(self.n_sources.load(Ordering::Relaxed), 1);
        let mut events = Vec::<epoll_event>::with_capacity(size);

        events.clear();

        let n_ready = syscall!(epoll_wait(
            self.epoll_fd,
            events.as_mut_ptr(),
            events.capacity() as i32,
            timeout,
        ));

        match n_ready {
            Ok(n_ready) => unsafe {
                events.set_len(n_ready as usize);
            },
            Err(e) => return Err(e),
        }

        Ok(events)
    }

    pub(crate) fn register(&mut self, fd: RawFd, event: &mut epoll_event) -> Result<()> {
        let res = syscall!(epoll_ctl(self.epoll_fd, EPOLL_CTL_ADD, fd, event)).map(|_| ());
        if res.is_ok() {
            self.n_sources.fetch_add(1, Ordering::Relaxed);
        }
        res
    }

    pub(crate) fn reregister(&mut self, fd: RawFd, event: &mut epoll_event) -> Result<()> {
        syscall!(epoll_ctl(self.epoll_fd, EPOLL_CTL_MOD, fd, event)).map(|_| ())
    }

    pub(crate) fn unregister(&mut self, fd: RawFd) -> Result<()> {
        self.n_sources.fetch_sub(1, Ordering::Relaxed);
        syscall!(epoll_ctl(self.epoll_fd, EPOLL_CTL_DEL, fd, ptr::null_mut())).map(|_| ())
    }
}

impl Drop for Epoll {
    fn drop(&mut self) {
        let _ = syscall!(close(self.epoll_fd));
    }
}

impl AsRawFd for Epoll {
    fn as_raw_fd(&self) -> RawFd {
        self.epoll_fd
    }
}
