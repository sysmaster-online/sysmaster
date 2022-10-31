//! # 一种对epoll接口的封装

use libc::epoll_event;
use libutils::Result;
use std::os::unix::{io::AsRawFd, io::RawFd};

pub(crate) mod epoll;
#[cfg(unix)]
use epoll::Epoll as Poller;

/// 一种对epoll接口的封装
#[derive(Debug, Default)]
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
        let _ = poll.register(listener.as_raw_fd(), &mut events);
        let _ = poll.poll(0).unwrap();
        let _ = poll.reregister(listener.as_raw_fd(), &mut events);
        let _ = poll.poll(0).unwrap();
        let _ = poll.unregister(listener.as_raw_fd());
    }
}
