use nix::{
    errno::Errno,
    libc,
    poll::{self, PollFd, PollFlags},
    sys::{signal::SigSet, time::TimeSpec},
};
use std::os::unix::prelude::RawFd;

fn ppoll_timeout(fds: &mut [PollFd], timeout: Option<TimeSpec>) -> Result<libc::c_int, Errno> {
    if fds.is_empty() {
        return Ok(0);
    }

    let ret = poll::ppoll(fds, timeout, SigSet::empty())?;

    if ret == 0 {
        return Ok(0);
    }

    for item in fds {
        if item.revents().is_none() {
            continue;
        }

        if item.revents().unwrap().eq(&PollFlags::POLLNVAL) {
            return Err(Errno::EBADF);
        }
    }

    Ok(ret)
}

pub fn wait_for_events(fd: RawFd, event: PollFlags, time_out: i64) -> Result<libc::c_int, Errno> {
    let poll_fd = PollFd::new(fd, event);
    let time_spec = TimeSpec::from_timespec(libc::timespec {
        tv_sec: time_out,
        tv_nsec: 0,
    });
    let mut fds = [poll_fd];

    let ret = ppoll_timeout(&mut fds, Some(time_spec))?;

    Ok(ret)
}
