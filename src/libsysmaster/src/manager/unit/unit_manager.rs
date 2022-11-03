use super::execute::{ExecCmdError, ExecParameters, ExecSpawn};
use super::job::{JobAffect, JobConf, JobKind, JobManager};
use super::notify::NotifyManager;
use super::sigchld::Sigchld;
use super::unit_base::{UnitDependencyMask, UnitRelationAtom};
use super::unit_datastore::UnitDb;
use super::unit_entry::{Unit, UnitObj, UnitX};
use super::unit_rentry::{ExecCommand, JobMode, UnitLoadState, UnitRe, UnitType};
use super::unit_runtime::UnitRT;
use super::{ExecContext, UnitActionError};
use crate::manager::pre_install::{Install, PresetMode};
use crate::manager::rentry::ReliLastFrame;
use crate::manager::table::{TableOp, TableSubscribe};
use crate::manager::unit::data::{DataManager, UnitState};
use crate::manager::{MngErrno, UnitRelations};
use crate::plugin::Plugin;
use crate::reliability::{ReStation, ReStationKind, Reliability};
use libevent::Events;
use libutils::path_lookup::LookupPaths;
use libutils::process_util;
use libutils::Result;
use nix::unistd::Pid;
use std::convert::TryFrom;
use std::io::Error;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use unit_load::UnitLoad;
use unit_submanager::UnitSubManagers;

