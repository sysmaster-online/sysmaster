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
use basic::time::UnitTimeStamp;
use event::Events;
use nix::unistd::Pid;
use std::cell::RefCell;
use std::{path::PathBuf, rc::Rc};

#[cfg(feature = "plugin")]
use constants::LOG_FILE_PATH;

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
    fn unit_has_dependency(
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

    /// check if there is already a job in process
    fn has_job(&self, _name: &str) -> bool {
        false
    }

    /// check if there is already a stop job in process
    fn has_stop_job(&self, _name: &str) -> bool {
        false
    }

    /// check if there is already a start job in process
    fn has_start_job(&self, _name: &str) -> bool {
        false
    }

    /// check the unit that will be triggered by {name} is in active or activating state
    fn relation_active_or_pending(&self, _name: &str) -> bool {
        false
    }

    /// Destroy Runtime Data, only RuntimeDirectory for now
    fn unit_destroy_runtime_data(&self, _runtime_directory: Vec<PathBuf>) -> Result<()> {
        Ok(())
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

    /// remove all the pid of unit id
    fn child_unwatch_all_pids(&self, _id: &str) {}

    ///rentry_trigger_merge
    fn rentry_trigger_merge(&self, _unit_id: &str, _force: bool) {}

    /// call the exec spawn to start the child service
    fn trigger_unit(&self, _lunit: &str) {}

    /// get trigger by id
    fn unit_get_trigger(&self, _id: &str) -> String {
        String::new()
    }

    /// Tests whether the unit to trigger is loaded
    fn test_trigger_loaded(&self, _id: &str) -> bool {
        false
    }

    /// call the exec spawn to start the child service
    fn exec_spawn(
        &self,
        _unit_name: &str,
        _cmdline: &ExecCommand,
        _params: &mut ExecParameters,
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
        UnitActiveState::Failed
    }

    /// get the subunit state of one unit. i.e. running(service), listening(socket)
    fn get_subunit_state(&self, _unit_name: &str) -> String {
        String::new()
    }

    /// add restart job
    fn restart_unit(&self, _unit_name: &str, _is_manual: bool) -> Result<()> {
        Ok(())
    }

    ///
    fn private_section(&self, _unit_type: UnitType) -> String {
        null_str!("")
    }

    /* ========== ONLY VALID IN SERVICE ========== */
    /// set the service's socket fd
    fn service_set_socket_fd(&self, _service_name: &str, _fd: i32) {}

    /// release the service's socket fd
    fn service_release_socket_fd(&self, _service_name: &str, _fd: i32) {}

    /* ========== ONLY VALID IN MOUNT ========== */
    /// setup existing mount
    fn setup_existing_mount(
        &self,
        _unit_name: &str,
        _what: &str,
        _mount_where: &str,
        _options: &str,
        _fstype: &str,
    ) {
    }

    /// setup new mount
    fn setup_new_mount(
        &self,
        _unit_name: &str,
        _what: &str,
        _mount_where: &str,
        _options: &str,
        _fstype: &str,
    ) {
    }

    /// update mount state
    fn update_mount_state_by_mountinfo(&self, _unit_name: &str) {}

    /// get unit timestamp data
    fn get_unit_timestamp(&self, _unit_name: &str) -> Rc<RefCell<UnitTimeStamp>> {
        Rc::new(RefCell::new(UnitTimeStamp::default()))
    }
    /// unit trigger TriggerBy units
    fn trigger_notify(&self, _name: &str) {}
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
    ///
    fn private_section(&self, _unit_type: UnitType) -> String {
        null_str!("")
    }
    ///
    fn can_transient(&self, _unit_type: UnitType) -> bool {
        false
    }
}

/// the macro for create a sub unit-manager instance
#[macro_export]
macro_rules! declare_umobj_plugin {
    ($unit_type:ty, $constructor:path) => {
        use log::Level;
        /// method for create the sub-unit-manager instance
        #[cfg_attr(feature = "plugin", no_mangle)]
        pub fn __um_obj_create(
            level: Level,
            target: &str,
            file_size: u32,
            file_number: u32,
        ) -> *mut dyn $crate::unit::UnitManagerObj {
            #[cfg(feature = "plugin")]
            log::init_log(
                PLUGIN_NAME,
                level,
                target.split(&[' ', '-'][..]).collect(),
                LOG_FILE_PATH,
                file_size,
                file_number,
                false,
            );
            let construcotr: fn() -> $unit_type = $constructor;
            let obj = construcotr();
            let boxed: Box<dyn $crate::unit::UnitManagerObj> = Box::new(obj);
            Box::into_raw(boxed)
        }
    };
}
