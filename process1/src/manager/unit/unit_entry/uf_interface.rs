use super::u_entry::{Unit, UnitObj};
use crate::manager::data::{
    DataManager, UnitActiveState, UnitConfig, UnitConfigItem, UnitRelations, UnitType,
};
use crate::manager::unit::unit_base::UnitActionError;
use crate::manager::unit::unit_file::UnitFile;
use crate::manager::unit::UnitErrno;
use nix::sys::signal::Signal;
use nix::unistd::Pid;
use std::any::Any;
use std::error::Error;
use std::rc::Rc;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnitX(Unit);

impl UnitX {
    pub fn init(&self) {}
    pub fn done(&self) {}
    pub fn load(&self) -> Result<(), Box<dyn Error>> {
        todo!()
    }
    pub fn try_load(&self) -> Result<(), UnitActionError> {
        // transaction_add_job_and_dependencies: bus_unit_validate_load_state + manager_unit_cache_should_retry_load + unit_load + bus_unit_validate_load_state
        todo!();
    }
    pub fn coldplug(&self) {}
    pub fn dump(&self) {}
    pub fn start(&self) -> Result<(), UnitActionError> {
        todo!();
    }
    pub fn stop(&self) -> Result<(), UnitActionError> {
        todo!();
    }
    pub fn reload(&self) -> Result<(), UnitActionError> {
        todo!();
    }

    pub fn kill(&self) {}
    pub fn check_gc(&self) -> bool {
        todo!();
    }
    pub fn release_resources(&self) {}
    pub fn check_snapshot(&self) {}
    pub fn sigchld_events(&self, _pid: Pid, _code: i32, _status: Signal) {
        todo!()
    }
    pub fn reset_failed(&self) {}
    pub fn trigger(&self, _other: &Self) {}
    pub fn in_load_queue(&self) -> bool {
        //self.in_load_queue()
        todo!()
    }
    pub fn dep_check(&self, _relation: UnitRelations, _other: &UnitX) -> Result<(), UnitErrno> {
        // unit_add_dependency: check input
        todo!()
    }
    pub fn eq(&self, _other: &UnitX) -> bool {
        todo!();
    }
    pub fn hash(&self) -> u64 {
        todo!();
    }
    pub fn as_any(&self) -> &dyn Any {
        todo!();
    }

    pub fn get_id(&self) -> &str {
        todo!();
    }
    pub fn set_config(&self, _config: &UnitConfig) {
        // get and compare each item, only the changed item needs to be set
        todo!()
    }
    pub fn get_config(&self, _item: &UnitConfigItem) -> UnitConfigItem {
        todo!();
    }

    pub fn get_state(&self) -> UnitActiveState {
        todo!();
    }
    pub fn get_perpetual(&self) -> bool {
        todo!();
    }
    pub fn can_start(&self) -> bool {
        todo!();
    }
    pub fn can_stop(&self) -> bool {
        todo!();
    }
    pub fn can_reload(&self) -> bool {
        todo!();
    }
    pub fn is_load_complete(&self) -> bool {
        todo!();
    }

    pub(in crate::manager::unit) fn new(
        dm: Rc<DataManager>,
        file: Rc<UnitFile>,
        unit_type: UnitType,
        name: &str,
        subclass: Box<dyn UnitObj>,
    ) -> UnitX {
        UnitX(Unit::new(Rc::clone(&dm), file, unit_type, name, subclass))
    }
}
