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

//! socket_port implement the management of configured ports, create , open and close the socket。
//!

use crate::{
    comm::SocketUnitComm,
    config::{SocketConfig, SocketPortConf},
    rentry::PortType,
};
use basic::{fd_util, fs_util, io_util, socket_util};
use nix::{
    errno::Errno,
    poll::PollFlags,
    sys::socket::{
        self,
        sockopt::{self},
        AddressFamily, SockFlag,
    },
};
use std::{cell::RefCell, fmt, os::unix::prelude::RawFd, rc::Rc};
use sysmaster::error::*;

pub(super) const SOCKET_INVALID_FD: RawFd = -1;

pub(crate) struct SocketPort {
    // associated objects
    comm: Rc<SocketUnitComm>,
    config: Rc<SocketConfig>,
    p_conf: Rc<SocketPortConf>,

    // owned objects
    fd: RefCell<RawFd>,
}

impl SocketPort {
    pub(super) fn new(
        commr: &Rc<SocketUnitComm>,
        configr: &Rc<SocketConfig>,
        p_confr: &Rc<SocketPortConf>,
    ) -> Self {
        SocketPort {
            comm: Rc::clone(commr),
            config: Rc::clone(configr),
            p_conf: Rc::clone(p_confr),

            fd: RefCell::new(SOCKET_INVALID_FD),
        }
    }

    pub(super) fn set_fd(&self, fd: RawFd) {
        *self.fd.borrow_mut() = fd;
    }

    pub(super) fn accept(&self) -> Result<i32> {
        socket::accept4(self.fd(), SockFlag::SOCK_NONBLOCK | SockFlag::SOCK_CLOEXEC)
            .context(NixSnafu)
    }

    // process reentrant
    pub(super) fn open_port(&self, update: bool) -> Result<()> {
        // process reentrant protection
        if self.fd() >= 0 {
            // debug: process reentrant
            return Ok(());
        }

        let fd = match self.p_conf.p_type() {
            PortType::Socket => {
                let flag = SockFlag::SOCK_CLOEXEC | SockFlag::SOCK_NONBLOCK;
                let fd = match self.p_conf.socket_listen(flag, 128) {
                    Err(e) => {
                        log::error!("Failed to listen {}: {e}", self.p_conf.listen());
                        return Err(Error::Nix { source: e });
                    }
                    Ok(v) => v,
                };
                self.apply_symlink();
                fd
            }
            PortType::Fifo => {
                let fd = match self.p_conf.open_fifo() {
                    Err(e) => {
                        log::error!("Failed to open FIFO file {}: {e}", self.p_conf.listen());
                        return Err(Error::Nix { source: e });
                    }
                    Ok(v) => v,
                };
                self.apply_symlink();
                fd
            }
            PortType::Special => match self.p_conf.open_special() {
                Err(e) => {
                    log::error!("Failed to open special file {}: {e}", self.p_conf.listen());
                    return Err(Error::Nix { source: e });
                }
                Ok(v) => v,
            },
            PortType::Invalid => todo!(),
        };

        if update {
            if let Err(e) = self.comm.reli().fd_cloexec(fd, false) {
                self.close(update);
                return Err(e);
            }
        }
        self.set_fd(fd);

        Ok(())
    }

    pub(super) fn close(&self, update: bool) {
        let fd = self.fd();
        if fd < 0 {
            // debug
            return;
        }

        if update {
            let ret = self.comm.reli().fd_cloexec(fd, true);
            if ret.is_err() {
                log::error!("close socket, remark fd[{}] failed, ret: {:?}", fd, ret);
            }
        }

        fd_util::close(fd);
        self.set_fd(SOCKET_INVALID_FD);
    }

    pub(super) fn unlink(&self) {
        match self.p_conf.p_type() {
            PortType::Socket => self.p_conf.unlink_socket(),
            PortType::Fifo => self.p_conf.unlink_fifo(),
            PortType::Special => self.p_conf.unlink_special(),
            PortType::Invalid => todo!(),
        }
    }

