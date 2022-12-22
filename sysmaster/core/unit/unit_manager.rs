///sysmaster entry
/// 1. Load all unit need loaded in a system
/// 2. Drive unit status through job engine;
/// 3. Mainlain all unit life cycle
///
///                    / ---->unit_load
/// ManagerX-> Manager | ---->job_manager
///                      ---->rentry
///
use super::super::job::{JobAffect, JobConf, JobKind, JobManager};
use super::execute::ExecSpawn;
use super::notify::NotifyManager;
use super::sigchld::Sigchld;
use super::unit_datastore::UnitDb;
use super::unit_entry::{Unit, UnitX};
use super::unit_load::UnitLoad;
use super::unit_rentry::{JobMode, UnitLoadState, UnitRe};
use super::unit_runtime::UnitRT;
use super::UnitRelationAtom;
use super::UnitRelations;
use crate::core::butil::table::{TableOp, TableSubscribe};
use crate::core::manager::pre_install::{Install, PresetMode};
use crate::core::unit::data::{DataManager, UnitState};
use libevent::Events;
use libutils::path_lookup::LookupPaths;
use libutils::proc_cmdline::get_process_cmdline;
use libutils::process_util;
use libutils::show_table::StatusItem;
use libutils::Result;
use nix::unistd::Pid;
use std::convert::TryFrom;
use std::io::Error;
use std::path::PathBuf;
use std::rc::Rc;
use sysmaster::execute::{ExecCmdError, ExecParameters};
use sysmaster::execute::{ExecCommand, ExecContext};
use sysmaster::reliability::{ReStation, ReStationKind, ReliLastFrame, Reliability};
use sysmaster::unit::{
    MngErrno, UmIf, UnitActionError, UnitActiveState, UnitDependencyMask, UnitType,
};
use unit_submanager::UnitSubManagers;

//#[derive(Debug)]
pub(in crate::core) struct UnitManagerX {
    dm: Rc<DataManager>,
    sub_name: String, // key for table-subscriber: UnitState
    data: Rc<UnitManager>,
    lookup_path: Rc<LookupPaths>,
}

impl Drop for UnitManagerX {
    fn drop(&mut self) {
        log::debug!("UnitManagerX drop, clear.");
        // repeating protection
        self.dm.clear();
    }
}

impl UnitManagerX {
    pub(in crate::core) fn new(
        eventr: &Rc<Events>,
        relir: &Rc<Reliability>,
        lookup_path: &Rc<LookupPaths>,
    ) -> UnitManagerX {
        let _dm = Rc::new(DataManager::new());
        let umx = UnitManagerX {
            dm: Rc::clone(&_dm),
            sub_name: String::from("UnitManagerX"),
            data: UnitManager::new(eventr, relir, &_dm, lookup_path),
            lookup_path: Rc::clone(lookup_path),
        };
        umx.register(&_dm, relir);
        umx
    }

    pub(in crate::core) fn register_ex(&self) {
        self.data.register_ex();
    }

    pub(crate) fn entry_clear(&self) {
        self.dm.entry_clear();
        self.data.entry_clear();
    }

    pub(in crate::core) fn entry_coldplug(&self) {
        self.data.entry_coldplug();
    }

    pub(in crate::core) fn start_unit(&self, name: &str) -> Result<(), MngErrno> {
        self.data.start_unit(name)
    }

    pub(in crate::core) fn stop_unit(&self, name: &str) -> Result<(), MngErrno> {
        self.data.stop_unit(name)
    }

    pub(in crate::core) fn get_unit_status(&self, name: &str) -> Result<String, MngErrno> {
        self.data.get_unit_status(name)
    }

    pub(in crate::core) fn child_sigchld_enable(&self, enable: bool) -> Result<i32> {
        self.data.sigchld.enable(enable)
    }

    pub(in crate::core) fn dispatch_load_queue(&self) {
        self.data.rt.dispatch_load_queue()
    }

    fn register(&self, dm: &DataManager, relir: &Reliability) {
        // dm-unit_state
        let subscriber = Rc::clone(&self.data);
        let ret = dm.register_unit_state(&self.sub_name, subscriber);
        assert!(ret.is_none());

        // reliability-station
        let station = Rc::clone(&self.data);
        let kind = ReStationKind::Level2;
        relir.station_register(&String::from("UnitManager"), kind, station);
    }

