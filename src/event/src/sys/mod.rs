mod epoll;

use std::{io, os::unix::io::RawFd, time};
use epoll::Epoll as Epoll;
use libc::epoll_event;

#[derive(Debug)]
pub struct Poll {
    poller: Epoll,
}

impl Poll {
    pub(crate) fn new() -> io::Result<Poll> {
        Ok( Poll { poller: Epoll::new()?, })
    }

    pub(crate) fn poll(
        &mut self,
        timeout: Option<time::Duration>,
    ) -> io::Result<Vec<epoll_event>> {
        self.poller.poll(timeout)
    }

    pub(crate) unsafe fn register(
        &mut self,
        fd: RawFd,
        event: &mut epoll_event,
    ) -> io::Result<()> {
        self.poller.register(fd, event)
    }

    pub(crate) unsafe fn reregister(
        &mut self,
        fd: RawFd,
        event: &mut epoll_event,
    ) -> io::Result<()> {
        self.poller.reregister(fd, event)
    }

    pub(crate) fn unregister(&mut self, fd: RawFd) -> io::Result<()> {
        self.poller.unregister(fd)
    }
}
