use std::rc::Rc;
use std::cell::RefCell;
use std::any::Any;
use std::error::Error;
use nix::sys::signal::Signal;
use nix::unistd::Pid;
use crate::plugin::Plugin;
use crate::manager::data::*;
use super::u_entry::{Unit};

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct UnitX(Unit);

impl UnitX {
    pub fn new(dm:Rc<DataManager>, unit_type: UnitType, name: &str) -> Result<Rc<UnitX>, Box<dyn Error>> {
        let plugins = Rc::clone(&Plugin::get_instance());
        plugins.borrow_mut().set_library_dir("../target/debug");
        plugins.borrow_mut().load_lib();
        let unit_obj = plugins.borrow().create_unit_obj(unit_type)?;
        Ok(Rc::new(UnitX(Unit::new(Rc::clone(&dm), unit_type, name, unit_obj))))
    }

    pub fn init(&self){}
    pub fn done(&self){}
    pub fn load(&mut self) -> Result<(), Box<dyn Error>> {Ok(())}
    pub fn coldplug(&self){}
    pub fn dump(&self){}
    pub fn start(&mut self){}
    pub fn stop(&mut self){}
    pub fn reload(&mut self){}

    pub fn kill(&self){}
    pub fn check_gc(&self)->bool {todo!();}
    pub fn release_resources(&self){}
    pub fn check_snapshot(&self){}
    pub fn sigchld_events(&mut self,_pid:Pid,_code:i32, _status:Signal) {}
    pub fn reset_failed(&self){}
    pub fn trigger(&self, _other: Rc<RefCell<Rc<UnitX>>>) {}
    pub fn in_load_queue(&self) -> bool {
        //self.in_load_queue()
        todo!()
    }

    pub fn eq(&self, _other: &UnitX) -> bool {todo!();}
    pub fn hash(&self) -> u64 {todo!();}
    pub fn as_any(&self) -> &dyn Any {todo!();}
    pub fn set_config(&self, _config:&UnitConfig) {
        // get and compare each item, only the changed item needs to be set
        todo!()
    }
    pub fn get_config(&self, _item:UnitConfigItem) -> UnitConfigItem {todo!();}
    pub fn add_dependencies(&self, _relation: UnitRelations, _name: &str) {todo!();}
}




