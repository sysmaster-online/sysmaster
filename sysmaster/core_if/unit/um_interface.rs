use std::{path::PathBuf, rc::Rc};

use libevent::Events;
use nix::unistd::Pid;
/// error number of manager
#[derive(Debug)]
pub enum MngErrno {
    /// invalid input
    Input,
    /// not existed
    NotExisted,
    /// Internal error
    Internal,
    /// not supported
    NotSupported,
}


use crate::reliability::{Reliability, ReStation};

use super::{
    ExecCmdError, ExecCommand, ExecContext, ExecParameters};
use super::{Unit,UnitDependencyMask, UnitRelationAtom, UnitRelations, UnitType,};
use super::{UnitActionError,};

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
    fn get_dependency_list(&self, _unit_name: &str, _atom: UnitRelationAtom) -> Vec<Rc<Unit>> {
        Vec::new()
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
    ) -> Result<(), UnitActionError> {
        Ok(())
    }
    /// load the unit for reference name
    fn load_unit_success(&self, _name: &str) -> bool {
        false
    }
    /// check the unit active state of of reference name
    fn unit_enabled(&self, _name: &str) -> Result<(), UnitActionError> {
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
    /// start the unit
    fn start_unit(&self, _name: &str) -> Result<(), MngErrno> {
        Ok(())
    }

    ///
    fn load_unit(&self, _name: &str) -> Option<Rc<Unit>> {
        None
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
        _unit: &Unit,
        _cmdline: &ExecCommand,
        _params: &ExecParameters,
        _ctx: Rc<ExecContext>,
    ) -> Result<Pid, ExecCmdError> {
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

    ///
    fn units_get_all(&self, _unit_type: Option<UnitType>) -> Vec<Rc<Unit>> {
        Vec::new()
    }

    ///
    fn units_get(&self, _name: &str) -> Option<Rc<Unit>> {
        None
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