    pub(in crate::core) fn enable_unit(&self, unit_file: &str) -> Result<(), Error> {
        log::debug!("unit enable file {}", unit_file);
        let install = Install::new(PresetMode::Disable, self.lookup_path.clone());
        install.unit_enable_files(unit_file)?;
        Ok(())
    }

    pub(in crate::core) fn disable_unit(&self, unit_file: &str) -> Result<(), Error> {
        log::debug!("unit disable file {}", unit_file);
        let install = Install::new(PresetMode::Disable, self.lookup_path.clone());
        install.unit_disable_files(unit_file)?;
        Ok(())
    }
}

/// the struct for manager the unit instance
pub struct UnitManager {
    // associated objects
    events: Rc<Events>,
    reli: Rc<Reliability>,
    // owned objects
    rentry: Rc<UnitRe>,
    db: Rc<UnitDb>,
    rt: Rc<UnitRT>,
    load: UnitLoad,
    jm: Rc<JobManager>,
    exec: ExecSpawn,
    sigchld: Sigchld,
    notify: NotifyManager,
    sms: UnitSubManagers,
}

impl UmIf for UnitManager {
    /// check the unit s_u_name and t_u_name have atom relation
    fn unit_has_dependecy(&self, s_u_name: &str, atom: UnitRelationAtom, t_u_name: &str) -> bool {
        self.unit_has_dependecy(s_u_name, atom, t_u_name)
    }

    ///add a unit dependency to th unit deplist
    /// can called by sub unit
    /// sub unit add some default dependency
    ///
    fn unit_add_dependency(
        &self,
        unit_name: &str,
        relation: UnitRelations,
        target_name: &str,
        add_ref: bool,
        mask: UnitDependencyMask,
    ) -> Result<(), UnitActionError> {
        self.unit_add_dependency(unit_name, relation, target_name, add_ref, mask)
    }

    /// load the unit for reference name
    fn load_unit_success(&self, name: &str) -> bool {
        self.load_unit_success(name)
    }

    fn unit_enabled(&self, name: &str) -> Result<(), UnitActionError> {
        let u = if let Some(unit) = self.db.units_get(name) {
            unit
        } else {
            return Err(UnitActionError::UnitActionENoent);
        };

        if u.load_state() != UnitLoadState::UnitLoaded {
            log::error!("related service unit: {} is not loaded", name);
            return Err(UnitActionError::UnitActionENoent);
        }

        if u.activated() {
            return Err(UnitActionError::UnitActionEBusy);
        }

        Ok(())
    }

    fn has_stop_job(&self, name: &str) -> bool {
        self.has_stop_job(name)
    }
    /// check the unit that will be triggered by {name} is in active or activating state
    fn relation_active_or_pending(&self, name: &str) -> bool {
        self.relation_active_or_pending(name)
    }

    /// start the unit
    fn start_unit(&self, name: &str) -> Result<(), MngErrno> {
        self.start_unit(name)
    }

    fn events(&self) -> Rc<Events> {
        self.events()
    }

    fn child_unwatch_pid(&self, id: &str, pid: Pid) {
        self.child_unwatch_pid(id, pid)
    }

    fn rentry_trigger_merge(&self, unit_id: &str, force: bool) {
        self.jm.rentry_trigger_merge(unit_id, force)
    }

    ///
    fn trigger_unit(&self, lunit: &str) {
        self.jm.trigger_unit(lunit)
    }

    /// call the exec spawn to start the child service
    fn exec_spawn(
        &self,
        unit: &str,
        cmdline: &ExecCommand,
        params: &ExecParameters,
        ctx: Rc<ExecContext>,
    ) -> Result<Pid, ExecCmdError> {
        let unit = self.units_get(unit);
        if let Some(u) = unit {
            self.exec.spawn(&u, cmdline, params, ctx)
        } else {
            Err(ExecCmdError::SpawnError)
        }
    }

    fn child_watch_pid(&self, id: &str, pid: Pid) {
        self.child_watch_pid(id, pid)
    }

    fn child_watch_all_pids(&self, id: &str) {
        self.child_watch_all_pids(id)
    }

