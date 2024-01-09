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

//! socket_config mod load the conf file list and convert it to structure which is defined in this mod.
//!
#![allow(non_snake_case)]
use super::comm::SocketUnitComm;
use super::rentry::{PortType, SectionSocket, SocketCommand};
use crate::base::NetlinkProtocol;
use basic::fd;
use core::error::*;
use core::exec::ExecCommand;
use core::rel::ReStation;
use core::unit::KillContext;
use nix::errno::Errno;
use nix::fcntl::{open, OFlag};
use nix::sys::signal::Signal;
use nix::sys::socket::sockopt::ReuseAddr;
use nix::sys::socket::{
    self, AddressFamily, NetlinkAddr, SockFlag, SockProtocol, SockType, SockaddrIn, SockaddrIn6,
    SockaddrLike, UnixAddr,
};
use nix::sys::stat::{self, fstat};
use nix::unistd::{Gid, Uid};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::fmt;
use std::fs;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use std::path::PathBuf;
use std::rc::Rc;
use std::str::FromStr;
use unit_parser::prelude::UnitConfig;

///
#[derive(Default)]
pub struct UnitRef {
    source: Option<String>,
    target: Option<String>,
}

impl UnitRef {
    ///
    pub fn new() -> Self {
        UnitRef {
            source: None,
            target: None,
        }
    }

    ///
    pub fn set_ref(&mut self, source: String, target: String) {
        self.source = Some(source);
        self.target = Some(target);
    }

    ///
    pub fn target(&self) -> Option<&String> {
        self.target.as_ref()
    }
}

pub struct SocketConfig {
    // associated objects
    comm: Rc<SocketUnitComm>,

    // owned objects
    /* original */
    data: Rc<RefCell<SocketConfigData>>,
    /* processed */
    service: RefCell<UnitRef>,
    ports: RefCell<Vec<Rc<SocketPortConf>>>,

    // resolved from ServiceConfigData
    kill_context: Rc<KillContext>,
}

impl ReStation for SocketConfig {
    // no input, no compensate

    // data
    fn db_map(&self, reload: bool) {
        if reload {
            return;
        }
        if let Some((data, service)) = self.comm.rentry_conf_get() {
            // SocketConfigData
            self.data.replace(SocketConfigData::new(data));

            // UnitRef
            if let Some(svc) = service {
                self.set_unit_ref(svc);
            }

            // SocketPortConf
            self.parse_port().unwrap();
        }
    }

    fn db_insert(&self) {
        self.comm
            .rentry_conf_insert(&self.data.borrow().Socket, self.unit_ref_target());
    }

    // reload: no external connections, no entry
}

impl SocketConfig {
    pub(super) fn new(commr: &Rc<SocketUnitComm>) -> Self {
        SocketConfig {
            comm: Rc::clone(commr),
            data: Rc::new(RefCell::new(SocketConfigData::default())),
            service: RefCell::new(UnitRef::new()),
            ports: RefCell::new(Vec::new()),
            kill_context: Rc::new(KillContext::default()),
        }
    }

    pub(super) fn reset(&self) {
        self.data.replace(SocketConfigData::default());
        self.service.replace(UnitRef::new());
        self.ports.replace(Vec::new());
        self.db_update();
    }

    pub(super) fn load(&self, paths: Vec<PathBuf>, update: bool) -> Result<()> {
        let name = paths[0].file_name().unwrap().to_string_lossy().to_string();
        let data = match SocketConfigData::load_config(paths, &name) {
            Ok(v) => v,
            Err(e) => {
                log::error!("Invalid Configuration: {}", e);
                return Err(Error::ConfigureError {
                    msg: format!("Invalid Configuration: {}", e),
                });
            }
        };

        // record original configuration
        *self.data.borrow_mut() = data;

        self.parse_kill_context()?;

        // parse and record processed configuration
        let ret1 = self.parse_service();
        let ret2 = self.parse_port();
        if ret1.is_err() || ret2.is_err() {
            self.reset(); // fallback
            return ret1.and(ret2);
        }

        if update {
            self.db_update();
        }

        Ok(())
    }

