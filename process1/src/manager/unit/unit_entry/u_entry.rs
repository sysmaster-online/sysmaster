use super::uu_child::UeChild;
use super::uu_config::{UeConfig, UnitConfigItem};
use super::uu_load::UeLoad;
use crate::manager::data::{DataManager, UnitActiveState, UnitState};
use crate::manager::unit::uload_util::{UnitFile};
use crate::manager::unit::unit_base::{UnitActionError, UnitLoadState, UnitType};

use log;
use nix::sys::signal::Signal;
use nix::unistd::Pid;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

pub struct Unit {
    // associated objects
    dm: Rc<DataManager>,

    // owned objects
    unit_type: UnitType,
    id: String,

    config: Rc<UeConfig>,
    load: UeLoad,
    child: UeChild,
    sub: RefCell<Box<dyn UnitObj>>,
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

pub trait UnitObj {
    fn init(&self) {}
    fn done(&self) {}
    fn load(&mut self, conf_str: &str) -> Result<(), Box<dyn Error>>;

    fn coldplug(&self) {}
    fn dump(&self) {}
    fn start(&mut self) -> Result<(), UnitActionError> {
        Ok(())
    }
    fn stop(&mut self) -> Result<(), UnitActionError> {
        Ok(())
    }
    fn reload(&mut self) {}

    fn kill(&self) {}
    fn release_resources(&self) {}
    fn sigchld_events(&mut self, _pid: Pid, _code: i32, _status: Signal) {}
    fn reset_failed(&self) {}
    fn trigger(&mut self, _other: Rc<RefCell<Box<dyn UnitObj>>>) {}

    fn get_private_conf_section_name(&self) -> Option<&str>;
    fn current_active_state(&self) -> UnitActiveState;
    fn attach_unit(&mut self, unit: Rc<Unit>);
}

impl Unit {
    pub fn notify(
        &self,
        original_state: UnitActiveState,
        new_state: UnitActiveState,
        flags: isize,
    ) {
        if original_state != new_state {
            log::debug!(
                "unit active state change from: {:?} to {:?}",
                original_state,
                new_state
            );
        }
        let u_state = UnitState::new(original_state, new_state, flags);
        self.dm.insert_unit_state(self.id.clone(), u_state);
    }

    pub fn get_id(&self) -> &str {
        &self.id
    }

    pub(super) fn new(
        unit_type: UnitType,
        name: &str,
        dmr: &Rc<DataManager>,
        filer: &Rc<UnitFile>,
        sub: RefCell<Box<dyn UnitObj>>,
    ) -> Self {
        let _config = Rc::new(UeConfig::new());
        Unit {
            dm: Rc::clone(dmr),
            unit_type,
            id: String::from(name),
            config: Rc::clone(&_config),
            load: UeLoad::new(dmr, filer, &_config, String::from(name)),
            child: UeChild::new(),
            sub,
        }
    }

    pub(super) fn in_load_queue(&self) -> bool {
        self.load.in_load_queue()
    }

    pub(super) fn set_in_load_queue(&self, t: bool) {
        self.load.set_in_load_queue(t);
    }

    pub(super) fn get_config(&self, item: &UnitConfigItem) -> UnitConfigItem {
        self.config.get(item)
    }

    pub(super) fn load_unit(&self) -> Result<(), Box<dyn Error>> {
        self.set_in_load_queue(false);
        match self.load.load_unit_confs() {
            Ok(confs) => {
                let ret = self.sub.borrow_mut().load(&confs);
                if let Ok(_) = ret {
                    self.load.set_load_state(UnitLoadState::UnitLoaded);
                } else {
                    return Err(format!("load Unit {} failed", self.id).into());
                }
                ret
            }
            Err(e) => {
                self.load.set_load_state(UnitLoadState::UnitNotFound);
                return Err(e);
            }
        }
    }

    pub(super) fn get_private_conf_section_name(&self) -> Option<String> {
        let str = self
            .sub
            .borrow()
            .get_private_conf_section_name()
            .map(|s| s.to_string());
        str
    }

    pub(super) fn current_active_state(&self) -> UnitActiveState {
        self.sub.borrow().current_active_state()
    }

    pub(super) fn start(&self) -> Result<(), UnitActionError> {
        self.sub.borrow_mut().start()
    }

    pub(super) fn stop(&self) -> Result<(), UnitActionError> {
        self.sub.borrow_mut().stop()
    }

    pub(super) fn sigchld_events(&self, pid: Pid, code: i32, signal: Signal) {
        self.sub.borrow_mut().sigchld_events(pid, code, signal)
    }

    pub(super) fn get_load_state(&self) -> UnitLoadState {
        self.load.get_load_state()
    }

    pub(super) fn attach_unit(&self, unit: &Rc<Unit>) {
        self.sub.borrow_mut().attach_unit(Rc::clone(unit))
    }
}
