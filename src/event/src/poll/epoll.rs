use libc::{epoll_event, EPOLL_CLOEXEC, EPOLL_CTL_ADD, EPOLL_CTL_DEL, EPOLL_CTL_MOD};
use std::os::unix::io::{AsRawFd, RawFd};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::{io, ptr};

#[allow(unused_macros)]
#[macro_export]
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

const LOWEST_FD: libc::c_int = 3;

#[derive(Debug, Default)]
pub(crate) struct Epoll {
    epoll_fd: RawFd,
    n_sources: AtomicUsize,
}

impl Epoll {
    pub(crate) fn new() -> io::Result<Epoll> {
        syscall!(epoll_create1(EPOLL_CLOEXEC)).map(|ep| Epoll {
            epoll_fd: ep,
            n_sources: AtomicUsize::new(0),
        })
    }

    pub(crate) fn try_clone(&self) -> io::Result<Epoll> {
        syscall!(fcntl(self.epoll_fd, libc::F_DUPFD_CLOEXEC, LOWEST_FD)).map(|ep| Epoll {
            epoll_fd: ep,
            n_sources: AtomicUsize::new(0),
        })
    }

    pub(crate) fn poll(&self, timeout: i32) -> io::Result<Vec<epoll_event>> {
        let mut events = Vec::<epoll_event>::with_capacity(self.n_sources.load(Relaxed));

        events.clear();
        let n_ready = syscall!(epoll_wait(
            self.epoll_fd,
            events.as_mut_ptr(),
            events.capacity() as i32,
            timeout,
        ));
        unsafe {
            events.set_len(n_ready.unwrap() as usize);
        }
        Ok(events)
    }

    pub(crate) fn register(&mut self, fd: RawFd, event: &mut epoll_event) -> io::Result<()> {
        self.n_sources.fetch_add(1, Relaxed);
        syscall!(epoll_ctl(self.epoll_fd, EPOLL_CTL_ADD, fd, event)).map(|_| ())
    }

    pub(crate) fn reregister(&mut self, fd: RawFd, event: &mut epoll_event) -> io::Result<()> {
        syscall!(epoll_ctl(self.epoll_fd, EPOLL_CTL_MOD, fd, event)).map(|_| ())
    }

    pub(crate) fn unregister(&mut self, fd: RawFd) -> io::Result<()> {
        self.n_sources.fetch_sub(1, Relaxed);
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

#[cfg(test)]
mod test {
    use super::Epoll;
    use libc::EPOLLIN;

    #[test]
    fn epoll_new() {
        let _ = Epoll::new();
    }

    #[test]
    fn epoll_add() {
        let mut poll = Epoll::new().unwrap();
        let mut events = libc::epoll_event {
            events: EPOLLIN as u32,
            u64: 0,
        };
        let _ = poll.register(poll.epoll_fd, &mut events);
        let _ = poll.unregister(poll.epoll_fd);
    }
}