    pub(super) fn config_data(&self) -> Rc<RefCell<SocketConfigData>> {
        self.data.clone()
    }

    pub(super) fn get_exec_cmds(&self, cmd_type: SocketCommand) -> Option<VecDeque<ExecCommand>> {
        self.data.borrow().get_exec_cmds(cmd_type)
    }

    pub(super) fn set_unit_ref(&self, service: String) {
        self.set_ref(service);
        self.db_update();
    }

    pub(super) fn unit_ref_target(&self) -> Option<String> {
        self.service.borrow().target().map(|v| v.to_string())
    }

    pub(super) fn ports(&self) -> Vec<Rc<SocketPortConf>> {
        self.ports.borrow().iter().cloned().collect::<_>()
    }

    fn parse_service(&self) -> Result<()> {
        if let Some(service) = self.config_data().borrow().Socket.Service.clone() {
            if !service.ends_with(".service") {
                log::warn!(
                    "socket service must be end with .service, ignoring:{}",
                    service
                );
                return Ok(());
            }

            self.set_unit_ref(service);
        }

        Ok(())
    }

    fn parse_port(&self) -> Result<()> {
        log::debug!("begin to parse socket section");
        let config = &self.data.borrow().Socket;
        self.parse_sockets(config.ListenStream.as_ref(), ListenItem::Stream)?;
        self.parse_sockets(config.ListenDatagram.as_ref(), ListenItem::Datagram)?;
        self.parse_sockets(config.ListenNetlink.as_ref(), ListenItem::Netlink)?;
        self.parse_sockets(
            config.ListenSequentialPacket.as_ref(),
            ListenItem::SequentialPacket,
        )?;
        self.parse_fifo(config.ListenFIFO.as_ref())?;
        self.parse_special(config.ListenSpecial.as_ref())?;
        Ok(())
    }

    fn parse_sockets(&self, listens: &[String], listen_item: ListenItem) -> Result<()> {
        let socket_type = match listen_item {
            ListenItem::Datagram => SockType::Datagram,
            ListenItem::Stream => SockType::Stream,
            ListenItem::SequentialPacket => SockType::SeqPacket,
            ListenItem::Netlink => SockType::Raw,
        };

        let parse_func = match listen_item {
            ListenItem::Netlink => parse_netlink_address,
            _ => parse_socket_address,
        };

        for v in listens {
            if v.is_empty() {
                continue;
            }

            let socket_addr = match parse_func(v, socket_type) {
                Err(_) => {
                    log::warn!("Invalid socket configuration: {}", v);
                    return Ok(());
                }
                Ok(v) => v,
            };

            let port = SocketPortConf::new(PortType::Socket, socket_addr, v);
            self.push_port(Rc::new(port));
        }

        Ok(())
    }

    fn parse_fifo(&self, listens: &[String]) -> Result<()> {
        for v in listens {
            let port = SocketPortConf::new(PortType::Fifo, SocketAddress::empty(), v);
            self.push_port(Rc::new(port));
        }
        Ok(())
    }

    fn parse_special(&self, listens: &[String]) -> Result<()> {
        for v in listens {
            let port = SocketPortConf::new(PortType::Special, SocketAddress::empty(), v);
            self.push_port(Rc::new(port));
        }
        Ok(())
    }

    fn set_ref(&self, target: String) {
        if let Some(u) = self.comm.owner() {
            self.service.borrow_mut().set_ref(u.id(), target)
        };
    }

    fn push_port(&self, port: Rc<SocketPortConf>) {
        self.ports.borrow_mut().push(port);
    }

    pub(super) fn kill_context(&self) -> Rc<KillContext> {
        self.kill_context.clone()
    }

    fn parse_kill_context(&self) -> Result<()> {
        self.kill_context
            .set_kill_mode(self.config_data().borrow().Socket.KillMode);

        let signal = Signal::from_str(&self.config_data().borrow().Socket.KillSignal)?;
        self.kill_context.set_kill_signal(signal);
        Ok(())
    }

