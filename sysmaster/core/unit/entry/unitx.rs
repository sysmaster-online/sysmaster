// Copyright (c) 2022 Huawei Technologies Co.,Ltd. All rights reserved.
//
// sysMaster is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan
// PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY
// KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

use super::uentry::Unit;
use super::UnitEmergencyAction;

use super::config::UeConfig;
use crate::unit::data::DataManager;
use crate::unit::rentry::{UnitLoadState, UnitRe};
use crate::unit::util::UnitFile;
use libutils::IN_SET;
use nix::sys::wait::WaitStatus;
use nix::unistd::Pid;
use std::ops::Deref;
use std::path::PathBuf;
use std::rc::Rc;
use sysmaster::error::*;
use sysmaster::rel::ReStation;
use sysmaster::unit::{SubUnit, UnitActiveState, UnitRelations, UnitType};

#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct UnitX(Rc<Unit>);

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
    #[allow(dead_code)]
    pub(crate) fn dump(&self) {}

    pub(in crate::unit) fn new(
        dmr: &Rc<DataManager>,
        rentryr: &Rc<UnitRe>,
        filer: &Rc<UnitFile>,
        unit_type: UnitType,
        name: &str,
        subclass: Box<dyn SubUnit>,
    ) -> UnitX {
        let unit = Unit::new(unit_type, name, dmr, rentryr, filer, subclass);
        UnitX(unit)
    }

    pub(in crate::unit) fn from_unit(unit: Rc<Unit>) -> UnitX {
        UnitX(unit)
    }

    #[allow(dead_code)]
    pub(crate) fn init(&self) {}
    #[allow(dead_code)]
    pub(crate) fn done(&self) {}
    #[allow(dead_code)]
    pub(crate) fn load(&self) -> Result<()> {
        self.0.load_unit()
    }
    #[allow(dead_code)]
    pub(crate) fn try_load(&self) -> Result<()> {
        // transaction_add_job_and_dependencies: bus_unit_validate_load_state + manager_unit_cache_should_retry_load + unit_load + bus_unit_validate_load_state
        todo!();
    }
    pub(crate) fn start(&self) -> Result<()> {
        log::debug!("unitx start the unit {}", self.id());
        self.0.start()
    }

    pub(crate) fn stop(&self, force: bool) -> Result<()> {
        self.0.stop(force)
    }
    pub(crate) fn reload(&self) -> Result<()> {
        todo!();
    }

    #[allow(dead_code)]
    pub(crate) fn kill(&self) {}
    #[allow(dead_code)]
    pub(crate) fn release_resources(&self) {}
    pub(crate) fn sigchld_events(&self, wait_status: WaitStatus) {
        self.0.sigchld_events(wait_status)
    }
    #[allow(dead_code)]
    pub(crate) fn reset_failed(&self) {}
    pub(crate) fn trigger(&self, _other: &Self) {}
    pub(crate) fn in_load_queue(&self) -> bool {
        self.0.in_load_queue()
    }

    pub(crate) fn set_in_load_queue(&self, t: bool) {
        self.0.set_in_load_queue(t);
    }

    pub(crate) fn in_target_dep_queue(&self) -> bool {
        self.0.in_target_dep_queue()
    }

    pub(crate) fn set_in_target_dep_queue(&self, t: bool) {
        self.0.set_in_target_dep_queue(t);
    }

    pub(crate) fn dep_check(&self, _relation: UnitRelations, _other: &UnitX) -> Result<()> {
        // unit_add_dependency: check input

        Ok(())
    }

    pub(in crate::unit) fn id(&self) -> &String {
        self.0.id()
    }

    // pub(in crate::manager::unit) fn get_config(&self, item: &UnitConfigItem) -> UnitConfigItem {
    //     self.0.get_config(item)
    // }

    pub(crate) fn get_success_action(&self) -> UnitEmergencyAction {
        self.0.get_success_action()
    }

    pub(crate) fn get_failure_action(&self) -> UnitEmergencyAction {
        self.0.get_failure_action()
    }

    pub(crate) fn get_start_limit_action(&self) -> UnitEmergencyAction {
        self.0.get_start_limit_action()
    }

    pub(crate) fn active_state(&self) -> UnitActiveState {
        //UnitActiveState::UnitActive
        self.0.current_active_state()
    }

    pub(crate) fn active_or_activating(&self) -> bool {
        IN_SET!(
            self.0.current_active_state(),
            UnitActiveState::UnitActive,
            UnitActiveState::UnitActivating,
            UnitActiveState::UnitReloading
        )
    }

    pub(crate) fn activated(&self) -> bool {
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

    #[allow(dead_code)]
    pub(crate) fn get_perpetual(&self) -> bool {
        todo!();
    }

    #[allow(dead_code)]
    pub(crate) fn can_start(&self) -> bool {
        todo!();
    }

    #[allow(dead_code)]
    pub(crate) fn can_stop(&self) -> bool {
        todo!();
    }

    #[allow(dead_code)]
    pub(crate) fn can_reload(&self) -> bool {
        todo!();
    }

    #[allow(dead_code)]
    pub(crate) fn is_load_complete(&self) -> bool {
        todo!();
    }

    pub(crate) fn cg_path(&self) -> PathBuf {
        self.0.cg_path()
    }

    pub(crate) fn load_state(&self) -> UnitLoadState {
        self.0.load_state()
    }

    pub(crate) fn unit_type(&self) -> UnitType {
        self.0.unit_type()
    }

    pub(crate) fn collect_fds(&self) -> Vec<i32> {
        self.0.collect_fds()
    }

    pub fn get_config(&self) -> Rc<UeConfig> {
        self.0.get_config()
    }

    pub(crate) fn default_dependencies(&self) -> bool {
        self.0.default_dependencies()
    }

    pub(crate) fn child_add_pids(&self, pid: Pid) {
        self.0.child_add_pids(pid);
    }

    pub(crate) fn child_remove_pids(&self, pid: Pid) {
        self.0.child_remove_pids(pid);
    }

    pub(crate) fn unit(&self) -> Rc<Unit> {
        Rc::clone(&self.0)
    }
}

impl Deref for UnitX {
    type Target = Rc<Unit>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
