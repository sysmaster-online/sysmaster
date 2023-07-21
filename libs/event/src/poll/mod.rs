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

//! Encapsulation of the epoll interface

use crate::Result;
use libc::epoll_event;
use std::os::unix::{io::AsRawFd, io::RawFd};
pub(crate) mod epoll;

#[cfg(unix)]
use epoll::Epoll as Poller;

/// Encapsulation of the epoll interface
#[derive(Debug)]
pub struct Poll {
    poller: Poller,
}

impl Poll {
    /// create a new poller
    pub fn new() -> Result<Poll> {
        Ok(Poll {
            poller: Poller::new()?,
        })
    }

    /// clone the poller
    pub fn try_clone(&self) -> Result<Poll> {
        Ok(Poll {
            poller: self.poller.try_clone().unwrap(),
        })
    }

    /// poll the poller
    pub fn poll(&self, timeout: i32) -> Result<Vec<epoll_event>> {
        self.poller.poll(timeout)
    }

    /// register the source to the poller
    pub fn register(&mut self, fd: RawFd, event: &mut epoll_event) -> Result<()> {
        self.poller.register(fd, event)
    }

    /// reregister the source to the poller
    pub fn reregister(&mut self, fd: RawFd, event: &mut epoll_event) -> Result<()> {
        self.poller.reregister(fd, event)
    }

    /// unregister the source from the poller
    pub fn unregister(&mut self, fd: RawFd) -> Result<()> {
        self.poller.unregister(fd)
    }
}

impl AsRawFd for Poll {
    fn as_raw_fd(&self) -> RawFd {
        self.poller.as_raw_fd()
    }
}

#[cfg(test)]
mod test {
    use super::Poll;
    use libc::EPOLLIN;
    use std::{net::TcpListener, os::unix::io::AsRawFd};

    #[test]
    fn epoll_new() {
        let _ = Poll::new();
    }

    #[test]
    fn epoll_add() {
        let mut poll = Poll::new().unwrap();
        let listener = TcpListener::bind("0.0.0.0:9098").unwrap();
        let mut events = libc::epoll_event {
            events: EPOLLIN as u32,
            u64: 0,
        };
        assert!(poll.try_clone().unwrap().as_raw_fd() > 0);
        let _ = poll.register(listener.as_raw_fd(), &mut events);
        let _ = poll.poll(0).unwrap();
        let _ = poll.reregister(listener.as_raw_fd(), &mut events);
        let _ = poll.poll(0).unwrap();
        let _ = poll.unregister(listener.as_raw_fd());
    }
}
