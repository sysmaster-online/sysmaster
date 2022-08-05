//! socket_port模块实现port的管理。实现socket套接字的创建等管理操作。
//!

use nix::{
    errno::Errno,
    libc::{self},
    poll::PollFlags,
    sys::socket::{
        self,
        sockopt::{self, ReuseAddr},
        AddressFamily, SockAddr, SockFlag, SockProtocol, SockType,
    },
};
use std::{
    cell::RefCell,
    os::unix::prelude::RawFd,
    path::PathBuf,
    rc::{Rc, Weak},
};

use crate::{
    socket_base::PortType,
    socket_config::SocketConfig,
    socket_mng::{SocketMng, SocketState},
};

use event::{EventType, Events, Source};
use utils::{fd_util, io_util, socket_util, Error};

//
pub(super) struct SocketPorts {
    data: RefCell<SocketPortsData>,
}

impl SocketPorts {
    pub(super) fn new() -> Self {
        SocketPorts {
            data: RefCell::new(SocketPortsData::new()),
        }
    }

    pub(super) fn push_port(&self, port: Rc<SocketPort>) {
        self.data.borrow_mut().push_port(port.clone());
    }

    #[allow(dead_code)]
    pub(super) fn clear_ports(&self) {
        self.data.borrow_mut().clear_ports();
    }

    pub(super) fn ports(&self) -> Vec<Rc<SocketPort>> {
        self.data.borrow().ports()
    }

    pub(super) fn no_accept_socket(&self) -> bool {
        self.data.borrow().no_accept_socket()
    }

    pub(super) fn attach(&self, sock_mng: Rc<SocketMng>) {
        self.data.borrow_mut().attach(sock_mng)
    }

    pub(super) fn collect_fds(&self) -> Vec<i32> {
        let mut fds = Vec::new();
        for port in self.ports().iter() {
            if port.fd() >= 0 {
                fds.push(port.fd() as i32);
            }
        }

        fds
    }
}

struct SocketPortsData {
    ports: Vec<Rc<SocketPort>>,
}

impl SocketPortsData {
    pub(self) fn new() -> Self {
        SocketPortsData { ports: Vec::new() }
    }

    pub(super) fn push_port(&mut self, port: Rc<SocketPort>) {
        self.ports.push(port.clone());
    }

    pub(super) fn clear_ports(&mut self) {
        self.ports.clear();
    }

    pub(self) fn ports(&self) -> Vec<Rc<SocketPort>> {
        log::debug!("ports length: {}", self.ports.len());
        self.ports.iter().map(|p| p.clone()).collect::<_>()
    }

    pub(self) fn no_accept_socket(&self) -> bool {
        for port in self.ports.iter() {
            if port.p_type() != PortType::Socket {
                return true;
            }

            if !port.can_accept() {
                return true;
            }
        }

        false
    }

    pub(self) fn attach(&mut self, sock_mng: Rc<SocketMng>) {
        for port in self.ports.iter() {
            port.clone().attach(sock_mng.clone())
        }
    }
}

#[allow(dead_code)]
pub(super) struct SocketAddress {
    sock_addr: SockAddr,
    sa_type: SockType,
    protocol: Option<SockProtocol>,
}

impl SocketAddress {
    pub(super) fn new(
        sock_addr: SockAddr,
        sa_type: SockType,
        protocol: Option<SockProtocol>,
    ) -> SocketAddress {
        SocketAddress {
            sock_addr,
            sa_type,
            protocol,
        }
    }

    pub(super) fn can_accept(&self) -> bool {
        if self.sa_type == SockType::Stream {
            return true;
        }

        false
    }

    pub(super) fn path(&self) -> Option<PathBuf> {
        if self.sock_addr.family() != AddressFamily::Unix {
            return None;
        }

        if let SockAddr::Unix(unix_addr) = self.sock_addr {
            return unix_addr.path().map(|p| p.to_path_buf());
        }
        None
    }

    pub(super) fn family(&self) -> AddressFamily {
        self.sock_addr.family()
    }