    fn notify_socket(&self) -> Option<PathBuf> {
        self.notify_socket()
    }

    fn same_unit_with_pid(&self, unit: &str, pid: Pid) -> bool {
        self.same_unit_with_pid(unit, pid)
    }

    fn collect_socket_fds(&self, name: &str) -> Vec<i32> {
        self.collect_socket_fds(name)
    }

    fn get_dependency_list(&self, _unit_name: &str, _atom: UnitRelationAtom) -> Vec<String> {
        self.get_dependency_list(_unit_name, _atom)
    }

    fn unit_has_default_dependecy(&self, _unit_name: &str) -> bool {
        let s_unit = if let Some(s_unit) = self.db.units_get(_unit_name) {
            s_unit
        } else {
            return false;
        };
        s_unit.default_dependencies()
    }

    fn units_get_all(&self, unit_type: Option<UnitType>) -> Vec<String> {
        self.units_get_all(unit_type)
    }

    fn current_active_state(&self, _unit_name: &str) -> UnitActiveState {
        let s_unit = if let Some(s_unit) = self.db.units_get(_unit_name) {
            s_unit
        } else {
            return UnitActiveState::UnitFailed;
        };
        s_unit.current_active_state()
    }

    fn unit_start(&self, _name: &str) -> Result<(), UnitActionError> {
        if let Some(unit) = self.db.units_get(_name) {
            unit.start()
        } else {
            Err(UnitActionError::UnitActionENoent)
        }
    }

    fn unit_stop(&self, _name: &str, force: bool) -> Result<(), UnitActionError> {
        if let Some(unit) = self.db.units_get(_name) {
            unit.stop(force)
        } else {
            Err(UnitActionError::UnitActionENoent)
        }
    }
}

/// the declaration "pub(self)" is for identification only.
impl UnitManager {
    /// add pid and its correspond unit to
    fn child_watch_pid(&self, id: &str, pid: Pid) {
        self.db.child_add_watch_pid(id, pid)
    }

    /// add all the pid of unit id, read pids from cgroup path.
    fn child_watch_all_pids(&self, id: &str) {
        self.db.child_watch_all_pids(id)
    }

    /// delete the pid from the db
    fn child_unwatch_pid(&self, id: &str, pid: Pid) {
        self.db.child_unwatch_pid(id, pid)
    }

    ///
    fn units_get(&self, name: &str) -> Option<Rc<Unit>> {
        self.db.units_get(name).map(|uxr| uxr.unit())
    }

    ///
    fn units_get_all(&self, unit_type: Option<UnitType>) -> Vec<String> {
        let units = self.db.units_get_all(unit_type);
        units
            .iter()
            .map(|uxr| uxr.unit().id().to_string())
            .collect::<Vec<_>>()
    }

    /// load the unit for reference name
    fn load_unit_success(&self, name: &str) -> bool {
        if let Some(_unit) = self.load_unitx(name) {
            return true;
        }

        false
    }

    /// check the unit s_u_name and t_u_name have atom relation
    fn unit_has_dependecy(&self, s_u_name: &str, atom: UnitRelationAtom, t_u_name: &str) -> bool {
        let s_unit = if let Some(s_unit) = self.db.units_get(s_u_name) {
            s_unit
        } else {
            return false;
        };

        let t_unit = if let Some(unit) = self.db.units_get(t_u_name) {
            unit
        } else {
            return false;
        };

        self.db.dep_is_dep_atom_with(&s_unit, atom, &t_unit)
    }

    ///add a unit dependency to th unit deplist
    /// can called by sub unit
    /// sub unit add some default dependency
    ///
    pub fn unit_add_dependency(
        &self,
        unit_name: &str,
        relation: UnitRelations,
        target_name: &str,
        add_ref: bool,
        mask: UnitDependencyMask,
    ) -> Result<(), UnitActionError> {
        let s_unit = if let Some(unit) = self.db.units_get(unit_name) {
            unit
        } else {
            return Err(UnitActionError::UnitActionENoent);
        };
        let t_unit = if let Some(unit) = self.db.units_get(target_name) {
            unit
        } else {
            return Err(UnitActionError::UnitActionENoent);
        };

        self.rt
            .unit_add_dependency(s_unit, relation, t_unit, add_ref, mask);
        Ok(())
    }

