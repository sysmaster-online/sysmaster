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

use nix::errno::Errno;
use nix::sys::epoll::{self, EpollEvent, EpollFlags, EpollOp};
use nix::sys::socket;
use nix::unistd;
use std::cmp::max;
use std::os::fd::RawFd;
use std::str;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct Epoll {
    pub epoll_fd: RawFd,
    n_sources: AtomicUsize,
}

impl Epoll {
    pub(crate) fn new() -> Result<Epoll, Errno> {
        let epoll_fd = epoll::epoll_create1(epoll::EpollCreateFlags::empty())?;
        Ok(Epoll {
            epoll_fd,
            n_sources: AtomicUsize::new(0),
        })
    }

    pub(crate) fn wait(&self) -> Result<Vec<EpollEvent>, Errno> {
        let size = max(self.n_sources.load(Ordering::Relaxed), 1);
        let event = epoll::EpollEvent::new(epoll::EpollFlags::empty(), 0);
        let mut events = vec![event; size];

        let res = epoll::epoll_wait(self.epoll_fd, &mut events, -1);
        let ep_size = match res {
            Ok(size) => size,
            Err(err) => {
                if Errno::EINTR == err {
                    return Ok(Vec::<EpollEvent>::with_capacity(0));
                }
                eprintln!("Failed to epoll_wait! {:?}", err);
                return Err(err);
            }
        };

        unsafe {
            events.set_len(ep_size);
        }
        Ok(events)
    }

    pub(crate) fn register(&self, fd: RawFd) -> Result<(), Errno> {
        self.n_sources.fetch_add(1, Ordering::Relaxed);
        let mut event = epoll::EpollEvent::new(epoll::EpollFlags::EPOLLIN, fd as u64);
        epoll::epoll_ctl(self.epoll_fd, EpollOp::EpollCtlAdd, fd, &mut event)
    }

    pub(crate) fn unregister(&self, fd: RawFd) -> Result<(), Errno> {
        self.n_sources.fetch_sub(1, Ordering::Relaxed);
        epoll::epoll_ctl(self.epoll_fd, EpollOp::EpollCtlDel, fd, None)?;
        self.safe_close(fd);
        Ok(())
    }

    pub(crate) fn recv_nowait(&self, fd: RawFd) -> Result<String, Errno> {
        let mut buffer = [0u8; 4096];
        let mut count = 0;
        loop {
            let buflen = match socket::recv(fd, &mut buffer, socket::MsgFlags::MSG_DONTWAIT) {
                Ok(len) => len,
                Err(err) => {
                    if Errno::EINTR == err {
                        continue;
                    }
                    if (Errno::EAGAIN == err || Errno::EWOULDBLOCK == err) && count < 3 {
                        count += 1;
                        continue;
                    }
                    return Err(err);
                }
            };

            match str::from_utf8(&buffer[..buflen]) {
                Ok(v) => {
                    return Ok(v.to_string());
                }
                Err(_) => return Err(Errno::EINVAL),
            }
        }
    }

    pub(crate) fn read(&self, fd: RawFd) -> Result<(), Errno> {
        let mut buffer = [0u8; 4096];
        if let Err(err) = unistd::read(fd, &mut buffer) {
            eprintln!("read failed! err:{:?}", err);
            if Errno::EAGAIN == err || Errno::EINTR == err {
                return Ok(());
            }
            return Err(err);
        }
        Ok(())
    }

    pub(crate) fn event_is_err(&self, ep_flags: EpollFlags) -> bool {
        if (ep_flags & EpollFlags::EPOLLERR) == EpollFlags::EPOLLERR
            || (ep_flags & EpollFlags::EPOLLHUP) == EpollFlags::EPOLLHUP
        {
            return true;
        }

        false
    }

    pub(crate) fn clear(&self) {
        self.safe_close(self.epoll_fd);
    }

    pub(crate) fn safe_close(&self, fd: RawFd) {
        if fd < 0 {
            return;
        }

        if let Err(err) = unistd::close(fd) {
            eprintln!("failed to close fd:{:?} err:{:?}", fd, err);
        };
    }
}
