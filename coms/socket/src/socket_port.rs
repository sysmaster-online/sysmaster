//! socket_port implement the management of configured ports, create , open and close the socket。
//!

use crate::{
    socket_comm::SocketUnitComm,
    socket_config::{SocketAddress, SocketConfig, SocketPortConf},
    socket_rentry::PortType,
};
use libutils::{fd_util, io_util, socket_util};
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

pub(super) const SOCKET_INVALID_FD: RawFd = -1;

pub(super) struct SocketPort {
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

    pub(super) fn accept(&self) -> Result<i32, Errno> {
        match socket::accept4(self.fd(), SockFlag::SOCK_NONBLOCK | SockFlag::SOCK_CLOEXEC) {
            Ok(fd) => Ok(fd),
            Err(e) => Err(e),
        }
    }

    // process reentrant
    pub(super) fn open_port(&self, update: bool) -> Result<(), Errno> {
        // process reentrant protection
        if self.fd() >= 0 {
            // debug: process reentrant
            return Ok(());
        }

        let fd = match self.p_conf.p_type() {
            PortType::Socket => {
                let flag = SockFlag::SOCK_CLOEXEC | SockFlag::SOCK_NONBLOCK;

                self.p_conf.sa().socket_listen(flag, 128)?
            }
            PortType::Fifo => todo!(),
            PortType::Invalid => todo!(),
        };

        if update {
            if let Err(e) = self.comm.reli().fd_cloexec(fd, false) {
                self.close(update);
                return Err(e.into());
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

        match self.p_conf.p_type() {
            PortType::Socket => {
                self.p_conf.sa().unlink();
            }
            PortType::Fifo => todo!(),
            PortType::Invalid => todo!(),
        }

        self.set_fd(SOCKET_INVALID_FD);
    }

    pub(super) fn flush_accept(&self) {
        let accept_conn = socket::getsockopt(self.fd(), sockopt::AcceptConn);
        if accept_conn.is_err() {
            return;
        }

        if !accept_conn.unwrap() {
            return;
        }

        for _i in 1..1024 {
            match io_util::wait_for_events(self.fd(), PollFlags::POLLIN, 0) {
                Ok(v) => {
                    if v == 0 {
                        return;
                    }
                }
                Err(e) => {
                    if e == Errno::EINTR {
                        continue;
                    }
                    return;
                }
            }

            match socket::accept4(self.fd(), SockFlag::SOCK_NONBLOCK | SockFlag::SOCK_CLOEXEC) {
                Ok(_) => {
                    fd_util::close(self.fd());
                }
                Err(e) => {
                    if e == Errno::EAGAIN {
                        return;
                    }

                    // todo!() err is to continue
                    return;
                }
            }
        }
    }

    pub(super) fn flush_fd(&self) {
        loop {
            match io_util::wait_for_events(self.fd(), PollFlags::POLLIN, 0) {
                Ok(v) => {
                    if v == 0 {
                        return;
                    }

                    let mut buf = [0; 2048];
                    match nix::unistd::read(self.fd(), &mut buf) {
                        Ok(v) => {
                            if v == 0 {
                                return;
                            }
                        }
                        Err(e) => {
                            if e == Errno::EINTR {
                                continue;
                            }
                            return;
                        }
                    }
                }
                Err(e) => {
                    if e == Errno::EINTR {
                        continue;
                    }
                    return;
                }
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

    pub(super) fn sa(&self) -> &SocketAddress {
        self.p_conf.sa()
    }

    pub(super) fn listen(&self) -> &str {
        self.p_conf.listen()
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
            self.p_conf.sa()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::{SocketPort, SOCKET_INVALID_FD};
    use crate::socket_base::NetlinkProtocol;
    use crate::socket_comm::SocketUnitComm;
    use crate::socket_config::{SocketAddress, SocketConfig, SocketPortConf};
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