    pub(super) fn set_property(&self, key: &str, value: &str) -> Result<()> {
        let ret = self.data.borrow_mut().set_property(key, value);
        self.db_update();
        ret
    }
}

#[derive(PartialEq)]
enum ListenItem {
    Stream,
    Datagram,
    Netlink,
    SequentialPacket,
}

#[derive(UnitConfig, Default, Debug)]
pub(crate) struct SocketConfigData {
    pub Socket: SectionSocket,
}

impl SocketConfigData {
    pub(self) fn new(Socket: SectionSocket) -> SocketConfigData {
        SocketConfigData { Socket }
    }

    // keep consistency with the configuration, so just copy from configuration.
    pub(self) fn get_exec_cmds(&self, cmd_type: SocketCommand) -> Option<VecDeque<ExecCommand>> {
        let mut res = VecDeque::new();
        for v in match cmd_type {
            SocketCommand::StartPre => self.Socket.ExecStartPre.clone(),
            SocketCommand::StartPost => self.Socket.ExecStartPost.clone(),
            SocketCommand::StopPre => self.Socket.ExecStopPre.clone(),
            SocketCommand::StopPost => self.Socket.ExecStopPost.clone(),
        } {
            res.push_back(v)
        }
        Some(res)
    }

    pub(self) fn set_property(&mut self, key: &str, value: &str) -> Result<()> {
        self.Socket.set_property(key, value)
    }
}

pub(super) struct SocketPortConf {
    p_type: PortType,
    sa: SocketAddress,
    /* raw addr */
    listen: String,
}

impl SocketPortConf {
    pub(super) fn new(p_type: PortType, sa: SocketAddress, listenr: &str) -> SocketPortConf {
        SocketPortConf {
            p_type,
            sa,
            listen: String::from(listenr),
        }
    }

    pub(super) fn p_type(&self) -> PortType {
        self.p_type
    }

    pub(super) fn sa(&self) -> &SocketAddress {
        &self.sa
    }

    pub(super) fn listen(&self) -> &str {
        &self.listen
    }

    pub(super) fn can_accept(&self) -> bool {
        if self.p_type() == PortType::Socket {
            self.sa.can_accept()
        } else {
            false
        }
    }

    pub(super) fn socket_listen(
        &self,
        flags: SockFlag,
        backlog: usize,
        socket_mode: u32,
    ) -> Result<i32, Errno> {
        if self.p_type() == PortType::Socket {
            self.sa.socket_listen(flags, backlog, socket_mode)
        } else {
            Err(Errno::ENOTSUP)
        }
    }

    pub(super) fn unlink_socket(&self) {
        self.sa().unlink()
    }

    pub(super) fn open_fifo(&self, socket_mode: u32) -> Result<i32, Errno> {
        let path = match PathBuf::from_str(self.listen()) {
            Err(_) => return Err(Errno::EINVAL),
            Ok(v) => v,
        };

        let old_mask = stat::umask(stat::Mode::from_bits_truncate(!socket_mode));

        stat::umask(stat::Mode::from_bits_truncate(
            !socket_mode | old_mask.bits(),
        ));
        nix::unistd::mkfifo(&path, stat::Mode::from_bits_truncate(socket_mode))?;
        stat::umask(old_mask);

        let oflag = OFlag::O_RDWR
            | OFlag::O_CLOEXEC
            | OFlag::O_NOCTTY
            | OFlag::O_NONBLOCK
            | OFlag::O_NOFOLLOW;

        let fd = match open(&path, oflag, stat::Mode::from_bits_truncate(socket_mode)) {
            Err(e) => return Err(e),
            Ok(v) => v,
        };
        Ok(fd)
    }

    pub(super) fn unlink_fifo(&self) {
        let path = match PathBuf::from_str(self.listen()) {
            Err(_) => return,
            Ok(v) => v,
        };
        if let Err(e) = nix::unistd::unlink(&path) {
            log::error!("Failed to unlink FIFO {}: {}", self.listen(), e);
        }
    }

