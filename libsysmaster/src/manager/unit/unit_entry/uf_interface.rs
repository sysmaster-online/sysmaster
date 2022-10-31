use super::u_entry::{Unit, UnitObj};
use super::uu_config::UeConfig;
use crate::manager::unit::data::{DataManager, UnitActiveState};
use crate::manager::unit::uload_util::UnitFile;
use crate::manager::unit::unit_base::UnitActionError;
use crate::manager::unit::unit_rentry::{UnitLoadState, UnitRe, UnitRelations, UnitType};
use crate::manager::unit::UnitErrno;
use crate::reliability::ReStation;
use libutils::IN_SET;
use nix::sys::signal::Signal;
use nix::unistd::Pid;
use std::error::Error;
use std::ops::Deref;
use std::path::PathBuf;
use std::rc::Rc;

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(in crate::manager) struct UnitX(Rc<Unit>);

impl ReStation for UnitX {
    // no input, no compensate

    // data
    fn db_map(&self) {
        self.0.db_map();
    }

    // reload: entry-only
    fn entry_coldplug(&self) {
        self.0.entry_coldplug();
    }

    fn entry_clear(&self) {
        self.0.entry_clear();
    }
}

impl UnitX {
    pub(in crate::manager) fn dump(&self) {}

    pub(in crate::manager::unit) fn new(
        dmr: &Rc<DataManager>,
        rentryr: &Rc<UnitRe>,
        filer: &Rc<UnitFile>,
        unit_type: UnitType,
        name: &str,
        subclass: Box<dyn UnitObj>,
    ) -> UnitX {
        let unit = Unit::new(unit_type, name, dmr, rentryr, filer, subclass);
        UnitX(unit)
    }

    pub(in crate::manager::unit) fn init(&self) {}
    pub(in crate::manager::unit) fn done(&self) {}
    pub(in crate::manager::unit) fn load(&self) -> Result<(), Box<dyn Error>> {
        self.0.load_unit()
    }
    pub(in crate::manager::unit) fn try_load(&self) -> Result<(), UnitActionError> {
        // transaction_add_job_and_dependencies: bus_unit_validate_load_state + manager_unit_cache_should_retry_load + unit_load + bus_unit_validate_load_state
        todo!();
    }
    pub(in crate::manager::unit) fn start(&self) -> Result<(), UnitActionError> {
        log::debug!("unitx start the unit {}", self.id());
        self.0.start()
    }

    pub(in crate::manager::unit) fn stop(&self, force: bool) -> Result<(), UnitActionError> {
        self.0.stop(force)
    }
    pub(in crate::manager::unit) fn reload(&self) -> Result<(), UnitActionError> {
        todo!();
    }

    pub(in crate::manager::unit) fn kill(&self) {}
    pub(in crate::manager::unit) fn release_resources(&self) {}
    pub(in crate::manager::unit) fn sigchld_events(&self, pid: Pid, code: i32, signal: Signal) {
        self.0.sigchld_events(pid, code, signal)
    }
    pub(in crate::manager::unit) fn reset_failed(&self) {}
    pub(in crate::manager::unit) fn trigger(&self, _other: &Self) {}
    pub(in crate::manager::unit) fn in_load_queue(&self) -> bool {
        self.0.in_load_queue()
    }

    pub(in crate::manager::unit) fn set_in_load_queue(&self, t: bool) {
        self.0.set_in_load_queue(t);
    }

    pub(in crate::manager::unit) fn in_target_dep_queue(&self) -> bool {
        self.0.in_target_dep_queue()
    }

    pub(in crate::manager::unit) fn set_in_target_dep_queue(&self, t: bool) {
        self.0.set_in_target_dep_queue(t);
    }

    pub(in crate::manager::unit) fn dep_check(
        &self,
        _relation: UnitRelations,
        _other: &UnitX,
    ) -> Result<(), UnitErrno> {
        // unit_add_dependency: check input

        Ok(())
    }

    pub(in crate::manager::unit) fn id(&self) -> &String {
        self.0.id()
    }

    // pub(in crate::manager::unit) fn get_config(&self, item: &UnitConfigItem) -> UnitConfigItem {
    //     self.0.get_config(item)
    // }

    pub(in crate::manager::unit) fn active_state(&self) -> UnitActiveState {
        //UnitActiveState::UnitActive
        self.0.current_active_state()
    }

    pub(in crate::manager::unit) fn active_or_activating(&self) -> bool {
        IN_SET!(
            self.0.current_active_state(),
            UnitActiveState::UnitActive,
            UnitActiveState::UnitActivating,
            UnitActiveState::UnitReloading
        )
    }

    pub(in crate::manager::unit) fn activated(&self) -> bool {
        // the unit is in activating or activated.
        if IN_SET!(
            self.0.current_active_state(),
            UnitActiveState::UnitInActive,
            UnitActiveState::UnitFailed,
            UnitActiveState::UnitActivating
        ) {
            return false;
        }

        true
    }

    pub(in crate::manager::unit) fn get_perpetual(&self) -> bool {
        todo!();
    }
    pub(in crate::manager::unit) fn can_start(&self) -> bool {
        todo!();
    }
    pub(in crate::manager::unit) fn can_stop(&self) -> bool {
        todo!();
    }
    pub(in crate::manager::unit) fn can_reload(&self) -> bool {
        todo!();
    }
    pub(in crate::manager::unit) fn is_load_complete(&self) -> bool {
        todo!();
    }

    pub(in crate::manager::unit) fn cg_path(&self) -> PathBuf {
        self.0.cg_path()
    }

    pub(in crate::manager::unit) fn load_state(&self) -> UnitLoadState {
        self.0.load_state()
    }

    pub(in crate::manager::unit) fn unit_type(&self) -> UnitType {
        self.0.unit_type()
    }

    pub(in crate::manager::unit) fn collect_fds(&self) -> Vec<i32> {
        self.0.collect_fds()
    }

    pub fn get_config(&self) -> Rc<UeConfig> {
        self.0.get_config()
    }

    pub(in crate::manager::unit) fn default_dependencies(&self) -> bool {
        self.0.default_dependencies()
    }

    pub(in crate::manager::unit) fn child_add_pids(&self, pid: Pid) {
        self.0.child_add_pids(pid);
    }

    pub(in crate::manager::unit) fn child_remove_pids(&self, pid: Pid) {
        self.0.child_remove_pids(pid);
    }

    pub(in crate::manager::unit) fn unit(&self) -> Rc<Unit> {
        Rc::clone(&self.0)
    }
}

impl Deref for UnitX {
    type Target = Rc<Unit>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