    /// get the unit the has atom relation with the unit
    fn get_dependency_list(&self, unit_name: &str, atom: UnitRelationAtom) -> Vec<String> {
        let s_unit = if let Some(unit) = self.db.units_get(unit_name) {
            unit
        } else {
            log::error!("unit [{}] not found!!!!!", unit_name);
            return Vec::new();
        };
        let dep_units = self.db.dep_gets_atom(&s_unit, atom);
        dep_units
            .iter()
            .map(|uxr| uxr.unit().id().to_string())
            .collect::<Vec<_>>()
    }

    /// check if there is already a stop job in process
    fn has_stop_job(&self, name: &str) -> bool {
        let u = if let Some(unit) = self.db.units_get(name) {
            unit
        } else {
            return false;
        };

        self.jm.has_stop_job(&u)
    }

    /// return the fds that trigger the unit {name};
    fn collect_socket_fds(&self, name: &str) -> Vec<i32> {
        let deps = self.db.dep_gets(name, UnitRelations::UnitTriggeredBy);
        let mut fds = Vec::new();
        for dep in deps.iter() {
            if dep.unit_type() != UnitType::UnitSocket {
                continue;
            }

            fds.extend(dep.collect_fds())
        }

        fds
    }

    /// check the unit that will be triggered by {name} is in active or activating state
    fn relation_active_or_pending(&self, name: &str) -> bool {
        let deps = self.db.dep_gets(name, UnitRelations::UnitTriggers);
        let mut pending: bool = false;
        for dep in deps.iter() {
            if dep.active_or_activating() {
                pending = true;
                break;
            }
        }

        pending
    }

    /// check the pid corresponding unit is the same with the unit
    fn same_unit_with_pid(&self, unit: &str, pid: Pid) -> bool {
        if !process_util::valid_pid(pid) {
            return false;
        }

        let p_unit = self.db.get_unit_by_pid(pid);
        if p_unit.is_none() {
            return false;
        }

        if p_unit.unwrap().id() == unit {
            return true;
        }

        false
    }

    fn start_unit(&self, name: &str) -> Result<(), MngErrno> {
        if let Some(unit) = self.load_unitx(name) {
            log::debug!("load unit {} success, send to job manager", name);
            self.jm.exec(
                &JobConf::new(&unit, JobKind::Start),
                JobMode::Replace,
                &mut JobAffect::new(false),
            )?;
            log::debug!("job exec success");
            Ok(())
        } else {
            Err(MngErrno::Internal)
        }
    }

    /// return the notify path
    fn notify_socket(&self) -> Option<PathBuf> {
        self.notify.notify_sock()
    }

    ///
    pub fn events(&self) -> Rc<Events> {
        Rc::clone(&self.events)
    }

    ///
    pub fn reliability(&self) -> Rc<Reliability> {
        Rc::clone(&self.reli)
    }

    #[allow(dead_code)]
    pub(in crate::core) fn get_unit_by_pid(&self, pid: Pid) -> Option<Rc<UnitX>> {
        self.db.get_unit_by_pid(pid)
    }

    pub(self) fn stop_unit(&self, name: &str) -> Result<(), MngErrno> {
        if let Some(unit) = self.load_unitx(name) {
            self.jm.exec(
                &JobConf::new(&unit, JobKind::Stop),
                JobMode::Replace,
                &mut JobAffect::new(false),
            )?;
            Ok(())
        } else {
            Err(MngErrno::Internal)
        }
    }

    fn get_unit_status_active_state(&self, active_state: UnitActiveState) -> String {
        match active_state {
            UnitActiveState::UnitActive => String::from("active"),
            UnitActiveState::UnitActivating => String::from("activating"),
            UnitActiveState::UnitDeActivating => String::from("deactivating"),
            UnitActiveState::UnitFailed => String::from("failed"),
            UnitActiveState::UnitInActive => String::from("inactive"),
            UnitActiveState::UnitMaintenance => String::from("maintenance"),
            UnitActiveState::UnitReloading => String::from("reloading"),
            #[allow(unreachable_patterns)]
            _ => String::from("unknown"),
        }
    }