    pub(super) fn open_special(&self) -> Result<i32, Errno> {
        let path = match PathBuf::from_str(self.listen()) {
            Err(_) => return Err(Errno::EINVAL),
            Ok(v) => v,
        };
        let oflag = OFlag::O_RDWR
            | OFlag::O_CLOEXEC
            | OFlag::O_NOCTTY
            | OFlag::O_NONBLOCK
            | OFlag::O_NOFOLLOW;
        let fd = match open(&path, oflag, stat::Mode::empty()) {
            Err(e) => return Err(e),
            Ok(v) => v,
        };
        let st = fstat(fd)?;
        if !fd::stat_is_reg(st.st_mode) && !fd::stat_is_char(st.st_mode) {
            return Err(Errno::EEXIST);
        }
        Ok(fd)
    }

    pub(super) fn unlink_special(&self) {
        /* Do noting for ListenSpecial */
    }

    pub(super) fn can_be_symlinked(&self) -> bool {
        if ![PortType::Socket, PortType::Fifo].contains(&self.p_type()) {
            return false;
        }
        if !self.listen().starts_with('/') {
            return false;
        }
        true
    }

    pub(super) fn chown(&self, uid: Uid, gid: Gid) -> Result<()> {
        let path = if self.p_type == PortType::Fifo {
            PathBuf::from(&self.listen)
        } else if let Some(path) = self.sa.path() {
            path
        } else {
            return Ok(());
        };

        nix::unistd::chown(&path, Some(uid), Some(gid))?;

        Ok(())
    }
}

pub(super) struct SocketAddress {
    sock_addr: Box<dyn SockaddrLike>,
    sa_type: SockType,
    protocol: Option<SockProtocol>,
}

impl SocketAddress {
    pub(super) fn new(
        sock_addr: Box<dyn SockaddrLike>,
        sa_type: SockType,
        protocol: Option<SockProtocol>,
    ) -> SocketAddress {
        SocketAddress {
            sock_addr,
            sa_type,
            protocol,
        }
    }

    pub(super) fn empty() -> SocketAddress {
        let unix_addr = UnixAddr::new(&PathBuf::from("/dev/null")).unwrap();
        SocketAddress {
            sock_addr: Box::new(unix_addr),
            sa_type: SockType::Raw,
            protocol: None,
        }
    }

    pub(super) fn can_accept(&self) -> bool {
        matches!(self.sa_type, SockType::SeqPacket | SockType::Stream)
    }

    pub(super) fn path(&self) -> Option<PathBuf> {
        if self.sock_addr.family() != Some(AddressFamily::Unix) {
            return None;
        }

        if let Some(unix_addr) =
            unsafe { UnixAddr::from_raw(self.sock_addr.as_ptr(), Some(self.sock_addr.len())) }
        {
            return unix_addr.path().map(|p| p.to_path_buf());
        }
        None
    }

    pub(super) fn family(&self) -> AddressFamily {
        self.sock_addr.family().unwrap()
    }

    pub(super) fn socket_listen(
        &self,
        flags: SockFlag,
        backlog: usize,
        socket_mode: u32,
    ) -> Result<i32, Errno> {
        log::debug!(
            "create socket, family: {:?}, type: {:?}, protocol: {:?}",
            self.sock_addr.family().unwrap(),
            self.sa_type,
            self.protocol
        );
        let fd = socket::socket(
            self.sock_addr.family().unwrap(),
            self.sa_type,
            flags,
            self.protocol,
        )?;

        socket::setsockopt(fd, ReuseAddr, &true)?;

        if let Some(path) = self.path() {
            let parent_path = path.as_path().parent();
            fs::create_dir_all(parent_path.unwrap()).map_err(|_e| Errno::EINVAL)?;

            let old_mask = stat::umask(stat::Mode::from_bits_truncate(!socket_mode));
            if let Err(Errno::EADDRINUSE) = socket::bind(fd, &*self.sock_addr) {
                self.unlink();
                socket::bind(fd, &*self.sock_addr)?;
            }

            stat::umask(stat::Mode::from_bits_truncate(old_mask.bits() & 0o777));
        } else {
            socket::bind(fd, &*self.sock_addr)?;
        }

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
        log::debug!("unlink socket, just useful in unix mode");
        if let Some(AddressFamily::Unix) = self.sock_addr.family() {
            if let Some(path) = self.path() {
                log::debug!("unlink path: {:?}", path);
                match nix::unistd::unlink(&path) {
                    Ok(_) => {}
                    Err(e) => {
                        log::warn!("Unable to unlink {:?}, error: {}", path, e)
                    }
                }
            }
        }
    }
}