    pub(super) fn flush_accept(&self) {
        if let Ok(true) = socket::getsockopt(self.fd(), sockopt::AcceptConn) {
            for _i in 1..1024 {
                while let Err(e) = io_util::wait_for_events(self.fd(), PollFlags::POLLIN, 0) {
                    if let basic::Error::Nix {
                        source: Errno::EINTR,
                    } = e
                    {
                        continue;
                    }
                    return;
                }
                match socket::accept4(self.fd(), SockFlag::SOCK_NONBLOCK | SockFlag::SOCK_CLOEXEC)
                    .map(|_| fd_util::close(self.fd()))
                {
                    Ok(_) => {}
                    Err(_e) => {
                        // todo!(): if e == Errno::EAGAIN { return; }
                        return;
                    }
                }
            }
        }
    }

    pub(super) fn flush_fd(&self) {
        loop {
            let v = io_util::wait_for_events(self.fd(), PollFlags::POLLIN, 0).unwrap_or(0);
            if v == 0 {
                return;
            };

            let mut buf = [0; 2048];
            // Use unwrap_or_else to handle errors
            let v = nix::unistd::read(self.fd(), &mut buf)
                .unwrap_or_else(|e| usize::from(e == Errno::EINTR));
            if v == 0 {
                return;
            }
        }
    }

    pub(super) fn apply_sock_opt(&self, fd: RawFd) {
        if let Some(v) = self.config.config_data().borrow().Socket.PassPacketInfo {
            if let Err(e) = socket_util::set_pkginfo(fd, self.family(), v) {
                log::warn!("set socket pkginfo errno: {}", e);
            }
        }

        if let Some(v) = self.config.config_data().borrow().Socket.PassCredentials {
            if let Err(e) = socket_util::set_pass_cred(fd, v) {
                log::warn!("set socket pass cred errno: {}", e);
            }
        }

        if let Some(v) = self.config.config_data().borrow().Socket.ReceiveBuffer {
            if let Err(e) = socket_util::set_receive_buffer(fd, v as usize) {
                log::warn!("set socket receive buffer errno: {}", e);
            }
        }

        if let Some(v) = self.config.config_data().borrow().Socket.SendBuffer {
            if let Err(e) = socket_util::set_send_buffer(fd, v as usize) {
                log::warn!("set socket send buffer errno: {}", e);
            }
        }

        if let Some(v) = self.config.config_data().borrow().Socket.KeepAlive {
            if let Err(e) = socket_util::set_keepalive_state(fd, v) {
                log::warn!("set keepalive state errno: {}", e);
            }
        }

        if let Some(v) = self.config.config_data().borrow().Socket.KeepAliveTimeSec {
            if let Err(e) = socket_util::set_keepalive_timesec(fd, v) {
                log::warn!("set keepalive time errno: {}", e);
            }
        }

        if let Some(v) = self
            .config
            .config_data()
            .borrow()
            .Socket
            .KeepAliveIntervalSec
        {
            if let Err(e) = socket_util::set_keepalive_intervalsec(fd, v) {
                log::warn!("set keepalive interval errno: {}", e);
            }
        }

        if let Some(v) = self.config.config_data().borrow().Socket.KeepAliveProbes {
            if let Err(e) = socket_util::set_keepalive_probes(fd, v) {
                log::warn!("set keepalive probe count errno: {}", e);
            }
        }

        if let Some(v) = self.config.config_data().borrow().Socket.Broadcast {
            if let Err(e) = socket_util::set_broadcast_state(fd, v) {
                log::warn!("set broadcast state errno: {}", e);
            }
        }
    }

    pub(super) fn fd(&self) -> RawFd {
        *self.fd.borrow()
    }

    pub(super) fn p_type(&self) -> PortType {
        self.p_conf.p_type()
    }

    pub(super) fn listen(&self) -> &str {
        self.p_conf.listen()
    }

    pub(super) fn can_accept(&self) -> bool {
        self.p_conf.can_accept()
    }

