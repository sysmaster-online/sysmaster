//! socket_config mod load the conf file list and convert it to structure which is defined in this mod.
//!
#![allow(non_snake_case)]
use super::socket_comm::SocketUnitComm;
use super::socket_rentry::{SectionSocket, SocketCommand};
use crate::socket_base::{NetlinkProtocol, PortType};
use confique::Config;
use libsysmaster::manager::{ExecCommand, UnitRef};
use libsysmaster::ReStation;
use libutils::socket_util;
use nix::errno::Errno;
use nix::sys::socket::sockopt::ReuseAddr;
use nix::sys::socket::{
    self, AddressFamily, NetlinkAddr, SockFlag, SockProtocol, SockType, SockaddrIn, SockaddrIn6,
    SockaddrLike, UnixAddr,
};
use std::cell::RefCell;
use std::error::Error;
use std::fmt;
use std::fs;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use std::path::PathBuf;
use std::rc::Rc;

pub(super) enum ListeningItem {
    Stream,
    Datagram,
    Netlink,
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
}

impl ReStation for SocketConfig {
    // no input, no compensate

    // data
    fn db_map(&self) {
        if let Some((data, service)) = self.comm.rentry_conf_get() {
            // SocketConfigData
            self.data.replace(SocketConfigData::new(data));

            // UnitRef
            if let Some(svc) = service {
                self.set_unit_ref(svc).unwrap();
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
        }
    }

    pub(super) fn reset(&self) {
        self.data.replace(SocketConfigData::default());
        self.service.replace(UnitRef::new());
        self.ports.replace(Vec::new());
        self.db_update();
    }

    pub(super) fn load(&self, paths: Vec<PathBuf>, update: bool) -> Result<(), Box<dyn Error>> {
        // get original configuration
        let mut builder = SocketConfigData::builder().env();
        for v in paths {
            builder = builder.file(&v);
        }
        let data = builder.load()?;

        // record original configuration
        *self.data.borrow_mut() = data;

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

    pub(super) fn get_exec_cmds(&self, cmd_type: SocketCommand) -> Option<Vec<ExecCommand>> {
        self.data.borrow().get_exec_cmds(cmd_type)
    }

    pub(super) fn set_unit_ref(&self, service: String) -> Result<(), Box<dyn Error>> {
        if !self.comm.um().load_unit_success(&service) {
            return Err(format!("failed to load unit {}", service).into());
        }

        self.set_ref(service);
        self.db_update();

        Ok(())
    }

    pub(super) fn unit_ref_target(&self) -> Option<String> {
        self.service.borrow().target().map(|v| v.to_string())
    }

    pub(super) fn ports(&self) -> Vec<Rc<SocketPortConf>> {
        self.ports.borrow().iter().cloned().collect::<_>()
    }

    fn parse_service(&self) -> Result<(), Box<dyn Error>> {
        if let Some(service) = self.config_data().borrow().Socket.Service.clone() {
            if !service.ends_with(".service") {
                return Err("socket service must be end with .service"
                    .to_string()
                    .into());
            }

            self.set_unit_ref(service)?;
        }

        Ok(())
    }

    fn parse_port(&self) -> Result<(), Box<dyn Error>> {
        log::debug!("begin to parse socket section");

        let config = &self.data;
        self.parse_listen_socket(ListeningItem::Stream, config.clone())?;
        self.parse_listen_socket(ListeningItem::Datagram, config.clone())?;
        self.parse_listen_socket(ListeningItem::Netlink, config.clone())?;

        Ok(())
    }

    fn parse_listen_socket(
        &self,
        item: ListeningItem,
        socket_conf: Rc<RefCell<SocketConfigData>>,
    ) -> Result<(), Box<dyn Error>> {
        // let sock_addr
        match item {
            ListeningItem::Stream => {
                if let Some(listen_stream) = socket_conf.borrow().listen_stream() {
                    for v in &listen_stream {
                        if v.is_empty() {
                            continue;
                        }
                        if let Ok(socket_addr) = parse_socket_address(v, SockType::Stream) {
                            let port = SocketPortConf::new(PortType::Socket, socket_addr);
                            self.push_port(Rc::new(port));
                        } else {
                            log::error!("create stream listening socket failed: {}", v);
                            return Err(
                                format!("create stream listening socket failed: {}", v).into()
                            );
                        }
                    }
                };
            }
            ListeningItem::Datagram => {
                if let Some(listen_datagram) = socket_conf.borrow().listen_datagram() {
                    for v in &listen_datagram {
                        if v.is_empty() {
                            continue;
                        }
                        if let Ok(socket_addr) = parse_socket_address(v, SockType::Datagram) {
                            let port = SocketPortConf::new(PortType::Socket, socket_addr);
                            self.push_port(Rc::new(port));
                        } else {
                            log::error!("create datagram listening socket failed: {}", v);
                            return Err(
                                format!("create stream datagram socket failed: {}", v).into()
                            );
                        }
                    }
                }
            }
            ListeningItem::Netlink => {
                if let Some(listen_netlink) = socket_conf.borrow().listen_netlink() {
                    for v in &listen_netlink {
                        if v.is_empty() {
                            continue;
                        }

                        if let Err(e) = parse_netlink_address(v) {
                            log::error!("create netlink listening socket: {}, failed: {:?}", v, e);
                            return Err(
                                format!("create netlink listening socket failed: {}", v).into()
                            );
                        }

                        let socket_addr = parse_netlink_address(v).unwrap();
                        let port = SocketPortConf::new(PortType::Socket, socket_addr);
                        self.push_port(Rc::new(port));
                    }
                }
            }
        }

        Ok(())
    }

    fn set_ref(&self, target: String) {
        let source = self.comm.unit().id().to_string();
        self.service.borrow_mut().set_ref(source, target);
    }

    fn push_port(&self, port: Rc<SocketPortConf>) {
        self.ports.borrow_mut().push(port);
    }
}

#[derive(Config, Default, Debug)]
pub(crate) struct SocketConfigData {
    #[config(nested)]
    pub Socket: SectionSocket,
}

impl SocketConfigData {
    pub(self) fn new(Socket: SectionSocket) -> SocketConfigData {
        SocketConfigData { Socket }
    }

    pub(self) fn get_exec_cmds(&self, cmd_type: SocketCommand) -> Option<Vec<ExecCommand>> {
        match cmd_type {
            SocketCommand::StartPre => self.Socket.ExecStartPre.clone(),
            SocketCommand::StartPost => self.Socket.ExecStartPost.clone(),
            SocketCommand::StopPre => self.Socket.ExecStopPre.clone(),
            SocketCommand::StopPost => self.Socket.ExecStopPost.clone(),
        }
    }

    pub(self) fn listen_stream(&self) -> Option<Vec<String>> {
        self.Socket
            .ListenStream
            .as_ref()
            .map(|v| v.iter().map(|v| v.to_string()).collect())
    }

    pub(self) fn listen_datagram(&self) -> Option<Vec<String>> {
        self.Socket
            .ListenDatagram
            .as_ref()
            .map(|v| v.iter().map(|v| v.to_string()).collect())
    }

    pub(self) fn listen_netlink(&self) -> Option<Vec<String>> {
        self.Socket
            .ListenNetlink
            .as_ref()
            .map(|v| v.iter().map(|v| v.to_string()).collect())
    }
}

pub(super) struct SocketPortConf {
    p_type: PortType,
    sa: SocketAddress,
}

impl SocketPortConf {
    pub(super) fn new(p_type: PortType, sa: SocketAddress) -> SocketPortConf {
        SocketPortConf { p_type, sa }
    }

    pub(super) fn p_type(&self) -> PortType {
        self.p_type
    }

    pub(super) fn sa(&self) -> &SocketAddress {
        &self.sa
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

    pub(super) fn can_accept(&self) -> bool {
        if self.sa_type == SockType::Stream {
            return true;
        }

        false
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

    pub(super) fn socket_listen(&self, flags: SockFlag, backlog: usize) -> Result<i32, Errno> {
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
            if let Err(Errno::EADDRINUSE) = socket::bind(fd, &*self.sock_addr) {
                self.unlink();
                socket::bind(fd, &*self.sock_addr)?;
            }
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

fn parse_netlink_address(item: &str) -> Result<SocketAddress, Box<dyn Error>> {
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
        SockType::Raw,
        Some(SockProtocol::from(family)),
    ))
}

fn parse_socket_address(
    item: &str,
    socket_type: SockType,
) -> Result<SocketAddress, Box<dyn Error>> {
    if item.starts_with('/') {
        let unix_addr = UnixAddr::new(&PathBuf::from(item))?;
        return Ok(SocketAddress::new(Box::new(unix_addr), socket_type, None));
    }

    if item.starts_with('@') {
        let unix_addr = UnixAddr::new_abstract(item.as_bytes())?;

        return Ok(SocketAddress::new(Box::new(unix_addr), socket_type, None));
    }

    if let Ok(port) = item.parse::<u16>() {
        if port == 0 {
            return Err("invalid port number".to_string().into());
        }

        if socket_util::ipv6_is_supported() {
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
    use crate::socket_comm::SocketUnitComm;
    use crate::socket_config::SocketConfig;
    use std::rc::Rc;
    use tests::get_project_root;

    #[test]
    fn test_socket_parse() {
        let mut file_path = get_project_root().unwrap();
        file_path.push("test_units/test.socket.toml");
        let paths = vec![file_path];

        let comm = Rc::new(SocketUnitComm::new());
        let config = SocketConfig::new(&comm);
        let result = config.load(paths, false);

        assert!(result.is_ok());
    }
}
