//! socket_config模块socket类型配置文件的定义，以及保存配置文件解析之后的内容
//!

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::socket_base::SocketCommand;
use proc_macro_utils::ConfigParseM;
use process1::manager::{ExecCommand, UnitRef};
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
    #[serde(alias = "ExecStartPre")]
    exec_start_pre: Option<Vec<String>>,
    #[serde(alias = "ExecStartChown")]
    exec_start_chown: Option<Vec<String>>,
    #[serde(alias = "ExecStartPost")]
    exec_start_post: Option<Vec<String>>,
    #[serde(alias = "ExecStopPre")]
    exec_stop_pre: Option<Vec<String>>,
    #[serde(alias = "ExecStopPost")]
    exec_stop_post: Option<Vec<String>>,
    #[serde(alias = "ListenStream")]
    listen_stream: Option<String>,
    #[serde(alias = "ListenDatagram")]
    listen_datagram: Option<String>,
    #[serde(alias = "ListenNetlink")]
    listen_netlink: Option<String>,
    #[serde(alias = "PassPacketInfo")]
    pass_pktinfo: Option<String>,
    #[serde(alias = "Accept")]
    accept: Option<String>,
    #[serde(alias = "Service")]
    service: Option<String>,
    #[serde(alias = "ReceiveBuffer")]
    receive_buffer: Option<String>,
    #[serde(alias = "SendBuffer")]
    send_buffer: Option<String>,
    #[serde(alias = "PassCredentials")]
    pass_cred: Option<String>,
    #[serde(alias = "Symlinks")]
    symlinks: Option<Vec<String>>,
    #[serde(alias = "PassSecurity")]
    pass_sec: Option<String>,
    #[serde(alias = "SocketMode")]
    socket_mode: Option<String>,
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

    pub(super) fn set_ref(&self, source: String, target: String) {
        self.data.borrow_mut().set_ref(source, target);
    }

    pub(super) fn unit_ref_target(&self) -> Option<String> {
        self.data.borrow_mut().unit_ref_target()
    }

    pub(super) fn insert_exec_cmds(&self, cmd_type: SocketCommand, cmd_line: Rc<ExecCommand>) {
        self.data.borrow_mut().insert_exec_cmds(cmd_type, cmd_line)
    }

    pub(super) fn exec_cmds(&self, cmd_type: SocketCommand) -> Vec<Rc<ExecCommand>> {
        self.data.borrow().exec_cmds(cmd_type)
    }
}

struct SocketConfigData {
    pass_pktinfo: bool,
    accept: bool,
    cred: bool,
    pass_sec: bool,
    socket_mode: u32,
    service: RefCell<UnitRef>,
    receive_buffer: u64,
    send_buffer: u64,
    exec_commands: HashMap<SocketCommand, Vec<Rc<ExecCommand>>>,
}

impl SocketConfigData {
    fn new() -> SocketConfigData {
        SocketConfigData {
            pass_pktinfo: false,
            accept: false,
            service: RefCell::new(UnitRef::new()),
            cred: false,
            pass_sec: false,
            socket_mode: 0,
            receive_buffer: 0,
            send_buffer: 0,
            exec_commands: HashMap::new(),
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

    pub(super) fn set_ref(&mut self, source: String, target: String) {
        self.service.borrow_mut().set_ref(source, target);
    }

    pub(super) fn unit_ref_target(&mut self) -> Option<String> {
        self.service
            .borrow()
            .target()
            .map_or(None, |v| Some(v.to_string()))
    }

    pub(self) fn insert_exec_cmds(&mut self, cmd_type: SocketCommand, cmd_line: Rc<ExecCommand>) {
        self.get_mut_cmds_pad(cmd_type).push(cmd_line);
    }

    pub(self) fn exec_cmds(&self, cmd_type: SocketCommand) -> Vec<Rc<ExecCommand>> {
        if let Some(cmds) = self.exec_commands.get(&cmd_type) {
            cmds.iter().map(|clr| Rc::clone(clr)).collect::<_>()
        } else {
            Vec::new()
        }
    }

    fn get_mut_cmds_pad(&mut self, cmd_type: SocketCommand) -> &mut Vec<Rc<ExecCommand>> {
        // verify existance
        if let None = self.exec_commands.get(&cmd_type) {
            // nothing exists, pad it.
            self.exec_commands.insert(cmd_type, Vec::new());
        }

        // return the one that must exist
        self.exec_commands
            .get_mut(&cmd_type)
            .expect("something inserted is not found.")
    }
}