    fn get_unit_cgroup_path(&self, unit: Rc<Unit>) -> String {
        let res = match unit.cg_path().to_str() {
            Some(res) => res.to_string(),
            None => String::new(),
        };
        if res.is_empty() {
            return "Empty cgroup path".to_string();
        }
        res
    }

    fn get_unit_status_pids(&self, unit: Rc<Unit>) -> String {
        let pids = unit.get_pids();
        if pids.is_empty() {
            return "No process".to_string();
        }
        let mut res = String::new();
        for pid in pids.iter() {
            if !res.is_empty() {
                res += "\n";
            }
            res += &pid.to_string();
            res += " ";
            res += get_process_cmdline(pid).as_str();
        }
        res
    }

    pub(self) fn get_unit_status(&self, name: &str) -> Result<String, MngErrno> {
        let unit = match self.units_get(name) {
            Some(unit) => unit,
            None => {
                return Err(MngErrno::NotExisted);
            }
        };
        let status_list = vec![
            StatusItem::new(
                "Loaded".to_string(),
                self.load_unit_success(name).to_string(),
            ),
            StatusItem::new(
                "Active".to_string(),
                self.get_unit_status_active_state(unit.current_active_state()),
            ),
            StatusItem::new(
                "CGroup".to_string(),
                self.get_unit_cgroup_path(unit.clone()),
            ),
            StatusItem::new("PID".to_string(), self.get_unit_status_pids(unit)),
        ];
        let mut status_table = tabled::Table::new(status_list);
        status_table
            .with(tabled::style::Style::empty()) // don't show the border
            .with(tabled::Disable::row(tabled::object::Rows::first())) // remove the first row
            .with(
                tabled::Modify::new(tabled::object::Columns::first()) // modify the first column
                    .with(tabled::format::Format::new(|s| format!("{}:", s))) // add ":"
                    .with(tabled::Alignment::right()),
            ); // align to right
        Ok("● ".to_string() + name + "\n" + &status_table.to_string())
    }

    pub(self) fn new(
        eventr: &Rc<Events>,
        relir: &Rc<Reliability>,
        dmr: &Rc<DataManager>,
        lookup_path: &Rc<LookupPaths>,
    ) -> Rc<UnitManager> {
        let _rentry = Rc::new(UnitRe::new(relir));
        let _db = Rc::new(UnitDb::new(&_rentry));
        let _rt = Rc::new(UnitRT::new(relir, &_rentry, &_db));
        let _jm = Rc::new(JobManager::new(eventr, relir, &_db));
        let um = Rc::new(UnitManager {
            events: Rc::clone(eventr),
            reli: Rc::clone(relir),
            rentry: Rc::clone(&_rentry),
            load: UnitLoad::new(dmr, &_rentry, &_db, &_rt, lookup_path),
            db: Rc::clone(&_db),
            rt: Rc::clone(&_rt),
            jm: Rc::clone(&_jm),
            exec: ExecSpawn::new(),
            sigchld: Sigchld::new(eventr, relir, &_db, &_jm),
            notify: NotifyManager::new(eventr, relir, &_rentry, &_db, &_jm),
            sms: UnitSubManagers::new(relir),
        });
        um.load.set_um(&um);
        um.sms.set_um(&um);
        um
    }

    fn load_unitx(&self, name: &str) -> Option<Rc<UnitX>> {
        self.load.load_unit(name)
    }
}

// inert states need jm,so put here
impl TableSubscribe<String, UnitState> for UnitManager {
    fn notify(&self, op: &TableOp<String, UnitState>) {
        match op {
            TableOp::TableInsert(name, config) => self.insert_states(name, config),
            TableOp::TableRemove(name, _) => self.remove_states(name),
        }
    }
}

impl UnitManager {
    fn insert_states(&self, source: &str, state: &UnitState) {
        log::debug!("insert unit states source {}, state: {:?}", source, state);
        let unitx = if let Some(u) = self.db.units_get(source) {
            u
        } else {
            return;
        };

        if let Err(_e) = self.jm.try_finish(&unitx, state.os, state.ns, state.flags) {
            // debug
        }

        let atom = UnitRelationAtom::UnitAtomTriggeredBy;
        for other in self.db.dep_gets_atom(&unitx, atom) {
            other.trigger(&unitx);
        }
    }

