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

use constants::INVALID_FD;
use nix::errno::Errno;
use nix::sys::epoll::{self, EpollEvent, EpollFlags, EpollOp};
use nix::unistd;
use std::os::unix::prelude::RawFd;

pub struct Epoll {
    pub epoll_fd: RawFd,
}

impl Epoll {
    pub(crate) fn new() -> Result<Epoll, Errno> {
        let epoll_fd = epoll::epoll_create1(epoll::EpollCreateFlags::EPOLL_CLOEXEC)?;
        Ok(Epoll { epoll_fd })
    }

    pub(crate) fn wait_one(&self) -> epoll::EpollEvent {
        let event = EpollEvent::new(EpollFlags::empty(), 0);
        let mut events = vec![event; 1];
        let empty_event = EpollEvent::new(EpollFlags::empty(), 0);

        let res = epoll::epoll_wait(self.epoll_fd, &mut events, -1);
        match res {
            Ok(_) => events.pop().unwrap_or(empty_event),
            Err(err) => {
                eprintln!("Failed to epoll_wait! {:?}", err);
                empty_event
            }
        }
    }

    pub(crate) fn register(&self, fd: RawFd) -> Result<(), Errno> {
        let mut event = epoll::EpollEvent::new(epoll::EpollFlags::EPOLLIN, fd as u64);
        epoll::epoll_ctl(self.epoll_fd, EpollOp::EpollCtlAdd, fd, &mut event)
    }

    pub(crate) fn unregister(&self, fd: RawFd) -> Result<(), Errno> {
        epoll::epoll_ctl(self.epoll_fd, EpollOp::EpollCtlDel, fd, None)
    }

    pub(crate) fn is_err(&self, event: EpollEvent) -> bool {
        let ep_flags = event.events();
        if (ep_flags & EpollFlags::EPOLLERR) == EpollFlags::EPOLLERR
            || (ep_flags & EpollFlags::EPOLLHUP) == EpollFlags::EPOLLHUP
        {
            eprintln!("fd:{:?}, flags:{:?}", event.data(), ep_flags);
            return true;
        }

        false
    }

    pub(crate) fn safe_close(&self, fd: RawFd) {
        if fd <= 0 {
            return;
        }

        if let Err(err) = unistd::close(fd) {
            eprintln!("failed to close fd:{:?} err:{:?}", fd, err);
        };
    }
}

impl Drop for Epoll {
    fn drop(&mut self) {
        self.safe_close(self.epoll_fd);
        self.epoll_fd = INVALID_FD;
    }
}