    pub(super) fn socket_listen(&self, falgs: SockFlag, backlog: usize) -> Result<i32, Errno> {
        let fd = socket::socket(self.sock_addr.family(), self.sa_type, falgs, self.protocol)?;

        socket::setsockopt(fd, ReuseAddr, &true)?;
        socket::bind(fd, &self.sock_addr)?;

        if self.can_accept() {
            match socket::listen(fd, backlog) {
                Ok(_) => {}
                Err(e) => {
                    return Err(e);
                }
            }
        }

        Ok(fd)
    }

    pub(super) fn unlink(&self) {
        if self.sock_addr.family() == AddressFamily::Unix {
            let path = self.path();
            if path.is_none() {
                return;
            }

            let tmp = path.unwrap();
            match nix::unistd::unlink(&tmp) {
                Ok(_) => {}
                Err(e) => {
                    log::warn!("Unable to unlink {:?}, error: {}", tmp, e)
                }
            }
        }
    }
}

#[allow(dead_code)]
pub(super) struct SocketPort {
    mng: RefCell<Weak<SocketMng>>,
    config: Rc<SocketConfig>,

    p_type: PortType,
    fd: RefCell<isize>,
    sa: RefCell<SocketAddress>,
}

impl SocketPort {
    pub(super) fn new(socket_addr: SocketAddress, configr: Rc<SocketConfig>) -> Self {
        SocketPort {
            p_type: PortType::Invalid,
            fd: RefCell::new(-1),
            sa: RefCell::new(socket_addr),
            mng: RefCell::new(Weak::new()),
            config: configr.clone(),
        }
    }

    pub(super) fn mng(&self) -> Rc<SocketMng> {
        self.mng.borrow().clone().upgrade().unwrap()
    }

    pub(super) fn set_sc_type(&mut self, p_type: PortType) {
        self.p_type = p_type;
    }

    pub(super) fn p_type(&self) -> PortType {
        self.p_type
    }

    pub(super) fn family(&self) -> AddressFamily {
        self.sa.borrow().family()
    }

    pub(super) fn fd(&self) -> RawFd {
        (*self.fd.borrow()).try_into().unwrap()
    }

    pub(super) fn can_accept(&self) -> bool {
        self.sa.borrow().can_accept()
    }

    pub(super) fn accept(&self) -> Result<i32, Errno> {
        match socket::accept4(
            (*self.fd.borrow()).try_into().unwrap(),
            SockFlag::SOCK_NONBLOCK | SockFlag::SOCK_CLOEXEC,
        ) {
            Ok(fd) => {
                return Ok(fd);
            }
            Err(e) => {
                return Err(e);
            }
        }
    }

    pub(super) fn open_port(&self) -> Result<(), Errno> {
        if *self.fd.borrow() >= 0 {
            return Ok(());
        }

        match self.p_type {
            PortType::Socket => {
                *self.fd.borrow_mut() = self
                    .sa
                    .borrow()
                    .socket_listen(SockFlag::SOCK_CLOEXEC | SockFlag::SOCK_NONBLOCK, 128)?
                    .try_into()
                    .unwrap();
            }
            PortType::Fifo => todo!(),
            PortType::Invalid => todo!(),
        }

        Ok(())
    }

    pub(super) fn close(&self) {
        if *self.fd.borrow() < 0 {
            return;
        }

        fd_util::close((*self.fd.borrow()).try_into().unwrap());

        *self.fd.borrow_mut() = -1;
        match self.p_type {
            PortType::Socket => {
                self.sa.borrow().unlink();
            }
            PortType::Fifo => todo!(),
            PortType::Invalid => todo!(),
        }
    }

