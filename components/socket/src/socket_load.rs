//! socket_load模块实现socket配置文件的加载解析。
//!

use nix::sys::socket::{
    NetlinkAddr, SockProtocol, SockType, SockaddrIn, SockaddrIn6, SockaddrLike, UnixAddr,
};
use process1::manager::{UnitRelations, UnitType};
use std::cell::RefCell;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use std::path::PathBuf;
use std::{error::Error, rc::Rc};
use utils::socket_util;

use crate::socket_base::{NetlinkProtocol, PortType};
use crate::socket_comm::SocketComm;
use crate::socket_config::{ListeningItem, SocketConfig, SocketConfigData};
use crate::socket_mng::SocketMng;
use crate::socket_port::{SocketAddress, SocketPort, SocketPorts};

pub(super) struct SocketLoad {
    config: Rc<SocketConfig>,
    comm: Rc<SocketComm>,
    ports: Rc<SocketPorts>,
}

impl SocketLoad {
    pub(super) fn new(
        configr: &Rc<SocketConfig>,
        commr: &Rc<SocketComm>,
        ports: &Rc<SocketPorts>,
    ) -> Self {
        SocketLoad {
            config: configr.clone(),
            comm: commr.clone(),
            ports: ports.clone(),
        }
    }

    pub(super) fn socket_add_extras(&self, mng: &Rc<SocketMng>) -> bool {
        log::debug!("socket add extras");
        if self.can_accept() {
            if mng.unit_ref_target().is_none() {
                if !mng.load_related_unit(UnitType::UnitService) {
                    return false;
                }
            }

            self.comm.unit().insert_two_deps(
                UnitRelations::UnitBefore,
                UnitRelations::UnitTriggers,
                mng.unit_ref_target().unwrap(),
            );
        }
        true
    }

    pub(super) fn socket_verify(&self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    pub(super) fn parse(
        &self,
        socket_conf: Rc<RefCell<SocketConfigData>>,
        mng: &Rc<SocketMng>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        log::debug!("begin to parse socket section");

        self.parse_listen_socket(ListeningItem::Stream, socket_conf.clone())?;

        self.parse_listen_socket(ListeningItem::Datagram, socket_conf.clone())?;

        self.parse_listen_socket(ListeningItem::Netlink, socket_conf.clone())?;

        self.parse_socket_service(mng)?;

        Ok(())
    }

    fn can_accept(&self) -> bool {
        if let Some(accept) = self.config.config_data().borrow().Socket.Accept {
            if !accept {
                return true;
            }
        };

        self.ports.no_accept_socket()
    }

    fn parse_socket_service(&self, mng: &Rc<SocketMng>) -> Result<(), Box<dyn Error>> {
        if let Some(service) = self.config.config_data().borrow().Socket.Service.clone() {
            if !service.ends_with(".service") {
                return Err(format!("socket service must be end with .service").into());
            }

            if !self.comm.um().load_unit_success(&service) {
                return Err(format!("failed to load unit {}", service).into());
            }

            mng.set_ref(self.comm.unit().get_id().to_string(), service);
        }

        Ok(())
    }

    fn parse_listen_socket(
        &self,
        item: ListeningItem,
        socket_conf: Rc<RefCell<SocketConfigData>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // let sock_addr
        match item {
            ListeningItem::Stream => {
                if let Some(listen_stream) = socket_conf.borrow().Socket.ListenStream.clone() {
                    if listen_stream.is_empty() {
                        return Ok(());
                    }
                    if let Ok(socket_addr) =
                        self.parse_socket_address(&listen_stream, SockType::Stream)
                    {
                        let mut port = SocketPort::new(socket_addr, self.config.clone());
                        port.set_sc_type(PortType::Socket);

                        self.ports.push_port(Rc::new(port));
                    } else {
                        log::error!("create stream listening socket failed: {}", listen_stream);
                        return Err(format!(
                            "create stream listening socket failed: {}",
                            listen_stream
                        )
                        .into());
                    }
                };
            }
            ListeningItem::Datagram => {
                if let Some(listen_datagram) = socket_conf.borrow().Socket.ListenDatagram.clone() {
                    if listen_datagram.is_empty() {
                        return Ok(());
                    }

                    if let Ok(socket_addr) =
                        self.parse_socket_address(&listen_datagram, SockType::Datagram)
                    {
                        let mut port = SocketPort::new(socket_addr, self.config.clone());
                        port.set_sc_type(PortType::Socket);
                        self.ports.push_port(Rc::new(port));
                    } else {
                        log::error!("create stream listening socket failed: {}", listen_datagram);
                        return Err(format!(
                            "create stream listening socket failed: {}",
                            listen_datagram
                        )
                        .into());
                    }
                }
            }
            ListeningItem::Netlink => {
                if let Some(listen_netlink) = socket_conf.borrow().Socket.ListenNetlink.clone() {
                    if listen_netlink.is_empty() {
                        return Ok(());
                    }

                    if let Err(e) = self.parse_netlink_address(&listen_netlink) {
                        log::error!(
                            "create stream listening socket: {}, failed: {:?}",
                            listen_netlink,
                            e
                        );
                        return Err(format!(
                            "create stream listening socket failed: {}",
                            listen_netlink
                        )
                        .into());
                    }

                    let socket_addr = self.parse_netlink_address(&listen_netlink).unwrap();
                    let mut port = SocketPort::new(socket_addr, self.config.clone());
                    port.set_sc_type(PortType::Socket);
                    self.ports.push_port(Rc::new(port));
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
        if words.len() != 2 {
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

        let net_link = NetlinkAddr::new(0, group);

        return Ok(SocketAddress::new(
            Box::new(net_link),
            SockType::Raw,
            Some(SockProtocol::from(family)),
        ));
    }

    fn parse_socket_address(
        &self,
        item: &str,
        socket_type: SockType,
    ) -> Result<SocketAddress, Box<dyn Error>> {
        if item.starts_with("/") {
            let unix_addr = UnixAddr::new(&PathBuf::from(item))?;
            return Ok(SocketAddress::new(Box::new(unix_addr), socket_type, None));
        }

        if item.starts_with("@") {
            let unix_addr = UnixAddr::new_abstract(item.as_bytes())?;

            return Ok(SocketAddress::new(Box::new(unix_addr), socket_type, None));
        }

        if let Ok(port) = item.parse::<u16>() {
            if port == 0 {
                return Err(format!("invalid port number").into());
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

        return Err(format!("invalid listening config").into());
    }
}