    fn remove_states(&self, _source: &str) {
        todo!();
    }
}

impl ReStation for UnitManager {
    // input
    fn input_rebuild(&self) {
        // sigchld
        self.sigchld.input_rebuild();

        // sub-manager
        self.sms.input_rebuild();
    }

    // compensate
    fn db_compensate_last(&self, lframe: (u32, Option<u32>, Option<u32>), lunit: Option<&String>) {
        let (frame, _, _) = lframe;
        if let Ok(f) = ReliLastFrame::try_from(frame) {
            match f {
                ReliLastFrame::Queue => self.rt.db_compensate_last(lframe, lunit),
                ReliLastFrame::JobManager => self.jm.db_compensate_last(lframe, lunit),
                ReliLastFrame::SigChld => self.sigchld.db_compensate_last(lframe, lunit),
                ReliLastFrame::CgEvent => todo!(),
                ReliLastFrame::Notify => self.notify.db_compensate_last(lframe, lunit),
                ReliLastFrame::SubManager => self.sms.db_compensate_last(lframe, lunit),
                _ => {} // not concerned, do nothing
            };
        }
    }

    fn db_compensate_history(&self) {
        // queue: do nothing

        // job
        self.jm.db_compensate_history();

        // sig-child: do nothing

        // cg-event: do nothing

        // notify: do nothing
    }

    fn do_compensate_last(&self, lframe: (u32, Option<u32>, Option<u32>), lunit: Option<&String>) {
        let (frame, _, _) = lframe;
        if let Ok(f) = ReliLastFrame::try_from(frame) {
            match f {
                ReliLastFrame::Queue => self.rt.do_compensate_last(lframe, lunit),
                ReliLastFrame::JobManager => self.jm.do_compensate_last(lframe, lunit),
                ReliLastFrame::SigChld => self.sigchld.do_compensate_last(lframe, lunit),
                ReliLastFrame::CgEvent => todo!(),
                ReliLastFrame::Notify => self.notify.do_compensate_last(lframe, lunit),
                ReliLastFrame::SubManager => self.sms.do_compensate_last(lframe, lunit),
                _ => {} // not concerned, do nothing
            };
        }
    }

    fn do_compensate_others(&self, lunit: Option<&String>) {
        // queue: do nothing

        // job
        self.jm.do_compensate_others(lunit);

        // sig-child: do nothing

        // cg-event: do nothing

        // notify: do nothing
    }

    // data
    fn db_map(&self) {
        // unit_datastore(with unit_entry)
        /* unit-sets with unit_entry */
        for unit_id in self.rentry.base_keys().iter() {
            let unit = self.load.try_new_unit(unit_id).unwrap();
            unit.db_map();
            self.db.units_insert(unit_id.clone(), unit);
        }
        /* others: unit-dep and unit-child */
        self.db.db_map_excl_units();

        // rt
        self.rt.db_map();

        // job
        self.jm.db_map();

        // notify
        self.notify.db_map();

        // sub-manager
        self.sms.db_map();
    }

    // reload
    fn register_ex(&self) {
        // notify
        self.notify.register_ex();

        // sub-manager
        self.sms.enumerate();
    }

    fn entry_coldplug(&self) {
        for unit in self.db.units_get_all(None).iter() {
            // unit
            unit.entry_coldplug();

            // job
            self.jm.coldplug_unit(unit);
        }
    }

    fn entry_clear(&self) {
        // job
        self.jm.entry_clear();

        // rt
        self.rt.entry_clear();

        // db
        self.db.entry_clear();
    }
}

/// the trait used for translate to UnitObj
/*pub trait UnitSubClass: SubUnit + UnitMngUtil {
    /// the method of translate to UnitObj
    fn into_unitobj(self: Box<Self>) -> Box<dyn SubUnit>;
}*/

mod unit_submanager {
    use crate::core::plugin::Plugin;

    use super::UnitManager;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::convert::TryFrom;
    use std::rc::{Rc, Weak};
    use sysmaster::reliability::Reliability;
    use sysmaster::unit::{UnitManagerObj, UnitType};

