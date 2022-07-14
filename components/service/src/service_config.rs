use super::service_base::{ServiceCommand, ServiceType};
use proc_macro_utils::ConfigParseM;
use process1::manager::ExecCommand;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{Error as IoError, ErrorKind};
use std::rc::Rc;
use utils::config_parser::{toml_str_parse, ConfigParse};

#[derive(Serialize, Deserialize, ConfigParseM)]
#[serdeName("Service")]
#[serde(rename_all = "PascalCase")]
pub(super) struct ServiceConf {
    #[serde(alias = "Type", default = "ServiceType::default")]
    service_type: ServiceType,
    #[serde(alias = "BusName")]
    bus_name: Option<String>,
    #[serde(alias = "ExecStart")]
    exec_start: Option<Vec<String>>,
    #[serde(alias = "ExecStop")]
    exec_stop: Option<Vec<String>>,
    #[serde(alias = "ExecCondition")]
    exec_condition: Option<Vec<String>>,
    #[serde(alias = "Sockets")]
    sockets: Option<String>,
    #[serde(alias = "Restart")]
    restart: Option<Vec<String>>,
    #[serde(alias = "RestrictRealtime")]
    restrict_realtime: Option<String>,
    #[serde(alias = "RebootArgument")]
    reboot_argument: Option<String>,
    #[serde(alias = "ExecReload")]
    exec_reload: Option<Vec<String>>,
    #[serde(alias = "OOMScoreAdjust")]
    oom_score_adjust: Option<String>,
    #[serde(alias = "RestartSec")]
    restart_sec: Option<u64>,
    #[serde(alias = "WatchdogUSec")]
    watchdog_sec: Option<u64>,
    #[serde(alias = "Slice")]
    slice: Option<String>,
    #[serde(alias = "MemoryLimit")]
    memory_limit: Option<u64>,
    #[serde(alias = "MemoryLow")]
    memory_low: Option<u64>,
    #[serde(alias = "MemoryMin")]
    memory_min: Option<u64>,
    #[serde(alias = "MemoryMax")]
    memory_max: Option<u64>,
    #[serde(alias = "MemoryHigh")]
    memory_high: Option<u64>,
    #[serde(alias = "MemorySwapMax")]
    memory_swap_max: Option<u64>,
}

pub(super) enum ServiceConfigItem {
    ScItemType(ServiceType),
    ScItemRestartSec(u64),
    ScItemWatchdogSec(u64),
    ScItemBusName(Option<String>),
}

pub(super) struct ServiceConfig {
    data: RefCell<ServiceConfigData>,
}

impl ServiceConfig {
    pub(super) fn new() -> ServiceConfig {
        ServiceConfig {
            data: RefCell::new(ServiceConfigData::new()),
        }
    }

    pub(super) fn set(&self, item: ServiceConfigItem) {
        self.data.borrow_mut().set(item)
    }

    pub(super) fn insert_exec_cmds(&self, cmd_type: ServiceCommand, cmd_line: Rc<ExecCommand>) {
        self.data.borrow_mut().insert_exec_cmds(cmd_type, cmd_line)
    }

    pub(super) fn get(&self, item: &ServiceConfigItem) -> ServiceConfigItem {
        self.data.borrow().get(item)
    }

    pub(super) fn service_type(&self) -> ServiceType {
        self.data.borrow().service_type()
    }

    pub(super) fn get_exec_cmds(&self, cmd_type: ServiceCommand) -> Vec<Rc<ExecCommand>> {
        self.data.borrow().get_exec_cmds(cmd_type)
    }
}

struct ServiceConfigData {
    service_type: ServiceType,
    restart_sec: u64,
    watchdog_sec: u64,
    bus_name: Option<String>,
    exec_commands: HashMap<ServiceCommand, Vec<Rc<ExecCommand>>>, // key: ServiceCommand, value: commands
}

// the declaration "pub(self)" is for identification only.
impl ServiceConfigData {
    pub(self) fn new() -> ServiceConfigData {
        ServiceConfigData {
            service_type: ServiceType::default(),
            exec_commands: HashMap::new(),
            restart_sec: 0,
            watchdog_sec: 0,
            bus_name: None,
        }
    }

    pub(self) fn set(&mut self, item: ServiceConfigItem) {
        match item {
            ServiceConfigItem::ScItemType(st) => {
                self.service_type = st;
            }
            ServiceConfigItem::ScItemRestartSec(rs_sec) => {
                self.restart_sec = rs_sec;
            }
            ServiceConfigItem::ScItemWatchdogSec(wd_sec) => {
                self.watchdog_sec = wd_sec;
            }
            ServiceConfigItem::ScItemBusName(bus_name) => {
                self.bus_name = bus_name;
            }
        }
    }

    pub(self) fn insert_exec_cmds(&mut self, cmd_type: ServiceCommand, cmd_line: Rc<ExecCommand>) {
        self.get_mut_cmds_pad(cmd_type).push(cmd_line);
    }

    pub(self) fn get(&self, item: &ServiceConfigItem) -> ServiceConfigItem {
        match item {
            ServiceConfigItem::ScItemType(_) => ServiceConfigItem::ScItemType(self.service_type),
            ServiceConfigItem::ScItemRestartSec(_) => {
                ServiceConfigItem::ScItemRestartSec(self.restart_sec)
            }
            ServiceConfigItem::ScItemWatchdogSec(_) => {
                ServiceConfigItem::ScItemWatchdogSec(self.watchdog_sec)
            }
            ServiceConfigItem::ScItemBusName(_) => {
                ServiceConfigItem::ScItemBusName(self.bus_name.as_ref().map(|s| s.to_string()))
            }
        }
    }

    pub(self) fn service_type(&self) -> ServiceType {
        self.service_type
    }

    pub(self) fn get_exec_cmds(&self, cmd_type: ServiceCommand) -> Vec<Rc<ExecCommand>> {
        if let Some(cmds) = self.exec_commands.get(&cmd_type) {
            cmds.iter().map(|clr| Rc::clone(clr)).collect::<_>()
        } else {
            Vec::new()
        }
    }

    fn get_mut_cmds_pad(&mut self, cmd_type: ServiceCommand) -> &mut Vec<Rc<ExecCommand>> {
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
