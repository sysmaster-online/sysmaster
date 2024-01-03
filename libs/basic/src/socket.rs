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

//!
use crate::error::*;
use crate::IN_SET;
use nix::sys::socket::{recv, MsgFlags};
use nix::{
    errno::Errno,
    sys::socket::{self, sockopt, AddressFamily},
};
use std::{os::unix::prelude::RawFd, path::Path};

///
/// The function checks if IPv6 is supported by checking the existence of the "/proc/net/if_inet6" file.
///
/// Returns:
///
/// a boolean value. If the file "/proc/net/if_inet6" exists, it will return true. Otherwise, it will
/// return false.
pub fn ipv6_is_supported() -> bool {
    let inet6 = Path::new("/proc/net/if_inet6");

    if inet6.exists() {
        return true;
    }

    false
}

///
/// The function `set_pkginfo` sets the packet information option for a given socket file descriptor and
/// address family.
///
/// Arguments:
///
/// * `fd`: The `fd` parameter is of type `RawFd`, which represents a raw file descriptor. It is used to
/// identify the socket on which the option is to be set.
/// * `family`: The `family` parameter is of type `AddressFamily` and represents the address family of
/// the socket. It can have one of the following values:
/// * `v`: The parameter `v` is a boolean value that determines whether to enable or disable a specific
/// socket option.
///
/// Returns:
///
/// The function `set_pkginfo` returns a `Result` type.
pub fn set_pkginfo(fd: RawFd, family: AddressFamily, v: bool) -> Result<()> {
    match family {
        socket::AddressFamily::Inet => {
            socket::setsockopt(fd as RawFd, sockopt::Ipv4PacketInfo, &v).context(NixSnafu)
        }
        socket::AddressFamily::Inet6 => {
            socket::setsockopt(fd as RawFd, sockopt::Ipv6RecvPacketInfo, &v).context(NixSnafu)
        }
        _ => Err(Error::Nix {
            source: Errno::EAFNOSUPPORT,
        }),
    }
}

///
/// The function `set_pass_cred` sets the `PassCred` socket option for a given file descriptor.
///
/// Arguments:
///
/// * `fd`: The `fd` parameter is of type `RawFd`, which represents a raw file descriptor. It is
/// typically used to refer to a socket or file descriptor in Unix-like systems.
/// * `v`: The parameter `v` is a boolean value that determines whether or not to enable the passing of
/// credentials on a socket. If `v` is `true`, it enables the passing of credentials, and if `v` is
/// `false`, it disables the passing of credentials.
///
/// Returns:
///
/// The function `set_pass_cred` returns a `Result` type.
pub fn set_pass_cred(fd: RawFd, v: bool) -> Result<()> {
    socket::setsockopt(fd, sockopt::PassCred, &v).context(NixSnafu)
}

///
/// The function sets the receive buffer size for a socket and checks if the size was set successfully.
///
/// Arguments:
///
/// * `fd`: fd is the file descriptor of the socket. It is used to identify the socket on which the
/// operation will be performed.
/// * `v`: The parameter `v` represents the desired size of the receive buffer. It is of type `usize`,
/// which means it should be a non-negative integer value. The function checks if `v` is greater than
/// the maximum value that can be represented by `isize` (a signed integer type)
///
/// Returns:
///
/// The function `set_receive_buffer` returns a `Result` type. If the operation is successful, it
/// returns `Ok(())`, indicating that no error occurred. If there is an error, it returns `Err(Error)`,
/// where `Error` is an enum type that represents different kinds of errors that can occur during the
/// operation.
pub fn set_receive_buffer(fd: RawFd, v: usize) -> Result<()> {
    /* Type of value is usize, so the v should smaller than the half of the value
     *  as the value = 2 * n.
     */
    if v > (std::isize::MAX) as usize {
        return Err(Error::Nix {
            source: Errno::ERANGE,
        });
    }

    // Set receive buffer size
    socket::setsockopt(fd, sockopt::RcvBuf, &v).context(NixSnafu)?;

    // The kernel has limitations of receive buffer, so we need to check if the size v was set.
    match socket::getsockopt(fd, sockopt::RcvBuf) {
        Ok(value) => {
            /* Ops, the walue didn't set successfully, we can try to set with RcvBufForce.
             *  By the way, the kernel doubles the value in the setsockopt, so we check that
             *  with 2 * v.
             */
            if value != 2 * v {
                return socket::setsockopt(fd, sockopt::RcvBufForce, &v).context(NixSnafu);
            }
            Ok(())
        }
        Err(e) => Err(Error::Nix { source: e }),
    }
}

