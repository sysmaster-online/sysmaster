use nix::libc;
use nix::sys::socket::{InetAddr, IpAddr, SockAddr, UnixAddr};
use process1::manager::{UnitRelations, UnitType};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::{error::Error, rc::Rc};
use utils::{conf_parser, socket_util};

use crate::socket_base::{NetlinkProtocol, PortType};
use crate::socket_comm::SocketComm;
use crate::socket_config::{ListeningItem, SocketConf, SocketConfig, SocketConfigItem};
use crate::socket_port::{SocketAddress, SocketPort};

pub(super) struct SocketLoad {
    config: Rc<SocketConfig>,
    comm: Rc<SocketComm>,
}

impl SocketLoad {
    pub(super) fn new(configr: &Rc<SocketConfig>, commr: &Rc<SocketComm>) -> Self {
        SocketLoad {
            config: configr.clone(),
            comm: commr.clone(),
        }
    }

    fn load_related_unit(&self, related_type: UnitType) -> bool {
        let unit_name = self.comm.unit().get_id().to_string();
        let stem_name = Path::new(&unit_name).file_stem().unwrap().to_str().unwrap();

        let suffix = String::from(related_type);
        if suffix.len() == 0 {
            return false;
        }

        let relate_name = format!("{}.{}", stem_name, suffix);
        if self.comm.um().load_unit_success(&relate_name) {
            return true;
        }

        self.config
            .set_ref(self.comm.unit().get_id().to_string(), relate_name);

        false
    }

    pub(super) fn socket_add_extras(&self) -> bool {
        if self.config.no_accept_socket() {
            if self.config.unit_ref_target().is_none() {
                if !self.load_related_unit(UnitType::UnitService) {
                    return false;
                }
            }

            self.comm.unit().insert_two_deps(
                UnitRelations::UnitBefore,
                UnitRelations::UnitTriggers,
                self.config.unit_ref_target().unwrap(),
            );
        }
        true
    }