    pub(super) fn flush_accept(&self) {
        let accept_conn =
            socket::getsockopt((*self.fd.borrow()).try_into().unwrap(), sockopt::AcceptConn);
        if accept_conn.is_err() {
            return;
        }

        if !accept_conn.unwrap() {
            return;
        }

        for _i in 1..1024 {
            match io_util::wait_for_events(
                (*self.fd.borrow()).try_into().unwrap(),
                PollFlags::POLLIN,
                0,
            ) {
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

            match socket::accept4(
                (*self.fd.borrow()).try_into().unwrap(),
                SockFlag::SOCK_NONBLOCK | SockFlag::SOCK_CLOEXEC,
            ) {
                Ok(_) => {
                    fd_util::close((*self.fd.borrow()).try_into().unwrap());
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
            match io_util::wait_for_events(
                (*self.fd.borrow()).try_into().unwrap(),
                PollFlags::POLLIN,
                0,
            ) {
                Ok(v) => {
                    if v == 0 {
                        return;
                    }

                    let mut buf = [0; 2048];
                    match nix::unistd::read((*self.fd.borrow()).try_into().unwrap(), &mut buf) {
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

    pub(self) fn attach(&self, sock_mng: Rc<SocketMng>) {
        *self.mng.borrow_mut() = Rc::downgrade(&sock_mng);
    }

    pub(super) fn apply_sock_opt(&self, fd: RawFd) {
        if let Some(v) = self.config.Socket.PassPacketInfo {
            if let Err(e) = socket_util::set_pkginfo(fd, self.family(), v) {
                log::warn!("set socket pkginfo errno: {}", e);
            }
        }

        if let Some(v) = self.config.Socket.PassCredentials {
            if let Err(e) = socket_util::set_pass_cred(fd, v) {
                log::warn!("set socket pass cred errno: {}", e);
            }
        }

        if let Some(v) = self.config.Socket.ReceiveBuffer {
            if let Err(e) = socket_util::set_receive_buffer(fd, v as usize) {
                log::warn!("set socket receive buffer errno: {}", e);
            }
        }

        if let Some(v) = self.config.Socket.SendBuffer {
            if let Err(e) = socket_util::set_send_buffer(fd, v as usize) {
                log::warn!("set socket send buffer errno: {}", e);
            }
        }
    }
}

impl Source for SocketPort {
    fn fd(&self) -> RawFd {
        self.fd()
    }

    fn event_type(&self) -> EventType {
        EventType::Io
    }

    fn epoll_event(&self) -> u32 {
        (libc::EPOLLIN) as u32
    }

    fn priority(&self) -> i8 {
        0i8
    }

    fn dispatch(&self, _: &Events) -> Result<i32, Error> {
        println!("Dispatching IO!");
        let afd: i32 = -1;

        if self.mng().state() != SocketState::Listening {
            return Ok(0);
        }

        if let Some(accept) = self.config.Socket.Accept {
            if accept && self.p_type() == PortType::Socket && self.can_accept() {
                let afd = self
                    .accept()
                    .map_err(|_e| Error::Other { msg: "accept err" })?;

                self.apply_sock_opt(afd)
            }
        }

        self.mng().enter_runing(afd);

        Ok(0)
    }

    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }
}

#[cfg(test)]
mod tests {
    use nix::sys::socket::{InetAddr, IpAddr, SockAddr, SockType};
    use std::rc::Rc;

    use crate::{socket_base::PortType, socket_config::SocketConfig};

    use super::{SocketAddress, SocketPort, SocketPorts};

    #[test]
    fn test_socket_ports() {
        let ports = SocketPorts::new();
        let config = Rc::new(SocketConfig::default());

        let sockv4 = SockAddr::Inet(InetAddr::new(IpAddr::new_v4(0, 0, 0, 0), 31457));
        let socket_addr = SocketAddress::new(sockv4, SockType::Stream, None);

        let mut p = SocketPort::new(socket_addr, config.clone());
        p.set_sc_type(PortType::Socket);

        let port = Rc::new(p);
        ports.push_port(port.clone());

        assert_eq!(ports.ports().len(), 1);
        assert_eq!(ports.no_accept_socket(), false);
        assert_eq!(ports.collect_fds().len(), 0);

        assert_eq!(port.fd(), -1);

        if let Err(_e) = port.open_port() {
            return;
        }

        assert_ne!(port.fd(), -1);

        port.flush_accept();
        port.flush_fd();
        port.close();
        ports.clear_ports();

        assert_eq!(ports.ports().len(), 0);
    }
}
