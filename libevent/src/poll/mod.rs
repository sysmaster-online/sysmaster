use libc::epoll_event;
use std::os::unix::{io::AsRawFd, io::RawFd};
use utils::Result;

pub mod epoll;
#[cfg(unix)]
use epoll::Epoll as Poller;

#[derive(Debug, Default)]
pub struct Poll {
    poller: Poller,
}

impl Poll {
    pub fn new() -> Result<Poll> {
        Ok(Poll {
            poller: Poller::new()?,
        })
    }

    pub fn try_clone(&self) -> Result<Poll> {
        Ok(Poll {
            poller: self.poller.try_clone().unwrap(),
        })
    }

    pub fn poll(&self, timeout: i32) -> Result<Vec<epoll_event>> {
        self.poller.poll(timeout)
    }

    pub fn register(&mut self, fd: RawFd, event: &mut epoll_event) -> Result<()> {
        self.poller.register(fd, event)
    }

    pub fn reregister(&mut self, fd: RawFd, event: &mut epoll_event) -> Result<()> {
        self.poller.reregister(fd, event)
    }

    pub fn unregister(&mut self, fd: RawFd) -> Result<()> {
        self.poller.unregister(fd)
    }
}

impl AsRawFd for Poll {
    fn as_raw_fd(&self) -> RawFd {
        self.poller.as_raw_fd()
    }
}
