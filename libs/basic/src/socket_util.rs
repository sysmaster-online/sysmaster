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
use nix::{
    errno::Errno,
    sys::socket::{self, sockopt, AddressFamily},
};
use std::{os::unix::prelude::RawFd, path::Path};

///
pub fn ipv6_is_supported() -> bool {
    let inet6 = Path::new("/proc/net/if_inet6");

    if inet6.exists() {
        return true;
    }

    false
}

///
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
pub fn set_pass_cred(fd: RawFd, v: bool) -> Result<()> {
    socket::setsockopt(fd, sockopt::PassCred, &v).context(NixSnafu)
}

///
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
