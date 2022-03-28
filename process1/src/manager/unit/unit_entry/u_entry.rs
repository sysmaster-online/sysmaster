extern crate siphasher;

use super::uu_child::UeChild;
use super::uu_config::UeConfig;
use super::uu_load::UeLoad;
use crate::manager::data::{UnitType, UnitRelations, DataManager};
use crate::manager::unit::unit_base::{UnitActiveState};
use crate::manager::unit::unit_manager::{UnitManager};
use utils:: {unit_config_parser};
use std::any::Any;
use std::cell::RefCell;
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use log;
use nix::sys::signal::Signal;
use nix::unistd::Pid;

#[derive(Debug)]
pub struct Unit {
    unit_type: UnitType,
    id: String,
    config: UeConfig,
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

impl Hash for Unit {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

pub trait UnitObj: std::fmt::Debug {
    fn init(&self) {}
    fn done(&self) {}
    fn load(&mut self, _m: &mut UnitManager) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
    fn coldplug(&self) {}
    fn dump(&self) {}
    fn start(&mut self, _m: &mut UnitManager) {}
    fn stop(&mut self, _m: &mut UnitManager) {}
    fn reload(&mut self, _m: &mut UnitManager) {}

    fn kill(&self) {}
    fn check_gc(&self) -> bool;
    fn release_resources(&self) {}
    fn check_snapshot(&self) {}
    fn sigchld_events(&mut self, _m: &mut UnitManager, _pid: Pid, _code: i32, _status: Signal) {}
    fn reset_failed(&self) {}
    fn trigger(&mut self, _other: Rc<RefCell<Box<dyn UnitObj>>>) {}
    fn in_load_queue(&self) -> bool;

    fn eq(&self, other: &dyn UnitObj) -> bool;
    fn hash(&self) -> u64;
    fn as_any(&self) -> &dyn Any;
}

#[macro_export]
macro_rules! declure_unitobj_plugin {
    ($unit_type:ty, $constructor:path) => {
        #[no_mangle]
        pub fn __unit_obj_create() -> *mut dyn $crate::manager::UnitObj {
            let construcotr: fn() -> $unit_type = $constructor;

            let obj = construcotr();
            let boxed: Box<dyn $crate::manager::UnitObj> = Box::new(obj);
            Box::into_raw(boxed)
        }
    };
}

impl Unit {
    pub fn load_unit(&self, m: &mut UnitManager) -> Result<(), Box<dyn Error>> {
        self.load.unit_load(m)
    }

    pub fn load_in_queue(&self) -> bool {
        self.load.in_load_queue()
    }

    pub fn load_parse(&self, m: &mut UnitManager) -> Result<(), Box<dyn Error>> {
        self.load.parse(m)
    }

    pub fn load_get_conf(&self) -> Option<Rc<unit_config_parser::Conf>> {
        self.load.get_conf()
    }

    pub fn notify(
        &self,
        manager: &mut UnitManager,
        original_state: UnitActiveState,
        new_state: UnitActiveState,
    ) {
        if original_state != new_state {
            log::debug!(
                "unit active state change from: {:?} to {:?}",
                original_state,
                new_state
            );
        }

        let unitx = manager.units_get(&self.id).unwrap();
        for other in manager.dep_get(&unitx, UnitRelations::UnitTriggeredBy) {
            other.trigger(Rc::clone(&unitx));
        }
    }

    pub fn get_id(&self) -> &str {
        &self.id
    }

    pub fn get_unit_type(&self) -> UnitType {
        self.unit_type
    }

    pub(super) fn new(
        dm: Rc<DataManager>,
        unit_type: UnitType,
        name: &str,
        sub: Box<dyn UnitObj>,
    ) -> Self {
        Unit {
            unit_type,
            id: String::from(name),
            config: UeConfig::new(),
            load: UeLoad::new(Rc::clone(&dm), String::from(name)),
            child: UeChild::new(),
            sub,
        }
    }
}
