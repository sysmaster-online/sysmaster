use super::u_entry::{Unit, UnitObj};
use super::uu_config::UeConfig;
use crate::manager::data::{
    DataManager, UnitActiveState, UnitConfig, UnitConfigItem, UnitRelations, UnitType,
};
use crate::manager::unit::unit_base::UnitActionError;
use crate::manager::unit::unit_file::UnitFile;
use crate::manager::unit::unit_parser_mgr::{UnitConfigParser, UnitParserMgr};
use crate::manager::unit::UnitErrno;
use nix::sys::signal::Signal;
use nix::unistd::Pid;
use std::any::Any;
use std::error::Error;
use std::rc::Rc;

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnitX(Unit);

impl UnitX {
    pub fn init(&self) {}
    pub fn done(&self) {}
    pub fn load(&self) -> Result<(), Box<dyn Error>> {
        self.0.load_unit()
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
        self.0.in_load_queue()
    }

    pub fn set_in_load_queue(&self, t: bool) {
        self.0.set_in_load_queue(t);
    }
    pub fn dep_check(&self, _relation: UnitRelations, _other: &UnitX) -> Result<(), UnitErrno> {
        // unit_add_dependency: check input

        Ok(())
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
        self.0.get_id()
    }
    pub fn set_config(&self, _config: &UnitConfig) {
        let name = _config.get_name();
        if !name.is_empty() {
            let mut ue_config = UeConfig::new();
            ue_config.set(UnitConfigItem::UcItemName(name.to_string()));
            ue_config.set(UnitConfigItem::UcItemDesc((&_config.desc).to_string()));
            ue_config.set(UnitConfigItem::UcItemDoc(
                (&_config.documentation).to_string(),
            ));
            ue_config.set(UnitConfigItem::UcItemAllowIsolate(_config.allow_isolate));
            ue_config.set(UnitConfigItem::UcItemAllowIsolate(_config.allow_isolate));
            ue_config.set(UnitConfigItem::UcItemIgnoreOnIsolate(
                _config.ignore_on_isolate,
            ));
            ue_config.set(UnitConfigItem::UcItemOnSucJobMode(
                _config.on_success_job_mode,
            ));
            ue_config.set(UnitConfigItem::UcItemOnFailJobMode(
                _config.on_failure_job_mode,
            ));
            self.0.set_config(ue_config);
        }
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

    pub fn get_private_conf_section_name(&self) -> Option<String> {
        self.0.get_private_conf_section_name()
    }
    pub(in crate::manager::unit) fn new(
        dm: Rc<DataManager>,
        file: Rc<UnitFile>,
        unit_conf_mgr: Rc<UnitParserMgr<UnitConfigParser>>,
        unit_type: UnitType,
        name: &str,
        subclass: Box<dyn UnitObj>,
    ) -> UnitX {
        UnitX(Unit::new(
            Rc::clone(&dm),
            file,
            unit_conf_mgr,
            unit_type,
            name,
            subclass,
        ))
    }
}
