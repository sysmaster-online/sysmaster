use std::any::Any;
use std::error::Error;
use nix::sys::signal::Signal;
use nix::unistd::Pid;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum UnitType {
    UnitService = 0,
    UnitTarget,
    UnitTypeMax,
    UnitTypeInvalid,
    UnitTypeErrnoMax,
}

impl Default for UnitType {
    fn default() -> Self { UnitType::UnitService }
}


pub trait UnitAction: std::fmt::Debug {
    fn init(&self){}
    fn done(&self){}
    fn load(&mut self) -> Result<(), Box<dyn Error>> {Ok(())}
    fn coldplug(&self){}
    fn dump(&self){}
    fn start(&mut self){}
    fn stop(&mut self){}
    fn reload(&mut self){}

    fn kill(&self){}
    fn check_gc(&self)->bool;
    fn release_resources(&self){}
    fn check_snapshot(&self){}
    fn sigchld_events(&mut self,_pid:Pid,_code:i32, _status:Signal) {}
    fn reset_failed(&self){}
    fn in_load_queue(&self) -> bool;

    fn eq(&self, other: &dyn UnitAction) -> bool;
    fn hash(&self) -> u64;
    fn as_any(&self) -> &dyn Any;
}