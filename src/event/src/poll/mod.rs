mod epoll;

use epoll::Epoll;
use libc::epoll_event;
use std::os::unix::{io::AsRawFd, io::RawFd};
use std::{io, time};

#[derive(Debug, Default)]
pub struct Poll {
    poller: Epoll,
}

impl Poll {
    pub fn new() -> io::Result<Poll> {
        Ok(Poll {
            poller: Epoll::new()?,
        })
    }

    pub fn try_clone(&self) -> io::Result<Poll> {
        Ok(Poll {
            poller: self.poller.try_clone().unwrap(),
        })
    }

    pub fn poll(&mut self, timeout: Option<time::Duration>) -> io::Result<Vec<epoll_event>> {
        self.poller.poll(timeout)
    }

    pub fn register(&mut self, fd: RawFd, event: &mut epoll_event) -> io::Result<()> {
        self.poller.register(fd, event)
    }

    pub fn reregister(&mut self, fd: RawFd, event: &mut epoll_event) -> io::Result<()> {
        self.poller.reregister(fd, event)
    }

    pub fn unregister(&mut self, fd: RawFd) -> io::Result<()> {
        self.poller.unregister(fd)
    }
}

impl AsRawFd for Poll {
    fn as_raw_fd(&self) -> RawFd {
        self.poller.as_raw_fd()
    }
}