//#[derive(Debug)]
pub(in crate::manager) struct UnitManagerX {
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
    pub(in crate::manager) fn new(
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

    pub(in crate::manager) fn register_ex(&self) {
        self.data.register_ex();
    }

    pub(in crate::manager) fn entry_clear(&self) {
        self.dm.entry_clear();
        self.data.entry_clear();
    }

    pub(in crate::manager) fn entry_coldplug(&self) {
        self.data.entry_coldplug();
    }

    pub(in crate::manager) fn enumerate(&self) {
        self.data.sms.enumerate()
    }

    pub(in crate::manager) fn start_unit(&self, name: &str) -> Result<(), MngErrno> {
        self.data.start_unit(name)
    }

    pub(in crate::manager) fn stop_unit(&self, name: &str) -> Result<(), MngErrno> {
        self.data.stop_unit(name)
    }

    pub(in crate::manager) fn child_sigchld_enable(&self, enable: bool) -> Result<i32> {
        self.data.sigchld.enable(enable)
    }

    pub(in crate::manager) fn dispatch_load_queue(&self) {
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

    pub(in crate::manager) fn enable_unit(&self, unit_file: &str) -> Result<(), Error> {
        log::debug!("unit disable file {}", unit_file);
        let install = Install::new(PresetMode::Disable, self.lookup_path.clone());
        install.unit_enable_files(unit_file)?;
        Ok(())
    }

    pub(in crate::manager) fn disable_unit(&self, unit_file: &str) -> Result<(), Error> {
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
    plugins: Arc<Plugin>,

    // owned objects
    rentry: Rc<UnitRe>,
    sms: UnitSubManagers,
    db: Rc<UnitDb>,
    rt: Rc<UnitRT>,
    load: UnitLoad,
    jm: Rc<JobManager>,
    exec: ExecSpawn,
    sigchld: Sigchld,
    notify: NotifyManager,
}

/// the declaration "pub(self)" is for identification only.
impl UnitManager {
    ///
    pub fn rentry_trigger_merge(&self, unit_id: &String, force: bool) {
        self.jm.rentry_trigger_merge(unit_id, force)
    }

    ///
    pub fn trigger_unit(&self, lunit: &str) {
        let unit = self.db.units_get(lunit).unwrap();
        let cnt = self.jm.run(Some(&unit));
        assert_ne!(cnt, 0); // something must be triggered
    }

    /// add pid and its correspond unit to
    pub fn child_watch_pid(&self, id: &str, pid: Pid) {
        self.db.child_add_watch_pid(id, pid)
    }

    /// add all the pid of unit id, read pids from cgroup path.
    pub fn child_watch_all_pids(&self, id: &str) {
        self.db.child_watch_all_pids(id)
    }

    /// delete the pid from the db
    pub fn child_unwatch_pid(&self, id: &str, pid: Pid) {
        self.db.child_unwatch_pid(id, pid)
    }

    ///
    pub fn units_get(&self, name: &str) -> Option<Rc<Unit>> {
        self.db.units_get(name).map(|uxr| uxr.unit())
    }

    ///
    pub fn units_get_all(&self, unit_type: Option<UnitType>) -> Vec<Rc<Unit>> {
        let units = self.db.units_get_all(unit_type);
        units.iter().map(|uxr| uxr.unit()).collect::<Vec<_>>()
    }

    /// call the exec spawn to start the child service
    pub fn exec_spawn(
        &self,
        unit: &Unit,
        cmdline: &ExecCommand,
        params: &ExecParameters,
        ctx: Rc<ExecContext>,
    ) -> Result<Pid, ExecCmdError> {
        self.exec.spawn(unit, cmdline, params, ctx)
    }

    ///
    pub fn load_unit(&self, name: &str) -> Option<Rc<Unit>> {
        self.load_unitx(name).map(|uxr| uxr.unit())
    }

    /// load the unit for reference name
    pub fn load_unit_success(&self, name: &str) -> bool {
        if let Some(_unit) = self.load_unitx(name) {
            return true;
        }

        false
    }

    /// load the unit of the dependency UnitType
    pub fn load_related_unit_success(&self, name: &str, unit_type: UnitType) -> bool {
        let stem_name = Path::new(name).file_stem().unwrap().to_str().unwrap();
        let relate_name = format!("{}.{}", stem_name, String::from(unit_type));

        if let Some(_unit) = self.load_unitx(&relate_name) {
            return true;
        }

        false
    }

    /// check the unit active state of of reference name
    pub fn unit_enabled(&self, name: &str) -> Result<(), UnitActionError> {
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

    /// check the unit s_u_name and t_u_name have atom relation
    pub fn unit_has_dependecy(
        &self,
        s_u_name: &str,
        atom: UnitRelationAtom,
        t_u_name: &str,
    ) -> bool {
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
    pub fn get_dependency_list(&self, unit_name: &str, atom: UnitRelationAtom) -> Vec<Rc<Unit>> {
        let s_unit = if let Some(unit) = self.db.units_get(unit_name) {
            unit
        } else {
            log::error!("unit [{}] not found!!!!!", unit_name);
            return Vec::new();
        };

        let dep_units = self.rt.get_dependency_list(&s_unit, atom);
        dep_units.iter().map(|uxr| uxr.unit()).collect::<Vec<_>>()
    }

    /// check if there is already a stop job in process
    pub fn has_stop_job(&self, name: &str) -> bool {
        let u = if let Some(unit) = self.db.units_get(name) {
            unit
        } else {
            return false;
        };

        self.jm.has_stop_job(&u)
    }

    /// return the fds that trigger the unit {name};
    pub fn collect_socket_fds(&self, name: &str) -> Vec<i32> {
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
    pub fn relation_active_or_pending(&self, name: &str) -> bool {
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
    pub fn same_unit_with_pid(&self, unit: &str, pid: Pid) -> bool {
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

    /// start the unit
    pub fn start_unit(&self, name: &str) -> Result<(), MngErrno> {
        if let Some(unit) = self.load_unitx(name) {
            log::debug!("load unit success, send to job manager");
            self.jm.exec(
                &JobConf::new(Rc::clone(&unit), JobKind::Start),
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
    pub fn notify_socket(&self) -> Option<PathBuf> {
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
    pub(in crate::manager) fn get_unit_by_pid(&self, pid: Pid) -> Option<Rc<UnitX>> {
        self.db.get_unit_by_pid(pid)
    }

    pub(self) fn stop_unit(&self, name: &str) -> Result<(), MngErrno> {
        if let Some(unit) = self.load_unitx(name) {
            self.jm.exec(
                &JobConf::new(Rc::clone(&unit), JobKind::Stop),
                JobMode::Replace,
                &mut JobAffect::new(false),
            )?;
            Ok(())
        } else {
            Err(MngErrno::Internal)
        }
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
            plugins: Plugin::get_instance(),
            sms: UnitSubManagers::new(relir),
            rentry: Rc::clone(&_rentry),
            load: UnitLoad::new(dmr, &_rentry, &_db, &_rt, lookup_path),
            db: Rc::clone(&_db),
            rt: Rc::clone(&_rt),
            jm: Rc::clone(&_jm),
            exec: ExecSpawn::new(),
            sigchld: Sigchld::new(eventr, relir, &_db, &_jm),
            notify: NotifyManager::new(eventr, relir, &_rentry, &_db, &_jm),
        });
        um.sms.set_um(&um);
        um.load.set_um(&um);
        um
    }

    fn load_unitx(&self, name: &str) -> Option<Rc<UnitX>> {
        self.load.load_unit(name)
    }
}

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

/// the trait used for attach UnitManager to sub unit
pub trait UnitMngUtil {
    /// the method of attach to UnitManager to sub unit
    fn attach_um(&self, um: Rc<UnitManager>);

    /// the method of attach to Reliability to sub unit
    fn attach_reli(&self, reli: Rc<Reliability>);
}

///The trait Defining Shared Behavior of sub unit-manager
pub trait UnitManagerObj: UnitMngUtil + ReStation {
    ///
    /* repeatable */
    fn enumerate_perpetual(&self) {}
    ///
    /* repeatable */
    fn enumerate(&self) {}
    ///
    fn shutdown(&self) {}
}

/// #[macro_use]
/// the macro for create a sub unit-manager instance
#[macro_export]
macro_rules! declure_umobj_plugin {
    ($unit_type:ty, $constructor:path, $name:expr, $level:expr) => {
        // method for create the sub-unit-manager instance
        #[no_mangle]
        pub fn __um_obj_create() -> *mut dyn $crate::manager::UnitManagerObj {
            logger::init_log_with_default($name, $level);
            let construcotr: fn() -> $unit_type = $constructor;

            let obj = construcotr();
            let boxed: Box<dyn $crate::manager::UnitManagerObj> = Box::new(obj);
            Box::into_raw(boxed)
        }
    };
}

/// the trait used for translate to UnitObj
pub trait UnitSubClass: UnitObj + UnitMngUtil {
    /// the method of translate to UnitObj
    fn into_unitobj(self: Box<Self>) -> Box<dyn UnitObj>;
}

/// #[macro_use]
/// the macro for create a sub unit instance
#[macro_export]
macro_rules! declure_unitobj_plugin {
    ($unit_type:ty, $constructor:path, $name:expr, $level:expr) => {
        /// method for create the unit instance
        #[no_mangle]
        pub fn __unit_obj_create() -> *mut dyn $crate::manager::UnitSubClass {
            logger::init_log_with_default($name, $level);
            let construcotr: fn() -> $unit_type = $constructor;

            let obj = construcotr();
            let boxed: Box<dyn $crate::manager::UnitSubClass> = Box::new(obj);
            Box::into_raw(boxed)
        }
    };
}

mod unit_submanager {
    use super::{UnitManager, UnitManagerObj};
    use crate::manager::unit::unit_rentry::UnitType;
    use crate::reliability::Reliability;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::convert::TryFrom;
    use std::rc::{Rc, Weak};

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
            if utype.is_none() {
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
            if utype.is_none() {
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
            let ret = um.plugins.create_um_obj(unit_type);
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

mod unit_load {
    use libutils::path_lookup::LookupPaths;

    use super::UnitManager;
    use crate::manager::table::{TableOp, TableSubscribe};
    use crate::manager::unit::data::{DataManager, UnitDepConf};
    use crate::manager::unit::uload_util::UnitFile;
    use crate::manager::unit::unit_datastore::UnitDb;
    use crate::manager::unit::unit_entry::UnitX;
    use crate::manager::unit::unit_rentry::{self, UnitRe, UnitType};
    use crate::manager::unit::unit_runtime::UnitRT;
    use std::cell::RefCell;
    use std::rc::{Rc, Weak};

    //#[derive(Debug)]
    pub(super) struct UnitLoad {
        sub_name: String, // key for table-subscriber: UnitDepConf
        data: Rc<UnitLoadData>,
    }

    impl UnitLoad {
        pub(super) fn new(
            dmr: &Rc<DataManager>,
            rentryr: &Rc<UnitRe>,
            dbr: &Rc<UnitDb>,
            rtr: &Rc<UnitRT>,
            lookup_path: &Rc<LookupPaths>,
        ) -> UnitLoad {
            let load = UnitLoad {
                sub_name: String::from("UnitLoad"),
                data: Rc::new(UnitLoadData::new(dmr, rentryr, dbr, rtr, lookup_path)),
            };
            load.register(dmr);
            load
        }

        pub(super) fn load_unit(&self, name: &str) -> Option<Rc<UnitX>> {
            self.data.load_unit(name)
        }

        pub(super) fn set_um(&self, um: &Rc<UnitManager>) {
            self.data.set_um(um);
        }

        pub(super) fn try_new_unit(&self, name: &str) -> Option<Rc<UnitX>> {
            self.data.try_new_unit(name)
        }

        fn register(&self, dm: &DataManager) {
            let subscriber = Rc::clone(&self.data);
            let ret = dm.register_ud_config(&self.sub_name, subscriber);
            assert!(ret.is_none())
        }
    }

    //#[derive(Debug)]
    struct UnitLoadData {
        // associated objects
        dm: Rc<DataManager>,
        rentry: Rc<UnitRe>,
        um: RefCell<Weak<UnitManager>>,
        db: Rc<UnitDb>,
        rt: Rc<UnitRT>,

        // owned objects
        file: Rc<UnitFile>,
    }

    // the declaration "pub(self)" is for identification only.
    impl UnitLoadData {
        pub(self) fn new(
            dmr: &Rc<DataManager>,
            rentryr: &Rc<UnitRe>,
            dbr: &Rc<UnitDb>,
            rtr: &Rc<UnitRT>,
            lookup_path: &Rc<LookupPaths>,
        ) -> UnitLoadData {
            log::debug!("UnitLoadData db count is {}", Rc::strong_count(dbr));
            let file = Rc::new(UnitFile::new(lookup_path));
            UnitLoadData {
                dm: Rc::clone(dmr),
                rentry: Rc::clone(rentryr),
                um: RefCell::new(Weak::new()),
                db: Rc::clone(dbr),
                rt: Rc::clone(rtr),
                file: Rc::clone(&file),
            }
        }

        pub(self) fn prepare_unit(&self, name: &str) -> Option<Rc<UnitX>> {
            match self.try_new_unit(name) {
                Some(unit) => {
                    self.db.units_insert(name.to_string(), Rc::clone(&unit));
                    self.rt.push_load_queue(Rc::clone(&unit));
                    Some(Rc::clone(&unit))
                }
                None => {
                    log::error!(
                        "create unit obj failed,name is {},{}",
                        name,
                        Rc::strong_count(&self.db)
                    );
                    None
                }
            }
        }

        pub(self) fn push_dep_unit_into_load_queue(&self, name: &str) -> Option<Rc<UnitX>> {
            if let Some(unit) = self.db.units_get(name) {
                return Some(Rc::clone(&unit));
            };

            self.prepare_unit(name)
        }

        pub(self) fn load_unit(&self, name: &str) -> Option<Rc<UnitX>> {
            if let Some(unit) = self.db.units_get(name) {
                return Some(Rc::clone(&unit));
            };
            let unit = self.prepare_unit(name)?;
            log::info!("begin dispatch unit in  load queue");
            self.rt.dispatch_load_queue();
            Some(Rc::clone(&unit))
        }

        pub(self) fn set_um(&self, um: &Rc<UnitManager>) {
            self.um.replace(Rc::downgrade(um));
        }

        pub(self) fn try_new_unit(&self, name: &str) -> Option<Rc<UnitX>> {
            let unit_type = unit_rentry::unit_name_to_type(name);
            if unit_type == UnitType::UnitTypeInvalid {
                return None;
            }

            log::info!(
                "begin create obj for type {:?}, name {} by plugin",
                unit_type,
                name
            );
            let um = self.um();
            let subclass = match um.plugins.create_unit_obj(unit_type) {
                Ok(sub) => sub,
                Err(_e) => {
                    log::error!("Failed to create unit_obj!{}", _e);
                    return None;
                }
            };

            let reli = um.reliability();
            subclass.attach_um(um);
            subclass.attach_reli(reli);

            Some(Rc::new(UnitX::new(
                &self.dm,
                &self.rentry,
                &self.file,
                unit_type,
                name,
                subclass.into_unitobj(),
            )))
        }

        fn um(&self) -> Rc<UnitManager> {
            self.um.clone().into_inner().upgrade().unwrap()
        }
    }

    impl TableSubscribe<String, UnitDepConf> for UnitLoadData {
        fn notify(&self, op: &TableOp<String, UnitDepConf>) {
            match op {
                TableOp::TableInsert(name, config) => self.insert_udconf(name, config),
                TableOp::TableRemove(_, _) => {} // self.remove_udconf(name)
            }
        }
    }

    impl UnitLoadData {
        fn insert_udconf(&self, name: &str, config: &UnitDepConf) {
            //hash map insert return is old value,need reconstruct
            let unit = match self.db.units_get(name) {
                Some(u) => u,
                None => {
                    log::error!("create unit obj error in unit manager");
                    return;
                } // load
            };

            // dependency
            for (relation, list) in config.deps.iter() {
                for o_name in list {
                    let tmp_unit: Rc<UnitX>;
                    if let Some(o_unit) = self.push_dep_unit_into_load_queue(o_name) {
                        //can not call unit_load directly, will be nested.
                        tmp_unit = Rc::clone(&o_unit);
                    } else {
                        log::error!("create unit obj error in unit manager");
                        return;
                    }

                    if let Err(_e) =
                        self.db
                            .dep_insert(Rc::clone(&unit), *relation, tmp_unit, true, 0)
                    //insert the dependency, but not judge loaded success, if loaded failed, whether record the dependency.
                    {
                        log::debug!("add dependency relation failed for source unit is {},dependency unit is {}",unit.id(),o_name);
                        return;
                    }
                }
            }
        }

        #[allow(dead_code)]
        fn remove_udconf(&self, _source: &str) {
            todo!();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manager::rentry::RELI_HISTORY_MAX_DBS;
    use crate::manager::unit::data::UnitActiveState;
    use crate::mount::mount_setup;
    use libevent::Events;
    use libutils::logger;
    use nix::errno::Errno;
    use nix::sys::signal::Signal;
    use std::thread;
    use std::time::Duration;

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
