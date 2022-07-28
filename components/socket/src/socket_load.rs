//! socket_load模块实现socket配置文件的加载解析。
//!

use nix::sys::socket::{InetAddr, IpAddr, SockAddr, SockProtocol, SockType, UnixAddr};
use process1::manager::{ExecCommand, UnitRelations, UnitType};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::{error::Error, rc::Rc};
use utils::{conf_parser, socket_util};

use crate::socket_base::{NetlinkProtocol, PortType, SocketCommand};
use crate::socket_comm::SocketComm;
use crate::socket_config::{ListeningItem, SocketConf, SocketConfig, SocketConfigItem};
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

    pub(super) fn socket_add_extras(&self) -> bool {
        log::debug!("socket add extras");
        if self.can_accept() {
            if self.config.unit_ref_target().is_none() {
                if !self.load_related_unit(UnitType::UnitService) {
                    return false;
                }
            }

            // self.comm.unit().insert_two_deps(
            //     UnitRelations::UnitBefore,
            //     UnitRelations::UnitTriggers,
            //     self.config.unit_ref_target().unwrap(),
            // );
        }
        true
    }

    pub(super) fn socket_verify(&self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    pub(super) fn parse(&self, socket_conf: SocketConf) -> Result<(), Box<dyn Error>> {
        log::debug!("begin to parse socket section");
        self.parse_command(&socket_conf)?;

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

    fn can_accept(&self) -> bool {
        if let SocketConfigItem::ScAccept(accept) =
            self.config.get(&SocketConfigItem::ScAccept(false))
        {
            if !accept {
                return true;
            }
        };

        self.ports.no_accept_socket()
    }

    fn load_related_unit(&self, related_type: UnitType) -> bool {
        let unit_name = self.comm.unit().get_id().to_string();
        let stem_name = Path::new(&unit_name).file_stem().unwrap().to_str().unwrap();

        let suffix = String::from(related_type);
        if suffix.len() == 0 {
            return false;
        }

        let relate_name = format!("{}.{}", stem_name, suffix);
        if !self.comm.um().load_unit_success(&relate_name) {
            return false;
        }

        self.config
            .set_ref(self.comm.unit().get_id().to_string(), relate_name);

        true
    }

    fn parse_receive_buffer(&self, socket_conf: &SocketConf) -> Result<(), Box<dyn Error>> {
        let r_val = socket_conf.get_receive_buffer().unwrap_or_default();
        if r_val.is_empty() {
            return Ok(());
        }

        match conf_parser::parse_size(&r_val, conf_parser::Base::Binary) {
            Ok(size) => self.config.set(SocketConfigItem::ScReceiveBuffer(size)),
            Err(e) => return Err(Box::new(e)),
        }

        Ok(())
    }

    fn parse_send_buffer(&self, socket_conf: &SocketConf) -> Result<(), Box<dyn Error>> {
        let r_val = socket_conf.get_send_buffer().unwrap_or_default();
        if r_val.is_empty() {
            return Ok(());
        }

        match conf_parser::parse_size(&r_val, conf_parser::Base::Binary) {
            Ok(size) => self.config.set(SocketConfigItem::ScSendBuffer(size)),
            Err(e) => return Err(Box::new(e)),
        }

        Ok(())
    }

    fn parse_socket_mode(&self, socket_conf: &SocketConf) -> Result<(), Box<dyn Error>> {
        let r_val = socket_conf.get_socket_mode().unwrap_or_default();
        if r_val.is_empty() {
            return Ok(());
        }
        println!("socket mode: {}", r_val.to_string());
        match u32::from_str_radix(&r_val, 8) {
            Ok(mode) => self.config.set(SocketConfigItem::ScSocketMode(mode)),
            Err(e) => return Err(Box::new(e)),
        }

        Ok(())
    }

    fn parse_pass_sec(&self, socket_conf: &SocketConf) -> Result<(), Box<dyn Error>> {
        let r_val = socket_conf.get_pass_sec().unwrap_or_default();
        if r_val.is_empty() {
            return Ok(());
        }

        match conf_parser::parse_boolen(&r_val) {
            Ok(sec) => {
                self.config.set(SocketConfigItem::ScSecurity(sec));
                Ok(())
            }
            Err(e) => return Err(Box::new(e)),
        }
    }

    fn parse_pass_cred(&self, socket_conf: &SocketConf) -> Result<(), Box<dyn Error>> {
        let r_val = socket_conf.get_pass_cred().unwrap_or_default();
        if r_val.is_empty() {
            return Ok(());
        }

        match conf_parser::parse_boolen(&r_val) {
            Ok(cred) => {
                self.config.set(SocketConfigItem::ScCred(cred));
                Ok(())
            }
            Err(e) => return Err(Box::new(e)),
        }
    }

    fn parse_socket_service(&self, socket_conf: &SocketConf) -> Result<(), Box<dyn Error>> {
        let r_val = socket_conf.get_service().unwrap_or_default();
        if r_val.is_empty() {
            return Ok(());
        }

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
        let r_val = socket_conf.get_pass_pktinfo().unwrap_or_default();
        if r_val.is_empty() {
            return Ok(());
        }

        match conf_parser::parse_boolen(&r_val) {
            Ok(pass) => {
                self.config.set(SocketConfigItem::ScPassPktinfo(pass));
                Ok(())
            }
            Err(e) => return Err(Box::new(e)),
        }
    }

    fn parse_accept(&self, socket_conf: &SocketConf) -> Result<(), Box<dyn Error>> {
        let r_val = socket_conf.get_accept().unwrap_or_default();
        if r_val.is_empty() {
            return Ok(());
        }

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
                let r_val = if let Some(v) = socket_conf.get_listen_stream() {
                    v
                } else {
                    return Ok(());
                };

                if r_val.is_empty() {
                    return Err(format!("ListenStream item is null").into());
                }

                if let Ok(socket_addr) = self.parse_socket_address(&r_val, SockType::Stream) {
                    let mut port = SocketPort::new(socket_addr, self.config.clone());
                    port.set_sc_type(PortType::Socket);

                    self.ports.push_port(Rc::new(port));
                } else {
                    log::error!("create stream listening socket failed: {}", r_val);
                    return Err(format!("create stream listening socket failed: {}", r_val).into());
                }
            }
            ListeningItem::Datagram => {
                let r_val = if let Some(v) = socket_conf.get_listen_datagram() {
                    v
                } else {
                    return Ok(());
                };

                if r_val.is_empty() {
                    return Err(format!("ListenDatagram item is null").into());
                }

                if let Ok(socket_addr) = self.parse_socket_address(&r_val, SockType::Datagram) {
                    let mut port = SocketPort::new(socket_addr, self.config.clone());
                    port.set_sc_type(PortType::Socket);
                    self.ports.push_port(Rc::new(port));
                } else {
                    log::error!("create stream listening socket failed: {}", r_val);
                    return Err(format!("create stream listening socket failed: {}", r_val).into());
                }
            }
            ListeningItem::Netlink => {
                let r_val = if let Some(v) = socket_conf.get_listen_netlink() {
                    v
                } else {
                    return Ok(());
                };

                if r_val.is_empty() {
                    return Err(format!("ListenDatagram item is null").into());
                }

                if let Ok(socket_addr) = self.parse_netlink_address(&r_val) {
                    let mut port = SocketPort::new(socket_addr, self.config.clone());
                    port.set_sc_type(PortType::Socket);
                    self.ports.push_port(Rc::new(port));
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
        return Ok(SocketAddress::new(
            sock_addr,
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
            let sock_unit = SockAddr::new_unix(&PathBuf::from(item))?;

            return Ok(SocketAddress::new(sock_unit, socket_type, None));
        }

        if item.starts_with("@") {
            let unix_addr = UnixAddr::new_abstract(item.as_bytes())?;

            return Ok(SocketAddress::new(
                SockAddr::Unix(unix_addr),
                socket_type,
                None,
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

            return Ok(SocketAddress::new(sock_unit, socket_type, None));
        }

        if let Ok(socket_addr) = item.parse::<SocketAddr>() {
            let sock_unit = SockAddr::Inet(InetAddr::from_std(&socket_addr));
            return Ok(SocketAddress::new(sock_unit, socket_type, None));
        }

        return Err(format!("invalid listening config").into());
    }

    fn parse_command(&self, socket_conf: &SocketConf) -> Result<(), Box<dyn Error>> {
        let update_exec_command = |command_type: SocketCommand| {
            let commands: Option<Vec<String>> = match command_type {
                SocketCommand::StartPre => socket_conf.get_exec_start_pre(),
                SocketCommand::StartChown => socket_conf.get_exec_start_chown(),
                SocketCommand::StartPost => socket_conf.get_exec_start_post(),
                SocketCommand::StopPre => socket_conf.get_exec_stop_pre(),
                SocketCommand::StopPost => socket_conf.get_exec_stop_post(),
            };
            if commands.is_some() {
                match self.prepare_command(command_type, &commands.unwrap()) {
                    Ok(_) => Ok(()),
                    Err(e) => {
                        return Err(e);
                    }
                }
            } else {
                return Ok(());
            }
        };

        update_exec_command(SocketCommand::StartPre)?;
        update_exec_command(SocketCommand::StartChown)?;
        update_exec_command(SocketCommand::StartPost)?;
        update_exec_command(SocketCommand::StopPre)?;
        update_exec_command(SocketCommand::StopPost)?;

        Ok(())
    }

    fn prepare_command(
        &self,
        cmd_type: SocketCommand,
        commands: &Vec<String>,
    ) -> Result<(), Box<dyn Error>> {
        if commands.len() == 0 {
            return Err(format!("config opton is error, value cannot be null").into());
        }

        let mut set_command = false;
        for cmd in commands.iter() {
            if cmd.is_empty() {
                continue;
            }

            set_command = true;
            let mut command: Vec<String> = cmd
                .trim_end()
                .split_whitespace()
                .map(|s| s.to_string())
                .collect();

            // get the command and leave the command args
            let exec_cmd = command.remove(0);
            let path = Path::new(&exec_cmd);

            if path.is_absolute() && !path.exists() {
                log::debug!("{:?} is not exist in parse!", path);
                return Err(format!("{:?} is not exist!", path).into());
            }

            let cmd = path.to_str().unwrap().to_string();
            let new_command = Rc::new(ExecCommand::new(cmd, command));

            self.config.insert_exec_cmds(cmd_type, new_command);
        }

        if set_command {
            Ok(())
        } else {
            return Err(format!("config opton is error, value cannot be null").into());
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::socket_config::SocketConfigItem;

    use super::*;
    use std::{fs::File, io::Read};
    use utils::config_parser::ConfigParse;

    #[test]
    fn test_socket_parse() {
        let file_path = "../../libutils/examples/test.socket";
        let mut file = File::open(file_path).unwrap();
        let mut buf = String::new();
        match file.read_to_string(&mut buf) {
            Ok(s) => s,
            Err(_e) => {
                return;
            }
        };

        let socket_parser = SocketConf::builder_parser();
        let socket_conf = socket_parser.conf_file_parse(buf.as_str());

        let comm = Rc::new(SocketComm::new());
        let config = Rc::new(SocketConfig::new());
        let ports = Rc::new(SocketPorts::new());
        let load = SocketLoad::new(&config, &comm, &ports);

        let ret = socket_conf.map(|conf| load.parse(conf));
        assert_ne!(ret.is_err(), true);

        for command in config.exec_cmds(SocketCommand::StartPre) {
            println!("cmd: {}, args: {:?}", command.path(), command.argv());
        }

        for port in ports.ports() {
            println!(
                "port type: {:?}, family: {:?}",
                port.p_type(),
                port.family()
            );
        }

        if let SocketConfigItem::ScAccept(v) = config.get(&SocketConfigItem::ScAccept(false)) {
            assert_eq!(v, false);
        }

        if let SocketConfigItem::ScPassPktinfo(v) =
            config.get(&SocketConfigItem::ScPassPktinfo(false))
        {
            assert_eq!(v, false);
        }

        if let SocketConfigItem::ScCred(v) = config.get(&SocketConfigItem::ScCred(false)) {
            assert_eq!(v, false);
        }

        if let SocketConfigItem::ScSecurity(v) = config.get(&SocketConfigItem::ScSecurity(false)) {
            assert_eq!(v, true);
        }

        if let SocketConfigItem::ScSocketMode(v) = config.get(&SocketConfigItem::ScSocketMode(0)) {
            assert_eq!(v, 384);
        }
    }
}
