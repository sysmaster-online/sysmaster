use nix::{libc, sys::socket::SockAddr};
use std::cell::RefCell;

use crate::socket_base::PortType;

#[allow(dead_code)]
pub(super) struct SocketAddress {
    sock_addr: SockAddr,
    sa_type: i32,
    protocol: i32,
}

impl SocketAddress {
    pub(super) fn new(sock_addr: SockAddr, sa_type: i32, protocol: i32) -> SocketAddress {
        SocketAddress {
            sock_addr,
            sa_type,
            protocol,
        }
    }

    pub(super) fn can_accept(&self) -> bool {
        if self.sa_type == libc::SOCK_STREAM {
            return true;
        }

        false
    }
}

#[allow(dead_code)]
pub(super) struct SocketPort {
    p_type: PortType,
    fd: u32,
    sa: RefCell<SocketAddress>,
}

impl SocketPort {
    pub(super) fn new(socket_addr: SocketAddress) -> Self {
        SocketPort {
            p_type: PortType::Invalid,
            fd: 0,
            sa: RefCell::new(socket_addr),
        }
    }

    pub(super) fn set_sc_type(&mut self, p_type: PortType) {
        self.p_type = p_type;
    }

    pub(super) fn p_type(&self) -> PortType {
        self.p_type
    }

    pub(super) fn can_accept(&self) -> bool {
        self.sa.borrow().can_accept()
    }
}