///
/// The function `set_send_buffer` sets the send buffer size for a given file descriptor in Rust, with
/// error handling for exceeding the maximum value.
///
/// Arguments:
///
/// * `fd`: The parameter `fd` is of type `RawFd`, which is a platform-specific integer representing a
/// file descriptor. In this case, it is likely referring to a socket file descriptor.
/// * `v`: The parameter `v` represents the desired size of the send buffer. It should be smaller than
/// half of the maximum value of `usize`, which is `(std::isize::MAX) as usize`.
///
/// Returns:
///
/// The function `set_send_buffer` returns a `Result` type. If the operation is successful, it returns
/// `Ok(())`, indicating that there was no error. If there is an error, it returns `Err(Error)`, where
/// `Error` is an enum type that represents different kinds of errors that can occur during the
/// operation.
pub fn set_send_buffer(fd: RawFd, v: usize) -> Result<()> {
    /* Type of value is usize, so the v should smaller than the half of the value
     *  as the value = 2 * n.
     */
    if v > (std::isize::MAX) as usize {
        return Err(Error::Nix {
            source: Errno::ERANGE,
        });
    }

    // Set send buffer size
    socket::setsockopt(fd, sockopt::SndBuf, &v).context(NixSnafu)?;

    // The kernel has limitations of send buffer, so we need to check if the size v was set.
    match socket::getsockopt(fd, sockopt::SndBuf) {
        Ok(value) => {
            /* Ops, the walue didn't set successfully, we can try to set with SndBufForce.
             *  By the way, the kernel doubles the value in the setsockopt, so we check that
             *  with 2 * v.
             */
            if value != 2 * v {
                return socket::setsockopt(fd, sockopt::SndBufForce, &v).context(NixSnafu);
            }
            Ok(())
        }
        Err(e) => Err(Error::Nix { source: e }),
    }
}

/// Set keepalive properties
pub fn set_keepalive_state(fd: RawFd, v: bool) -> Result<()> {
    socket::setsockopt(fd, sockopt::KeepAlive, &v).context(NixSnafu)
}

/// Set the interval between the last data packet sent and the first keepalive probe
pub fn set_keepalive_timesec(fd: RawFd, v: u32) -> Result<()> {
    socket::setsockopt(fd, sockopt::TcpKeepIdle, &v).context(NixSnafu)
}

/// Set the interval between subsequential keepalive probes
pub fn set_keepalive_intervalsec(fd: RawFd, v: u32) -> Result<()> {
    socket::setsockopt(fd, sockopt::TcpKeepInterval, &v).context(NixSnafu)
}

/// Set the number of unacknowledged probes to send
pub fn set_keepalive_probes(fd: RawFd, v: u32) -> Result<()> {
    socket::setsockopt(fd, sockopt::TcpKeepCount, &v).context(NixSnafu)
}

/// Set Broadcast state
pub fn set_broadcast_state(fd: RawFd, v: bool) -> Result<()> {
    socket::setsockopt(fd, sockopt::Broadcast, &v).context(NixSnafu)
}

/// get the size of data in fd
pub fn next_datagram_size_fd(fd: RawFd) -> Result<usize> {
    /* This is a bit like FIONREAD/SIOCINQ, however a bit more powerful. The difference being: recv(MSG_PEEK) will
     * actually cause the next datagram in the queue to be validated regarding checksums, which FIONREAD doesn't
     * do. This difference is actually of major importance as we need to be sure that the size returned here
     * actually matches what we will read with recvmsg() next, as otherwise we might end up allocating a buffer of
     * the wrong size.
     */

    let mut buf = Vec::new();
    match recv(fd, &mut buf, MsgFlags::MSG_PEEK | MsgFlags::MSG_TRUNC) {
        Ok(len) => {
            if len != 0 {
                return Ok(len);
            }
        }
        Err(err) => {
            if !IN_SET!(err, Errno::EOPNOTSUPP, Errno::EFAULT) {
                return Err(Error::Nix { source: err });
            }
        }
    }

    /* Some sockets (AF_PACKET) do not support null-sized recv() with MSG_TRUNC set, let's fall back to FIONREAD
     * for them. Checksums don't matter for raw sockets anyway, hence this should be fine.
     */

    let k: usize = 0;
    Errno::clear();
    if unsafe { libc::ioctl(fd, libc::FIONREAD, &k) } < 0 {
        return Err(Error::Nix {
            source: Errno::from_i32(nix::errno::errno()),
        });
    }

    Ok(k)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nix::sys::socket::{send, socketpair, AddressFamily, MsgFlags, SockFlag, SockType};
    use std::fs::{remove_file, File};
    use std::io::Write;
    use std::os::unix::io::AsRawFd;

    #[test]
    fn test_next_datagram_size_fd() {
        let buf: Vec<u8> = vec![0, 1, 2, 3, 4];

        let mut not_socket_file = File::create("/tmp/test_next_datagram_size_fd").unwrap();
        not_socket_file.write_all(&buf).unwrap();
        assert!(next_datagram_size_fd(not_socket_file.as_raw_fd()).is_err());
        remove_file("/tmp/test_next_datagram_size_fd").unwrap();

        let (fd1, fd2) = socketpair(
            AddressFamily::Unix,
            SockType::Stream,
            None,
            SockFlag::SOCK_CLOEXEC | SockFlag::SOCK_NONBLOCK,
        )
        .unwrap();
        send(fd1, &buf, MsgFlags::empty()).unwrap();
        assert_eq!(next_datagram_size_fd(fd2).unwrap(), buf.len());
    }
}
