use std::{os::unix::prelude::RawFd, path::Path};

use nix::{
    errno::Errno,
    sys::socket::{self, sockopt, AddressFamily},
};

pub fn ipv6_is_supported() -> bool {
    let inet6 = Path::new("/proc/net/if_inet6");

    if inet6.exists() {
        return true;
    }

    false
}

pub fn set_pkginfo(fd: RawFd, family: AddressFamily, v: bool) -> Result<(), Errno> {
    match family {
        socket::AddressFamily::Inet => socket::setsockopt(fd as RawFd, sockopt::Ipv4PacketInfo, &v),
        socket::AddressFamily::Inet6 => {
            socket::setsockopt(fd as RawFd, sockopt::Ipv6RecvPacketInfo, &v)
        }
        _ => Err(Errno::EAFNOSUPPORT),
    }
}

pub fn set_pass_cred(fd: RawFd, v: bool) -> Result<(), Errno> {
    socket::setsockopt(fd, sockopt::PassCred, &v)
}

pub fn set_receive_buffer(fd: RawFd, v: usize) -> Result<(), Errno> {
    socket::setsockopt(fd, sockopt::RcvBuf, &v)
}

pub fn set_send_buffer(fd: RawFd, v: usize) -> Result<(), Errno> {
    socket::setsockopt(fd, sockopt::SndBuf, &v)
}
