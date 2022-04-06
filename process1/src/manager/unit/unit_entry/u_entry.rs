extern crate siphasher;
use super::uu_child::UeChild;
use super::uu_config::UeConfig;
use super::uu_load::UeLoad;
use super::uu_state::UeState;
use crate::manager::data::{DataManager, UnitActiveState, UnitType};
use crate::manager::unit::unit_file::UnitFile;
use log;
use nix::sys::signal::Signal;
use nix::unistd::Pid;
use std::any::Any;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use utils::unit_config_parser;

#[derive(Debug)]
pub struct Unit {
    // associated objects
    file: Rc<UnitFile>,

    unit_type: UnitType,
    id: String,
    config: UeConfig,
    state: UeState,
    load: UeLoad,
    child: UeChild,
    sub: Box<dyn UnitObj>,
}

impl PartialEq for Unit {
    fn eq(&self, other: &Self) -> bool {
        self.unit_type == other.unit_type && self.id == other.id
    }
}

impl Eq for Unit {}

impl PartialOrd for Unit {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Unit {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl Hash for Unit {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

pub trait UnitObj: std::fmt::Debug {
    fn init(&self) {}
    fn done(&self) {}
    fn load(&mut self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
    fn coldplug(&self) {}
    fn dump(&self) {}
    fn start(&mut self) {}
    fn stop(&mut self) {}
    fn reload(&mut self) {}

    fn kill(&self) {}
    fn check_gc(&self) -> bool;
    fn release_resources(&self) {}
    fn check_snapshot(&self) {}
    fn sigchld_events(&mut self, _pid: Pid, _code: i32, _status: Signal) {}
    fn reset_failed(&self) {}
    fn trigger(&mut self, _other: Rc<RefCell<Box<dyn UnitObj>>>) {}
    fn in_load_queue(&self) -> bool;

    fn eq(&self, other: &dyn UnitObj) -> bool;
    fn hash(&self) -> u64;
    fn as_any(&self) -> &dyn Any;
}

impl Unit {
    pub fn load_unit(&self) -> Result<(), Box<dyn Error>> {
        self.load.unit_load(&self.file)
    }

    pub fn load_in_queue(&self) -> bool {
        self.load.in_load_queue()
    }

    pub fn load_parse(&self) -> Result<(), Box<dyn Error>> {
        self.load.parse()
    }

    pub fn load_get_conf(&self) -> Option<Rc<unit_config_parser::Conf>> {
        self.load.get_conf()
    }

    pub fn notify(&self, original_state: UnitActiveState, new_state: UnitActiveState) {
        if original_state != new_state {
            log::debug!(
                "unit active state change from: {:?} to {:?}",
                original_state,
                new_state
            );
        }

        self.state.update(&self.id, original_state, new_state, 0);
    }

    pub fn get_id(&self) -> &str {
        &self.id
    }

    pub fn get_unit_type(&self) -> UnitType {
        self.unit_type
    }

    pub(super) fn new(
        dm: Rc<DataManager>,
        file: Rc<UnitFile>,
        unit_type: UnitType,
        name: &str,
        sub: Box<dyn UnitObj>,
    ) -> Self {
        Unit {
            unit_type,
            file,
            id: String::from(name),
            config: UeConfig::new(),
            state: UeState::new(Rc::clone(&dm)),
            load: UeLoad::new(Rc::clone(&dm), String::from(name)),
            child: UeChild::new(),
            sub,
        }
    }
}