    pub(super) fn socket_verify(&self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    pub(super) fn parse(&self, socket_conf: SocketConf) -> Result<(), Box<dyn Error>> {
        self.parse_listen_socket(ListeningItem::Stream, &socket_conf)?;

        self.parse_listen_socket(ListeningItem::Datagram, &socket_conf)?;

        self.parse_listen_socket(ListeningItem::Netlink, &socket_conf)?;

        self.parse_pass_pktinfo(&socket_conf)?;
        self.parse_accept(&socket_conf)?;
        self.parse_pass_cred(&socket_conf)?;

        self.parse_pass_sec(&socket_conf)?;
        self.parse_socket_service(&socket_conf)?;

        self.parse_socket_mode(&socket_conf)?;

        self.parse_send_buffer(&socket_conf)?;
        self.parse_receive_buffer(&socket_conf)?;

        Ok(())
    }

    fn parse_receive_buffer(&self, socket_conf: &SocketConf) -> Result<(), Box<dyn Error>> {
        let r_val = socket_conf.get_receive_buffer();
        match conf_parser::parse_size(&r_val, conf_parser::Base::Binary) {
            Ok(size) => self.config.set(SocketConfigItem::ScReceiveBuffer(size)),
            Err(_) => todo!(),
        }

        Ok(())
    }

    fn parse_send_buffer(&self, socket_conf: &SocketConf) -> Result<(), Box<dyn Error>> {
        let r_val = socket_conf.get_send_buffer();
        match conf_parser::parse_size(&r_val, conf_parser::Base::Binary) {
            Ok(size) => self.config.set(SocketConfigItem::ScSendBuffer(size)),
            Err(_) => todo!(),
        }

        Ok(())
    }

    fn parse_socket_mode(&self, socket_conf: &SocketConf) -> Result<(), Box<dyn Error>> {
        let r_val = socket_conf.get_socket_mode();
        match u32::from_str_radix(&r_val, 8) {
            Ok(mode) => self.config.set(SocketConfigItem::ScSocketMode(mode)),
            Err(_) => todo!(),
        }

        Ok(())
    }

    fn parse_pass_sec(&self, socket_conf: &SocketConf) -> Result<(), Box<dyn Error>> {
        let r_val = socket_conf.get_pass_sec();
        match conf_parser::parse_boolen(&r_val) {
            Ok(sec) => {
                self.config.set(SocketConfigItem::ScSecurity(sec));
                Ok(())
            }
            Err(e) => return Err(Box::new(e)),
        }
    }

    fn parse_pass_cred(&self, socket_conf: &SocketConf) -> Result<(), Box<dyn Error>> {
        let r_val = socket_conf.get_pass_cred();
        match conf_parser::parse_boolen(&r_val) {
            Ok(cred) => {
                self.config.set(SocketConfigItem::ScCred(cred));
                Ok(())
            }
            Err(e) => return Err(Box::new(e)),
        }
    }

    fn parse_socket_service(&self, socket_conf: &SocketConf) -> Result<(), Box<dyn Error>> {
        let r_val = socket_conf.get_service();
        if !r_val.ends_with(".service") {
            return Err(format!("socket service must be end with .service").into());
        }

        if !self.comm.um().load_unit_success(&r_val) {
            return Err(format!("failed to load unit {}", r_val).into());
        }

        self.config
            .set_ref(self.comm.unit().get_id().to_string(), r_val);

        Ok(())
    }

    fn parse_pass_pktinfo(&self, socket_conf: &SocketConf) -> Result<(), Box<dyn Error>> {
        let r_val = socket_conf.get_pass_pktinfo();
        match conf_parser::parse_boolen(&r_val) {
            Ok(pass) => {
                self.config.set(SocketConfigItem::ScPassPktinfo(pass));
                Ok(())
            }
            Err(e) => return Err(Box::new(e)),
        }
    }

    fn parse_accept(&self, socket_conf: &SocketConf) -> Result<(), Box<dyn Error>> {
        let r_val = socket_conf.get_accept();
        match conf_parser::parse_boolen(&r_val) {
            Ok(accept) => {
                self.config.set(SocketConfigItem::ScAccept(accept));
                Ok(())
            }
            Err(e) => return Err(Box::new(e)),
        }
    }

    fn parse_listen_socket(
        &self,
        item: ListeningItem,
        socket_conf: &SocketConf,
    ) -> Result<(), Box<dyn Error>> {
        // let sock_addr
        match item {
            ListeningItem::Stream => {
                let r_val = socket_conf.get_listen_stream();
                if r_val.is_empty() {
                    self.config.clear_ports();
                    return Ok(());
                }

                if let Ok(socket_addr) = self.parse_socket_address(&r_val, libc::SOCK_STREAM) {
                    let mut port = SocketPort::new(socket_addr);
                    port.set_sc_type(PortType::Socket);

                    self.config.push_port(port);
                } else {
                    log::error!("create stream listening socket failed: {}", r_val);
                    return Err(format!("create stream listening socket failed: {}", r_val).into());
                }
            }
            ListeningItem::Datagram => {
                let r_val = socket_conf.get_listen_datagram();
                if r_val.is_empty() {
                    self.config.clear_ports();
                    return Ok(());
                }

                if let Ok(socket_addr) = self.parse_socket_address(&r_val, libc::SOCK_DGRAM) {
                    let mut port = SocketPort::new(socket_addr);
                    port.set_sc_type(PortType::Socket);
                    self.config.push_port(port);
                } else {
                    log::error!("create stream listening socket failed: {}", r_val);
                    return Err(format!("create stream listening socket failed: {}", r_val).into());
                }
            }
            ListeningItem::Netlink => {
                let r_val = socket_conf.get_listen_netlink();
                if r_val.is_empty() {
                    self.config.clear_ports();
                    return Ok(());
                }

                if let Ok(socket_addr) = self.parse_netlink_address(&r_val) {
                    let mut port = SocketPort::new(socket_addr);
                    port.set_sc_type(PortType::Socket);
                    self.config.push_port(port);
                } else {
                    log::error!("create stream listening socket failed: {}", r_val);
                    return Err(format!("create stream listening socket failed: {}", r_val).into());
                }
            }
        }

        Ok(())
    }

    fn parse_netlink_address(&self, item: &str) -> Result<SocketAddress, Box<dyn Error>> {
        let words: Vec<String> = item
            .trim_end()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();
        if words.len() == 2 {
            return Err(format!("Netlink configuration format is not correct: {}", item).into());
        }

        let family = NetlinkProtocol::from(words[0].to_string());
        if family == NetlinkProtocol::NetlinkInvalid {
            return Err(format!("Netlink family is invalid").into());
        }

        let group = if let Ok(g) = words[1].parse::<u32>() {
            g
        } else {
            return Err(format!("Netlink group is invalid").into());
        };

        let sock_addr: SockAddr = SockAddr::new_netlink(0, group);
        return Ok(SocketAddress::new(sock_addr, libc::SOCK_RAW, family as i32));
    }

    fn parse_socket_address(
        &self,
        item: &str,
        socket_type: i32,
    ) -> Result<SocketAddress, Box<dyn Error>> {
        if item.starts_with("/") {
            let sock_unit = SockAddr::new_unix(&PathBuf::from(item))?;

            return Ok(SocketAddress::new(sock_unit, socket_type, 0));
        }

        if item.starts_with("@") {
            let unix_addr = UnixAddr::new_abstract(item.as_bytes())?;

            return Ok(SocketAddress::new(
                SockAddr::Unix(unix_addr),
                socket_type,
                0,
            ));
        }

        if let Ok(port) = item.parse::<u16>() {
            if port == 0 {
                return Err(format!("invalid port number").into());
            }

            let sock_unit = if socket_util::ipv6_is_supported() {
                let sock_unit =
                    SockAddr::Inet(InetAddr::new(IpAddr::new_v6(0, 0, 0, 0, 0, 0, 0, 0), port));
                sock_unit
            } else {
                let sock_unit = SockAddr::Inet(InetAddr::new(IpAddr::new_v4(0, 0, 0, 0), port));
                sock_unit
            };

            return Ok(SocketAddress::new(sock_unit, socket_type, 0));
        }

        if let Ok(socket_addr) = item.parse::<SocketAddr>() {
            let sock_unit = SockAddr::Inet(InetAddr::from_std(&socket_addr));
            return Ok(SocketAddress::new(sock_unit, socket_type, 0));
        }

        return Err(format!("invalid listening config").into());
    }
}