    #[allow(dead_code)]
    pub(super) struct UnitSubManagers {
        reli: Rc<Reliability>,
        um: RefCell<Weak<UnitManager>>,
        db: RefCell<HashMap<UnitType, Box<dyn UnitManagerObj>>>,
    }

    impl UnitSubManagers {
        pub(super) fn new(relir: &Rc<Reliability>) -> UnitSubManagers {
            UnitSubManagers {
                reli: Rc::clone(relir),
                um: RefCell::new(Weak::new()),
                db: RefCell::new(HashMap::new()),
            }
        }

        pub(super) fn set_um(&self, um: &Rc<UnitManager>) {
            // update um
            self.um.replace(Rc::downgrade(um));

            // fill all unit-types
            for ut in 0..UnitType::UnitTypeMax as u32 {
                self.add_sub(UnitType::try_from(ut).ok().unwrap());
            }
        }

        pub(super) fn enumerate(&self) {
            for (_, sub) in self.db.borrow().iter() {
                sub.enumerate();
            }
        }

        pub(super) fn input_rebuild(&self) {
            for (_, sub) in self.db.borrow().iter() {
                sub.input_rebuild();
            }
        }

        pub(super) fn db_map(&self) {
            for (_, sub) in self.db.borrow().iter() {
                sub.db_map();
            }
        }

        pub(super) fn db_compensate_last(
            &self,
            lframe: (u32, Option<u32>, Option<u32>),
            lunit: Option<&String>,
        ) {
            let utype = self.last_unittype(lframe);
            if utype.is_none() || lunit.is_none() {
                return;
            }

            let unit_type = utype.unwrap();
            if let Some(sub) = self.db.borrow().get(&unit_type) {
                sub.db_compensate_last(lframe, lunit);
            }
        }

        pub(super) fn do_compensate_last(
            &self,
            lframe: (u32, Option<u32>, Option<u32>),
            lunit: Option<&String>,
        ) {
            let utype = self.last_unittype(lframe);
            if utype.is_none() || lunit.is_none() {
                return;
            }

            let unit_type = utype.unwrap();
            if let Some(sub) = self.db.borrow().get(&unit_type) {
                sub.do_compensate_last(lframe, lunit);
            }
        }

        fn add_sub(&self, unit_type: UnitType) {
            assert!(!self.db.borrow().contains_key(&unit_type));

            let sub = self.new_sub(unit_type);
            if let Some(s) = sub {
                self.db.borrow_mut().insert(unit_type, s);
            }
        }

        fn new_sub(&self, unit_type: UnitType) -> Option<Box<dyn UnitManagerObj>> {
            let um = self.um();
            let ret = Plugin::get_instance().create_um_obj(unit_type);
            if ret.is_err() {
                log::info!("create um_obj is not found, type {:?}!", unit_type);
                return None;
            }

            let sub = ret.unwrap();
            let reli = um.reliability();
            sub.attach_um(um);
            sub.attach_reli(reli);
            Some(sub)
        }

        fn last_unittype(&self, lframe: (u32, Option<u32>, Option<u32>)) -> Option<UnitType> {
            let (_, utype, _) = lframe;
            utype?;

            let ut = utype.unwrap();
            if ut > UnitType::UnitTypeMax as u32 {
                // error
                return None;
            }

            Some(UnitType::try_from(ut).ok().unwrap())
        }

