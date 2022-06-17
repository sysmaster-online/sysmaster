use std::cell::RefCell;

use crate::{socket_base::PortType, socket_port::SocketPort};
use nix::libc::mode_t;
use proc_macro_utils::ConfigParseM;
use process1::manager::UnitRef;
use serde::{Deserialize, Serialize};
use std::io::{Error as IoError, ErrorKind};
use utils::config_parser::{toml_str_parse, ConfigParse};

pub(super) enum ListeningItem {
    Stream,
    Datagram,
    Netlink,
}

#[derive(Serialize, Deserialize, ConfigParseM)]
#[serdeName("Socket")]
#[serde(rename_all = "PascalCase")]
pub(super) struct SocketConf {
    #[serde(alias = "ListenStream")]
    listen_stream: String,
    #[serde(alias = "ListenDataGram")]
    listen_datagram: String,
    #[serde(alias = "ListenNetlink")]
    listen_netlink: String,
    #[serde(alias = "PassPacketInfo")]
    pass_pktinfo: String,
    #[serde(alias = "Accept")]
    accept: String,
    #[serde(alias = "Service")]
    service: String,
    #[serde(alias = "ReceiveBuffer")]
    receive_buffer: String,
    #[serde(alias = "ReceiveBuffer")]
    send_buffer: String,
    #[serde(alias = "PassCredentials")]
    pass_cred: String,
    #[serde(alias = "Symlinks")]
    symlinks: Vec<String>,
    #[serde(alias = "PassSecurity")]
    pass_sec: String,
    #[serde(alias = "SocketMode")]
    socket_mode: String,
}

pub(super) enum SocketConfigItem {
    ScPassPktinfo(bool),
    ScAccept(bool),
    ScCred(bool),
    ScSecurity(bool),
    ScSocketMode(u32),
    ScReceiveBuffer(u64),
    ScSendBuffer(u64),
}

pub(super) struct SocketConfig {
    data: RefCell<SocketConfigData>,
}

impl SocketConfig {
    pub(super) fn new() -> Self {
        SocketConfig {
            data: RefCell::new(SocketConfigData::new()),
        }
    }

    pub(super) fn set(&self, item: SocketConfigItem) {
        self.data.borrow_mut().set(item);
    }

    #[allow(dead_code)]
    pub(super) fn get(&self, item: &SocketConfigItem) -> SocketConfigItem {
        self.data.borrow().get(item)
    }

    pub(super) fn push_port(&self, port: SocketPort) {
        self.data.borrow_mut().push_port(port);
    }

    pub(super) fn clear_ports(&self) {
        self.data.borrow_mut().clear_ports();
    }

    pub(super) fn set_ref(&self, source: String, target: String) {
        self.data.borrow_mut().set_ref(source, target);
    }

    pub(super) fn no_accept_socket(&self) -> bool {
        self.data.borrow_mut().no_accept_socket()
    }

    pub(super) fn unit_ref_target(&self) -> Option<String> {
        self.data.borrow_mut().unit_ref_target()
    }
}

struct SocketConfigData {
    ports: Vec<SocketPort>,
    pass_pktinfo: bool,
    accept: bool,
    cred: bool,
    pass_sec: bool,
    socket_mode: mode_t,
    service: RefCell<UnitRef>,
    receive_buffer: u64,
    send_buffer: u64,
}

impl SocketConfigData {
    fn new() -> SocketConfigData {
        SocketConfigData {
            ports: Vec::new(),
            pass_pktinfo: false,
            accept: false,
            service: RefCell::new(UnitRef::new()),
            cred: false,
            pass_sec: false,
            socket_mode: 0o666,
            receive_buffer: 0,
            send_buffer: 0,
        }
    }

    pub(super) fn set(&mut self, item: SocketConfigItem) {
        match item {
            SocketConfigItem::ScPassPktinfo(v) => self.pass_pktinfo = v,
            SocketConfigItem::ScAccept(accept) => self.accept = accept,
            SocketConfigItem::ScCred(cred) => self.cred = cred,
            SocketConfigItem::ScSecurity(sec) => self.pass_sec = sec,
            SocketConfigItem::ScSocketMode(mode) => self.socket_mode = mode,
            SocketConfigItem::ScReceiveBuffer(buffer) => self.receive_buffer = buffer,
            SocketConfigItem::ScSendBuffer(buffer) => self.send_buffer = buffer,
        }
    }

    pub(self) fn get(&self, item: &SocketConfigItem) -> SocketConfigItem {
        match item {
            SocketConfigItem::ScPassPktinfo(_) => {
                SocketConfigItem::ScPassPktinfo(self.pass_pktinfo)
            }
            SocketConfigItem::ScAccept(_) => SocketConfigItem::ScAccept(self.accept),
            SocketConfigItem::ScCred(_) => SocketConfigItem::ScCred(self.cred),
            SocketConfigItem::ScSecurity(_) => SocketConfigItem::ScSecurity(self.pass_sec),
            SocketConfigItem::ScSocketMode(_) => SocketConfigItem::ScSocketMode(self.socket_mode),
            SocketConfigItem::ScReceiveBuffer(_) => {
                SocketConfigItem::ScReceiveBuffer(self.receive_buffer)
            }
            SocketConfigItem::ScSendBuffer(_) => SocketConfigItem::ScSendBuffer(self.send_buffer),
        }
    }

    pub(super) fn push_port(&mut self, port: SocketPort) {
        self.ports.push(port)
    }

    pub(super) fn clear_ports(&mut self) {
        self.ports.clear();
    }

    pub(super) fn set_ref(&mut self, source: String, target: String) {
        self.service.borrow_mut().set_ref(source, target);
    }

    pub(super) fn no_accept_socket(&self) -> bool {
        if !self.accept {
            return true;
        }

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

    pub(super) fn unit_ref_target(&mut self) -> Option<String> {
        self.service
            .borrow()
            .target()
            .map_or(None, |v| Some(v.to_string()))
    }
}
