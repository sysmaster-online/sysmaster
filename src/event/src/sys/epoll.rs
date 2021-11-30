use std::{io, os::unix::io::RawFd, ptr, sync::atomic::{AtomicUsize, Ordering::Relaxed}};
use libc::{epoll_event, EPOLL_CLOEXEC, EPOLL_CTL_ADD, EPOLL_CTL_DEL, EPOLL_CTL_MOD};

#[allow(unused_macros)]
macro_rules! syscall {
    ($fn: ident ( $($arg: expr),* $(,)* ) ) => {{
        let res = unsafe { libc::$fn($($arg, )*) };
        if res == -1 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(res)
        }
    }};
}

#[derive(Debug)]
pub struct Epoll {
    epoll_fd: RawFd,
    n_sources: AtomicUsize,
}

impl Epoll {
    pub fn new() -> io::Result<Epoll> {
        syscall!(epoll_create1(EPOLL_CLOEXEC)).map( | ep | 
            Epoll { epoll_fd: ep,
            n_sources: AtomicUsize::new(0), 
        })
    }

    pub fn poll(&mut self, timeout: Option<std::time::Duration>) -> io::Result<Vec<epoll_event>> {
        let mut events = Vec::<epoll_event>::with_capacity(self.n_sources.load(Relaxed));
        let _n_ready = syscall!(epoll_wait(
            self.epoll_fd,
            events.as_mut_ptr(),
            events.capacity() as i32,
            timeout.map(|to| to.as_millis() as libc::c_int).unwrap_or(-1),
        ));
        Ok(events)
    }
    
    pub fn register(&mut self, fd: RawFd, event: &mut epoll_event) -> io::Result<()> {
        self.n_sources.fetch_add(1, Relaxed);
        syscall!(epoll_ctl(self.epoll_fd, EPOLL_CTL_ADD, fd, event)).map(|_| ())
    }

    pub fn reregister(&mut self, fd: RawFd, event: &mut epoll_event) -> io::Result<()> {
        syscall!(epoll_ctl(self.epoll_fd, EPOLL_CTL_MOD, fd, event)).map(|_| ())
    }
    
    pub fn unregister(&mut self, fd: RawFd) -> io::Result<()> {
        self.n_sources.fetch_sub(1, Relaxed);
        syscall!(epoll_ctl(self.epoll_fd, EPOLL_CTL_DEL, fd, ptr::null_mut())).map(|_| ())
    }
}

impl Drop for Epoll {
    fn drop(&mut self) {
        let _ = syscall!(close(self.epoll_fd));
    }
}

#[cfg(test)]
mod test {
    use libc::EPOLLIN;
    use super::Epoll;

    #[cfg(unix)]
    #[test]
    fn epoll_new() {
        let _ = Epoll::new();
    }

    #[cfg(unix)]
    #[test]
    fn epoll_add() {
        let mut poll = Epoll::new().unwrap();
        let mut events = libc::epoll_event {events: EPOLLIN as u32, u64: 0,};
        poll.register(poll.epoll_fd, &mut events);
        poll.unregister(poll.epoll_fd);
    }
}