        fn um(&self) -> Rc<UnitManager> {
            self.um.clone().into_inner().upgrade().unwrap()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::manager::rentry::RELI_HISTORY_MAX_DBS;
    use crate::core::mount::mount_setup;
    use libevent::Events;
    use libutils::logger;
    use nix::errno::Errno;
    use nix::sys::signal::Signal;
    use std::thread;
    use std::time::Duration;
    use sysmaster::unit::UnitActiveState;

    fn init_dm_for_test() -> (Rc<DataManager>, Rc<Events>, Rc<UnitManager>) {
        logger::init_log_with_console("manager test", 4);
        let mut l_path = LookupPaths::new();
        l_path.init_lookup_paths();
        let lookup_path = Rc::new(l_path);

        let event = Rc::new(Events::new().unwrap());
        let dm = Rc::new(DataManager::new());
        let reli = Rc::new(Reliability::new(RELI_HISTORY_MAX_DBS));
        let um = UnitManager::new(&event, &reli, &dm, &lookup_path);
        (dm, event, um)
    }

    #[allow(dead_code)]
    fn setup_mount_point() -> Result<(), Errno> {
        mount_setup::mount_setup()?;

        Ok(())
    }

    #[test]
    fn test_service_unit_load() {
        logger::init_log_with_console("test_service_unit_load", 4);
        let dm = init_dm_for_test();
        let unit_name = String::from("config.service");
        let unit = dm.2.load_unitx(&unit_name);

        match unit {
            Some(_unit_obj) => assert_eq!(_unit_obj.id(), "config.service"),
            None => println!("test unit load, not found unit: {}", unit_name),
        };
    }

    // #[test]
    #[allow(dead_code)]
    fn test_service_unit_start() {
        let ret = setup_mount_point();
        if ret.is_err() {
            return;
        }

        logger::init_log_with_console("test_service_unit_start", 4);
        let dm = init_dm_for_test();
        let unit_name = String::from("config.service");
        let unit = dm.2.load_unitx(&unit_name);

        assert!(unit.is_some());
        let u = unit.unwrap();

        let ret = u.start();
        assert!(ret.is_ok());

        log::debug!("unit start end!");
        let ret = u.stop(false);
        assert!(ret.is_ok());
        log::debug!("unit stop end!");
    }

    // #[test]
    #[allow(dead_code)]
    fn test_socket_unit_start_and_stop() {
        logger::init_log_with_console("test_socket_unit_start_stop", 4);

        let ret = setup_mount_point();
        if ret.is_err() {
            return;
        }

        let dm = init_dm_for_test();

        let unit_name = String::from("test.socket");
        let unit = dm.2.load_unitx(&unit_name);

        assert!(unit.is_some());
        let u = unit.unwrap();

        let ret = u.start();
        log::debug!("socket start ret is: {:?}", ret);
        assert!(ret.is_ok());

        thread::sleep(Duration::from_secs(4));
        u.sigchld_events(Pid::from_raw(-1), 0, Signal::SIGCHLD);
        assert_eq!(u.active_state(), UnitActiveState::UnitActive);

        let ret = u.stop(false);
        log::debug!("socket stop ret is: {:?}", ret);
        assert!(ret.is_ok());

        thread::sleep(Duration::from_secs(4));
        assert_eq!(u.active_state(), UnitActiveState::UnitDeActivating);
        u.sigchld_events(Pid::from_raw(-1), 0, Signal::SIGCHLD);

        assert_eq!(u.active_state(), UnitActiveState::UnitInActive);
    }

    #[test]
    fn test_service_unit_start_conflicts() {
        let dm = init_dm_for_test();
        let conflict_unit_name = String::from("conflict.service");
        let confilict_unit = dm.2.start_unit(&conflict_unit_name);

        assert!(confilict_unit.is_ok());
    }

    #[test]
    fn test_units_load() {
        logger::init_log_with_console("test_units_load", 4);

        let dm = init_dm_for_test();
        let mut unit_name_lists: Vec<String> = Vec::new();

        unit_name_lists.push("config.service".to_string());
        // unit_name_lists.push("testsunit.target".to_string());
        for u_name in unit_name_lists.iter() {
            let unit = dm.2.load_unitx(u_name);

            match unit {
                Some(_unit_obj) => assert_eq!(_unit_obj.id(), u_name),
                None => println!("test unit load, not found unit: {}", u_name),
            };
        }
    }
    #[test]
    fn test_target_unit_load() {
        logger::init_log_with_console("test_target_unit_load", 4);
        let dm = init_dm_for_test();
        let mut unit_name_lists: Vec<String> = Vec::new();

        unit_name_lists.push("testsunit.target".to_string());
        // unit_name_lists.push("testsunit.target".to_string());
        for u_name in unit_name_lists.iter() {
            let unit = dm.2.load_unitx(u_name);
            match unit {
                Some(_unit_obj) => {
                    println!(
                        "{:?}",
                        _unit_obj.get_config().config_data().borrow().Unit.Requires
                    );
                    assert_eq!(_unit_obj.id(), u_name);
                }
                None => println!("test unit load, not found unit: {}", u_name),
            };
        }
    }
}