impl fmt::Display for SocketAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "sock type: {:?}, sock family: {:?}",
            self.sa_type,
            self.sock_addr.family().unwrap(),
        )
    }
}

fn parse_netlink_address(item: &str, socket_type: SockType) -> Result<SocketAddress> {
    let words: Vec<String> = item.split_whitespace().map(|s| s.to_string()).collect();
    if words.len() != 2 {
        return Err(format!("Netlink configuration format is not correct: {}", item).into());
    }

    let family = NetlinkProtocol::from(words[0].to_string());
    if family == NetlinkProtocol::NetlinkInvalid {
        return Err("Netlink family is invalid".to_string().into());
    }

    let group = if let Ok(g) = words[1].parse::<u32>() {
        g
    } else {
        return Err("Netlink group is invalid".to_string().into());
    };

    let net_link = NetlinkAddr::new(0, group);

    Ok(SocketAddress::new(
        Box::new(net_link),
        socket_type,
        Some(SockProtocol::from(family)),
    ))
}

fn parse_socket_address(item: &str, socket_type: SockType) -> Result<SocketAddress> {
    if item.starts_with('/') {
        let unix_addr = UnixAddr::new(&PathBuf::from(item)).context(NixSnafu)?;
        return Ok(SocketAddress::new(Box::new(unix_addr), socket_type, None));
    }

    if item.starts_with('@') {
        let address = item.trim_start_matches('@').as_bytes();
        let unix_addr = UnixAddr::new_abstract(address).context(NixSnafu)?;

        return Ok(SocketAddress::new(Box::new(unix_addr), socket_type, None));
    }

    if let Ok(port) = item.parse::<u16>() {
        if port == 0 {
            return Err("invalid port number".to_string().into());
        }

        if basic::socket::ipv6_is_supported() {
            let addr = SockaddrIn6::from(SocketAddrV6::new(
                Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0),
                port,
                0,
                0,
            ));
            return Ok(SocketAddress::new(Box::new(addr), socket_type, None));
        }

        let addr = SockaddrIn::from(SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), port));
        return Ok(SocketAddress::new(Box::new(addr), socket_type, None));
    }

    if let Ok(socket_addr) = item.parse::<SocketAddr>() {
        let sock_addr: Box<dyn SockaddrLike> = match socket_addr {
            SocketAddr::V4(addr) => Box::new(SockaddrIn::from(addr)),
            SocketAddr::V6(addr) => Box::new(SockaddrIn6::from(addr)),
        };

        return Ok(SocketAddress::new(sock_addr, socket_type, None));
    }

    Err("invalid listening config".to_string().into())
}

#[cfg(test)]
mod tests {
    use crate::comm::SocketUnitComm;
    use crate::config::SocketConfig;
    use libtests::get_project_root;
    use std::rc::Rc;

    #[test]
    fn test_socket_parse() {
        let mut file_path = get_project_root().unwrap();
        file_path.push("tests/test_units/uttest.socket");
        let paths = vec![file_path];

        let comm = Rc::new(SocketUnitComm::new());
        let config = SocketConfig::new(&comm);
        let result = config.load(paths, false);

        assert!(result.is_ok());
    }
}