    pub(super) fn apply_symlink(&self) {
        if !self.p_conf.can_be_symlinked() {
            return;
        }
        let config = self.config.config_data();
        if config.borrow().Socket.Symlinks.is_none() {
            return;
        }

        let target = self.listen();
        for symlink in config.borrow().Socket.Symlinks.as_ref().unwrap() {
            if let Err(e) = fs_util::symlink(target, symlink, false) {
                let unit_name = match self.comm.owner() {
                    None => "null".to_string(),
                    Some(v) => v.id().to_string(),
                };
                log::error!(
                    "Failed to apply Symlinks for {}: {:?}, skipping.",
                    unit_name,
                    e
                );
            }
        }
    }

    fn family(&self) -> AddressFamily {
        self.p_conf.sa().family()
    }
}

impl fmt::Display for SocketPort {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "port type: {:?}, socket address: {}",
            self.p_conf.p_type(),
            self.p_conf.listen()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::{SocketPort, SOCKET_INVALID_FD};
    use crate::base::NetlinkProtocol;
    use crate::comm::SocketUnitComm;
    use crate::config::{SocketAddress, SocketConfig, SocketPortConf};
    use libtests::get_project_root;
    use nix::sys::socket::{
        AddressFamily, NetlinkAddr, SockProtocol, SockType, SockaddrIn, UnixAddr,
    };
    use std::path::PathBuf;
    use std::{
        net::{Ipv4Addr, SocketAddrV4},
        rc::Rc,
    };

    #[test]
    fn test_socket_addr_v4() {
        let comm = Rc::new(SocketUnitComm::new());
        let config = Rc::new(SocketConfig::new(&comm));
        let sock_addr = SockaddrIn::from(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 31457));
        let socket_addr = SocketAddress::new(Box::new(sock_addr), SockType::Stream, None);
        let p_conf = Rc::new(SocketPortConf::new(
            PortType::Socket,
            socket_addr,
            "0.0.0.0:31457",
        ));

        let p = SocketPort::new(&comm, &config, &p_conf);
        let port = Rc::new(p);

        assert_eq!(port.fd(), SOCKET_INVALID_FD);

        let ret = port.open_port(false);
        assert!(ret.is_ok());

        assert_ne!(port.fd(), SOCKET_INVALID_FD);
        assert_eq!(port.family(), AddressFamily::Inet);

