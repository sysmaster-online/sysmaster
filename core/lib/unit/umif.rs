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

//! UnitManager interfaces
use super::{UnitDependencyMask, UnitRelationAtom, UnitRelations};
use crate::error::*;
use crate::exec::{ExecCommand, ExecContext, ExecParameters};
use crate::rel::{ReStation, Reliability};
use crate::unit::{UnitActiveState, UnitType};
use event::Events;
use nix::unistd::Pid;
use std::{path::PathBuf, rc::Rc};

///The trait Defining Shared Behavior of UnitManager
///
///The Behavior shared with all SubUnit,
///
/// if SubUnit needs to obtain information about othes,
///
/// need create Self by attach a Impl UmIf
///
pub trait UmIf {
    /// get the unit the has atom relation with the unit
    fn get_dependency_list(&self, _unit_name: &str, _atom: UnitRelationAtom) -> Vec<String> {
        Vec::new()
    }

    /// judge the unit has default dependency
    fn unit_has_default_dependecy(&self, _unit_name: &str) -> bool {
        false
    }
    /// check the unit s_u_name and t_u_name have atom relation
    fn unit_has_dependecy(
        &self,
        _s_u_name: &str,
        _atom: UnitRelationAtom,
        _t_u_name: &str,
    ) -> bool {
        false
    }

    ///add a unit dependency to th unit deplist
    /// can called by sub unit
    /// sub unit add some default dependency
    ///
    fn unit_add_dependency(
        &self,
        _unit_name: &str,
        _relation: UnitRelations,
        _target_name: &str,
        _add_ref: bool,
        _mask: UnitDependencyMask,
    ) -> Result<()> {
        Ok(())
    }

    ///add two unit dependency to the unit
    /// can called by sub unit
    /// sub unit add some default dependency
    ///
    fn unit_add_two_dependency(
        &self,
        _unit_name: &str,
        _ra: UnitRelations,
        _rb: UnitRelations,
        _target_name: &str,
        _add_ref: bool,
        _mask: UnitDependencyMask,
    ) -> Result<()> {
        Ok(())
    }

    /// load the unit for reference name
    fn load_unit_success(&self, _name: &str) -> bool {
        false
    }
    /// check the unit active state of of reference name
    fn unit_enabled(&self, _name: &str) -> Result<()> {
        Ok(())
    }
    /// check if there is already a stop job in process
    fn has_stop_job(&self, _name: &str) -> bool {
        false
    }

    /// check the unit that will be triggered by {name} is in active or activating state
    fn relation_active_or_pending(&self, _name: &str) -> bool {
        false
    }
    /// starting a unit by pushing it to job queue
    fn unit_start_by_job(&self, _name: &str) -> Result<()> {
        Ok(())
    }

    /// starting a unit by calling unit.start()
    fn unit_start_directly(&self, _name: &str) -> Result<()> {
        Ok(())
    }

    /// call the unit stop function
    fn unit_stop(&self, _name: &str, _force: bool) -> Result<()> {
        Ok(())
    }
    /// why need events? need reconstruct
    fn events(&self) -> Rc<Events> {
        Rc::new(Events::new().unwrap())
    }

    /// add pid and its correspond unit to
    fn child_watch_pid(&self, _id: &str, _pid: Pid) {}
    /// delete the pid from the db
    fn child_unwatch_pid(&self, _id: &str, _pid: Pid) {}

    /// add all the pid of unit id, read pids from cgroup path.
    fn child_watch_all_pids(&self, _id: &str) {}

    ///rentry_trigger_merge
    fn rentry_trigger_merge(&self, _unit_id: &str, _force: bool) {}

    /// call the exec spawn to start the child service
    fn trigger_unit(&self, _lunit: &str) {}

    /// call the exec spawn to start the child service
    fn exec_spawn(
        &self,
        _unit_name: &str,
        _cmdline: &ExecCommand,
        _params: &ExecParameters,
        _ctx: Rc<ExecContext>,
    ) -> Result<Pid> {
        Ok(Pid::this())
    }

    /// return the notify path
    fn notify_socket(&self) -> Option<PathBuf> {
        None
    }
    /// check the pid corresponding unit is the same with the unit
    fn same_unit_with_pid(&self, _unit: &str, _pid: Pid) -> bool {
        false
    }

    /// return the fds that trigger the unit {name};
    fn collect_socket_fds(&self, _name: &str) -> Vec<i32> {
        Vec::new()
    }

    /// get all unit in sysmaster
    fn units_get_all(&self, _unit_type: Option<UnitType>) -> Vec<String> {
        Vec::new()
    }

    /// check the unit active state of of reference name
    fn current_active_state(&self, _unit_name: &str) -> UnitActiveState {
        UnitActiveState::UnitFailed
    }

    /// get the subunit state of one unit. i.e. running(service), listening(socket)
    fn get_subunit_state(&self, _unit_name: &str) -> String {
        String::new()
    }

    /// add restart job
    fn restart_unit(&self, _unit_name: &str, _is_manual: bool) -> Result<()> {
        Ok(())
    }

    /// get the log file config
    fn get_log_file(&self) -> &str {
        ""
    }

    /// get the log target config
    fn get_log_target(&self) -> &str {
        ""
    }
}

/// the trait used for attach UnitManager to sub unit
pub trait UnitMngUtil {
    /// the method of attach to UnitManager to sub unit
    fn attach_um(&self, um: Rc<dyn UmIf>);

    /// the method of attach to Reliability to sub unit
    fn attach_reli(&self, reli: Rc<Reliability>);
}

///The trait Defining Shared Behavior of sub unit-manager
pub trait UnitManagerObj: UnitMngUtil + ReStation {
    ///
    fn enumerate_perpetual(&self) {}
    ///
    fn enumerate(&self) {
        self.register_ex();
    }
    ///
    fn shutdown(&self) {}
}

/// #[macro_use]
/// the macro for create a sub unit-manager instance
#[macro_export]
macro_rules! declure_umobj_plugin {
    ($unit_type:ty, $constructor:path, $name:expr) => {
        // method for create the sub-unit-manager instance
        use log::LevelFilter;
        #[no_mangle]
        pub fn __um_obj_create(
            level: LevelFilter,
            target: &str,
            file: &str,
        ) -> *mut dyn $crate::unit::UnitManagerObj {
            logger::init_log_for_subum($name, level, target, file);
            let construcotr: fn() -> $unit_type = $constructor;
            let obj = construcotr();
            let boxed: Box<dyn $crate::unit::UnitManagerObj> = Box::new(obj);
            Box::into_raw(boxed)
        }
    };
}
