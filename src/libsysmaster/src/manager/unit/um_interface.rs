use std::{rc::Rc, path::PathBuf};

use libevent::Events;
use nix::unistd::Pid;

use crate::manager::MngErrno;

use super::{
    ExecCmdError, ExecCommand, ExecContext, ExecParameters, Unit, UnitActionError,
    UnitDependencyMask, UnitRelationAtom, UnitRelations, UnitType,
};

///The trait Defining Shared Behavior of UnitManager
///
///The Behavior shared with all SubUnit,
///
/// if Subunit needs to obtain information about othes,
///
/// need create Self by attach a Impl UmIf
/// ```rust
/// fn -> new(owner Rc<&Impl Self>) -> Self{
/// }
/// ```
///
pub trait UmIf {
    fn get_dependency_list(&self, unit_name: &str, atom: UnitRelationAtom) -> Vec<Rc<Unit>>;

    fn unit_has_dependecy(&self, s_u_name: &str, atom: UnitRelationAtom, t_u_name: &str) -> bool;

    fn unit_add_dependency(
        &self,
        unit_name: &str,
        relation: UnitRelations,
        target_name: &str,
        add_ref: bool,
        mask: UnitDependencyMask,
    ) -> Result<(), UnitActionError>;

    fn load_unit_success(&self, name: &str) -> bool;

    fn unit_enabled(&self, name: &str) -> Result<(), UnitActionError>;

    fn has_stop_job(&self, name: &str) -> bool;

    fn relation_active_or_pending(&self, name: &str) -> bool;

    fn start_unit(&self, name: &str) -> Result<(), MngErrno>;

    ///
    fn load_unit(&self, name: &str) -> Option<Rc<Unit>>;

    /// why need events? need reconstruct
    fn events(&self) -> Rc<Events>;

 /// add pid and its correspond unit to
    fn child_watch_pid(&self, id: &str, pid: Pid);

    fn child_unwatch_pid(&self, id: &str, pid: Pid);

    /// add all the pid of unit id, read pids from cgroup path.
    fn child_watch_all_pids(&self, id: &str);

    fn rentry_trigger_merge(&self, unit_id: &String, force: bool);

    fn trigger_unit(&self, lunit: &str);

    fn exec_spawn(
        &self,
        unit: &Unit,
        cmdline: &ExecCommand,
        params: &ExecParameters,
        ctx: Rc<ExecContext>,
    ) -> Result<Pid, ExecCmdError>;

    /// return the notify path
    fn notify_socket(&self) -> Option<PathBuf>;

    fn same_unit_with_pid(&self, unit: &str, pid: Pid) -> bool;

     /// return the fds that trigger the unit {name};
    fn collect_socket_fds(&self, name: &str) -> Vec<i32>;

    ///
    fn units_get_all(&self, unit_type: Option<UnitType>) -> Vec<Rc<Unit>>;

    ///
    fn units_get(&self, name: &str) -> Option<Rc<Unit>>;

}