        port.flush_accept();
        port.flush_fd();
        port.close(false);
    }

    #[test]
    fn test_socket_unix_addr() {
        let comm = Rc::new(SocketUnitComm::new());
        let config = Rc::new(SocketConfig::new(&comm));
        let unix_path = PathBuf::from("/tmp/test.socket");
        let unix_addr = UnixAddr::new(&unix_path).unwrap();

        let socket_addr = SocketAddress::new(Box::new(unix_addr), SockType::Stream, None);
        assert_eq!(socket_addr.path().unwrap(), unix_path);
        let p_conf = Rc::new(SocketPortConf::new(
            PortType::Socket,
            socket_addr,
            "/tmp/test.socket",
        ));

        let p = SocketPort::new(&comm, &config, &p_conf);
        let port = Rc::new(p);

        assert_eq!(port.fd(), SOCKET_INVALID_FD);

        let ret = port.open_port(false);
        assert!(ret.is_ok());

        assert_ne!(port.fd(), SOCKET_INVALID_FD);
        assert_eq!(port.family(), AddressFamily::Unix);

        port.flush_accept();
        port.flush_fd();
        port.close(false);
    }

    #[test]
    fn test_socket_netlink() {
        let comm = Rc::new(SocketUnitComm::new());
        let config = Rc::new(SocketConfig::new(&comm));

        let family = NetlinkProtocol::from("route".to_string());
        assert_ne!(family, NetlinkProtocol::NetlinkInvalid);

        let group = 0;
        let net_link = NetlinkAddr::new(0, group);

        let socket_addr = SocketAddress::new(
            Box::new(net_link),
            SockType::Raw,
            Some(SockProtocol::from(family)),
        );
        let p_conf = Rc::new(SocketPortConf::new(
            PortType::Socket,
            socket_addr,
            "route 0",
        ));

        let p = SocketPort::new(&comm, &config, &p_conf);
        let port = Rc::new(p);
        assert_eq!(port.fd(), SOCKET_INVALID_FD);

        let ret = port.open_port(false);
        assert!(ret.is_ok());

        assert_ne!(port.fd(), SOCKET_INVALID_FD);
        assert_eq!(port.family(), AddressFamily::Netlink);

        port.flush_accept();
        port.flush_fd();
        port.close(false);
    }

    #[test]
    fn test_apply_sock_opt() {
        let recv_buff_size = 4096;
        let send_buff_size = 4096;
        let mut file_path = get_project_root().unwrap();
        file_path.push("tests/test_units/test.socket.toml");
        let paths = vec![file_path];
        let comm = Rc::new(SocketUnitComm::new());
        let config = Rc::new(SocketConfig::new(&comm));
        let result = config.load(paths, false);

        // Check fileload result
        assert!(result.is_ok());

        let sock_addr = SockaddrIn::from(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), 31457));
        let socket_addr = SocketAddress::new(Box::new(sock_addr), SockType::Stream, None);
        let p_conf = Rc::new(SocketPortConf::new(
            PortType::Socket,
            socket_addr,
            "0.0.0.0:31457",
        ));

        let p = SocketPort::new(&comm, &config, &p_conf);
        let port = Rc::new(p);

        let ret = port.open_port(false);
        assert!(ret.is_ok());
        assert_ne!(port.fd(), SOCKET_INVALID_FD);
        assert_eq!(port.family(), AddressFamily::Inet);

        port.apply_sock_opt(port.fd());

        let pass_packet_info = socket::getsockopt(port.fd(), sockopt::Ipv4PacketInfo);
        match pass_packet_info {
            Ok(v) => assert!(!v),
            Err(e) => println!("Error get PassPacketInfo: {:?}", e),
        }

        let passcredentials_state = socket::getsockopt(port.fd(), sockopt::PassCred);
        match passcredentials_state {
            Ok(v) => assert!(v),
            Err(e) => println!("Error get PassCredentials: {:?}", e),
        }

        /*
           Notice:
               The kernel doubles this value (to allow space for bookkeeping
               overhead) when it is set using setsockopt, and this doubled
               value is returned by getsockopt.(Reference: https://man7.org/linux/man-pages/man7/socket.7.html)
           So we also need to double it in our testcases.
        */
        let recv_buff = socket::getsockopt(port.fd(), sockopt::RcvBuf);
        match recv_buff {
            Ok(v) => assert_eq!(v, recv_buff_size * 2),
            Err(e) => println!("Error get ReceiveBuffer: {:?}", e),
        }

        let send_buff = socket::getsockopt(port.fd(), sockopt::SndBuf);
        match send_buff {
            Ok(v) => assert_eq!(v, send_buff_size * 2),
            Err(e) => println!("Error get SendBuffer: {:?}", e),
        }

        let keepalive_state = socket::getsockopt(port.fd(), sockopt::KeepAlive);
        match keepalive_state {
            Ok(v) => assert!(v),
            Err(e) => println!("Error get keepalive state: {:?}", e),
        }
        let keepalive_timesec = socket::getsockopt(port.fd(), sockopt::TcpKeepIdle);
        match keepalive_timesec {
            Ok(v) => assert_eq!(v, 7000),
            Err(e) => println!("Error get keepalive timeseconds: {:?}", e),
        }
        let keepalive_intervalsec = socket::getsockopt(port.fd(), sockopt::TcpKeepInterval);
        match keepalive_intervalsec {
            Ok(v) => assert_eq!(v, 70),
            Err(e) => println!("Error get keepalive interval: {:?}", e),
        }
        let keepalive_probes = socket::getsockopt(port.fd(), sockopt::TcpKeepCount);
        match keepalive_probes {
            Ok(v) => assert_eq!(v, 10),
            Err(e) => println!("Error get keepalive probes: {:?}", e),
        }
        let broadcast_state = socket::getsockopt(port.fd(), sockopt::Broadcast);
        match broadcast_state {
            Ok(v) => assert!(v),
            Err(e) => println!("Error get broadcast state: {:?}", e),
        }

        // Rosource reclaim
        port.flush_accept();
        port.flush_fd();
        port.close(false);
    }
}
