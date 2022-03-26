extern crate siphasher;

use std::any::Any;
use std::error::Error;
use std::rc::Rc;
use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use crate::manager::data::*;
use crate::manager::unit::unit_base::*;
use crate::manager::unit::unit_manager::*;
use super::uu_load::{UeLoad};
use super::uu_config::{UeConfig};
use super::uu_dep::{UeDep};
use super::uu_child::{UeChild};

use nix::sys::signal::Signal;
use nix::unistd::Pid;
use log;


#[derive(Debug)]
pub struct Unit {
    dm: Rc<DataManager>,
    pub unit_type: UnitType,
    pub id: String,
    config: UeConfig,
    dependencies: UeDep,
    pub load: UeLoad,
    child: UeChild,
    sub: Box<dyn UnitObj>,
}

impl PartialEq for Unit {
     fn eq(&self, other: &Self) -> bool {
         self.unit_type == other.unit_type && self.id == other.id
     }
}

impl Eq for Unit {

}

impl Hash for Unit {
    fn hash<H:Hasher>(&self, state:&mut H) {
        self.id.hash(state);
    }
}

pub trait UnitObj: std::fmt::Debug {
    fn init(&self){}
    fn done(&self){}
    fn load(&mut self, _m: &mut UnitManager) -> Result<(), Box<dyn Error>> {Ok(())}
    fn coldplug(&self){}
    fn dump(&self){}
    fn start(&mut self, _m: &mut UnitManager){}
    fn stop(&mut self, _m: &mut UnitManager){}
    fn reload(&mut self, _m: &mut UnitManager){}
    
    fn kill(&self){}
    fn check_gc(&self)->bool;
    fn release_resources(&self){}
    fn check_snapshot(&self){}
    fn sigchld_events(&mut self,_m: &mut UnitManager, _pid:Pid,_code:i32, _status:Signal) {}
    fn reset_failed(&self){}
    fn trigger(&mut self, _other: Rc<RefCell<Box<dyn UnitObj>>>) {}
    fn in_load_queue(&self) -> bool;

    fn eq(&self, other: &dyn UnitObj) -> bool;
    fn hash(&self) -> u64;
    fn as_any(&self) -> &dyn Any;
    fn getDependencies(&self) -> Vec<(UnitRelations,Rc<RefCell<Box<dyn UnitObj>>>)>  { let v: Vec<(UnitRelations,Rc<RefCell<Box<dyn UnitObj>>>)> = Vec::new(); v}
}

#[macro_export]
macro_rules! declure_unitobj_plugin {
    ($unit_type:ty, $constructor:path) => {
        #[no_mangle]
        pub fn __unit_obj_create() -> *mut dyn $crate::manager::unit::UnitObj {
            let construcotr: fn() ->$unit_type = $constructor;

            let obj = construcotr();
            let boxed: Box<dyn $crate::manager::unit::UnitObj>  = Box::new(obj);
            Box::into_raw(boxed)
        }
    };
}

impl Unit {
    pub fn new(dm:Rc<DataManager>, unit_type:UnitType, name: &str, sub:Box<dyn UnitObj>) -> Self {
        Unit{
            dm:Rc::clone(&dm),
            unit_type,
            id: String::from(name),
            config: UeConfig::new(),
            dependencies: UeDep::new(),
            load: UeLoad::new(Rc::clone(&dm), String::from(name)),
            child: UeChild::new(),
            sub,
        }
    }

    pub fn notify(&self, manager: &mut UnitManager, original_state: UnitActiveState, new_state: UnitActiveState) {
        if original_state != new_state {
            log::debug!("unit active state change from: {:?} to {:?}", original_state, new_state);
        }

        let unitx = manager.units.get_unit_on_name(&self.id).unwrap();
        for other in self.dependencies.get(UnitRelations::UnitTriggeredBy) {
            other.borrow().trigger(Rc::clone(&unitx));
        }
    }
 }






