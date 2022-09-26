use super::execute::{ExecCmdError, ExecCommand, ExecParameters, ExecSpawn};
use super::job::{JobAffect, JobConf, JobKind, JobManager};
use super::unit_base::{JobMode, UnitDependencyMask, UnitLoadState, UnitRelationAtom};
use super::unit_datastore::UnitDb;
use super::unit_entry::{Unit, UnitObj, UnitX};
use super::unit_runtime::UnitRT;
use super::{ExecContext, UnitActionError, UnitType};
use crate::manager::data::{DataManager, UnitState};
use crate::manager::manager_config::ManagerConfig;
use crate::manager::table::{TableOp, TableSubscribe};
use crate::manager::{MngErrno, UnitActiveState, UnitRelations};
use event::{EventState, Events, Source};
use libmount::mountinfo;
use nix::sys::socket::UnixCredentials;
use nix::unistd::Pid;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use unit_load::UnitLoad;

use utils::error::Error as ServiceError;
use utils::process_util;

//#[derive(Debug)]
pub(in crate::manager) struct UnitManagerX {
    sub_name: String, // key for table-subscriber: UnitState
    data: Rc<UnitManager>,
}

impl UnitManagerX {
    pub(in crate::manager) fn new(
        dmr: &Rc<DataManager>,
        eventr: &Rc<Events>,
        configm: &Rc<ManagerConfig>,
    ) -> UnitManagerX {
        let umx = UnitManagerX {
            sub_name: String::from("UnitManagerX"),
            data: UnitManager::new(dmr, eventr, configm),
        };
        umx.register(dmr);
        umx
    }

    pub(in crate::manager) fn start_unit(&self, name: &str) -> Result<(), MngErrno> {
        self.data.start_unit(name)
    }

    pub(in crate::manager) fn stop_unit(&self, name: &str) -> Result<(), MngErrno> {
        self.data.stop_unit(name)
    }

    pub(in crate::manager) fn child_dispatch_sigchld(&self) -> Result<(), Box<dyn Error>> {
        self.data.db.child_dispatch_sigchld()
    }

    pub(in crate::manager) fn dispatch_mountinfo(&self) -> Result<(), MngErrno> {
        self.data.dispatch_mountinfo()
    }

    pub(in crate::manager) fn dispatch_load_queue(&self) {
        self.data.rt.dispatch_load_queue()
    }

    fn register(&self, dm: &DataManager) {
        let subscriber = Rc::clone(&self.data);
        let register_result = dm.register_unit_state(&self.sub_name, subscriber);
        if let Some(_r) = register_result {
            log::info!("TableSubcribe for {} is already register", &self.sub_name);
        } else {
            log::info!("register  TableSubcribe for {}  successful", &self.sub_name);
        }
    }

    pub(crate) fn notify_message(
        &self,
        ucred: &UnixCredentials,
        messages: &HashMap<&str, &str>,
        fds: &Vec<i32>,
    ) -> Result<(), ServiceError> {
        self.data.notify_message(ucred, messages, fds)
    }
}

//#[derive(Debug)]
pub struct UnitManager {
    db: Rc<UnitDb>,
    rt: Rc<UnitRT>,
    load: UnitLoad,
    jm: JobManager,
    exec: ExecSpawn,
    events: Rc<Events>,
    config: Rc<ManagerConfig>,
}

fn mount_point_to_unit_name(mount_point: &str) -> String {
    let mut res = String::from(mount_point).replace('/', "-") + ".mount";
    if res != "-.mount" {
        res = String::from(&res[1..])
    }
    res
}

// the declaration "pub(self)" is for identification only.
impl UnitManager {
    pub fn child_watch_pid(&self, pid: Pid, id: &str) {
        self.db.child_add_watch_pid(pid, id)
    }

    pub fn child_watch_all_pids(&self, id: &str) {
        self.db.child_watch_all_pids(id)
    }

    pub fn child_unwatch_pid(&self, pid: Pid) {
        self.db.child_unwatch_pid(pid)
    }

    pub fn exec_spawn(
        &self,
        unit: &Unit,
        cmdline: &ExecCommand,
        params: &ExecParameters,
        ctx: Rc<ExecContext>,
    ) -> Result<Pid, ExecCmdError> {
        self.exec.spawn(unit, cmdline, params, ctx.clone())
    }

    // load the unit for reference name
    pub fn load_unit_success(&self, name: &str) -> bool {
        if let Some(_unit) = self.load_unit(name) {
            return true;
        }

        return false;
    }

    // load the unit of the dependency UnitType
    pub fn load_related_unit_success(&self, name: &str, unit_type: UnitType) -> bool {
        let stem_name = Path::new(name).file_stem().unwrap().to_str().unwrap();
        let relate_name = format!("{}.{}", stem_name, String::from(unit_type));

        if let Some(_unit) = self.load_unit(&relate_name) {
            return true;
        }

        return false;
    }

    // check the unit active state of of reference name
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

        return Ok(());
    }

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

    pub fn get_dependency_list(&self, unit_name: &str, atom: UnitRelationAtom) -> Vec<Rc<Unit>> {
        let mut ret_list: Vec<Rc<Unit>> = Vec::new();
        let s_unit = if let Some(unit) = self.db.units_get(unit_name) {
            unit
        } else {
            log::error!("unit [{}] not found!!!!!", unit_name);
            return ret_list;
        };

        let dep_units = self.rt.get_dependency_list(&s_unit, atom);
        for unit_x in dep_units {
            let a = Rc::clone(&*(*unit_x));
            ret_list.push(a);
        }
        ret_list
    }

    pub fn register(&self, source: Rc<dyn Source>) {
        self.events.add_source(source).unwrap();
    }

    pub fn enable(&self, source: Rc<dyn Source>, state: EventState) {
        self.events.set_enabled(source, state).unwrap();
    }

    pub fn unregister(&self, source: Rc<dyn Source>) {
        self.events.del_source(source).unwrap();
    }

    // check if there is already a stop job in process
    pub fn has_stop_job(&self, name: &str) -> bool {
        let u = if let Some(unit) = self.db.units_get(name) {
            unit
        } else {
            return false;
        };

        self.jm.has_stop_job(&u)
    }

    // return the fds that trigger the unit {name};
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

    // check the unit that will be triggered by {name} is in active or activating state
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

    // check the pid corresponding unit is the same with the unit
    pub fn same_unit_with_pid(&self, unit: &str, pid: Pid) -> bool {
        if !process_util::valid_pid(pid) {
            return false;
        }

        let p_unit = self.db.get_unit_by_pid(pid);
        if p_unit.is_none() {
            return false;
        }

        if p_unit.unwrap().get_id() == unit {
            return true;
        }

        false
    }

    pub fn start_unit(&self, name: &str) -> Result<(), MngErrno> {
        if let Some(unit) = self.load_unit(name) {
            log::debug!("load unit success, send to job manager");
            self.jm.exec(
                &JobConf::new(Rc::clone(&unit), JobKind::JobStart),
                JobMode::JobReplace,
                &mut JobAffect::new(false),
            )?;
            log::debug!("job exec success");
            Ok(())
        } else {
            return Err(MngErrno::MngErrInternel);
        }
    }

    pub fn notify_socket(&self) -> Option<PathBuf> {
        self.config.notify_sock()
    }

    pub(in crate::manager) fn get_unit_by_pid(&self, pid: Pid) -> Option<Rc<UnitX>> {
        self.db.get_unit_by_pid(pid)
    }

    pub(self) fn stop_unit(&self, name: &str) -> Result<(), MngErrno> {
        if let Some(unit) = self.load_unit(name) {
            self.jm.exec(
                &JobConf::new(Rc::clone(&unit), JobKind::JobStop),
                JobMode::JobReplace,
                &mut JobAffect::new(false),
            )?;
            Ok(())
        } else {
            return Err(MngErrno::MngErrInternel);
        }
    }

    pub(self) fn dispatch_mountinfo(&self) -> Result<(), MngErrno> {
        // First mark all active mount point we have as dead.
        let mut dead_mount_set: HashSet<String> = HashSet::new();
        for unit in self.db.units_get_all().iter() {
            if unit.unit_type() == UnitType::UnitMount
                && unit.active_state() == UnitActiveState::UnitActive
            {
                dead_mount_set.insert(String::from(unit.get_id()));
            }
        }

        // Then start mount point we don't know.
        let mut mountinfo_content = String::new();
        File::open("/proc/self/mountinfo")
            .unwrap()
            .read_to_string(&mut mountinfo_content)
            .unwrap();
        let parser = mountinfo::Parser::new(mountinfo_content.as_bytes());
        for mount_result in parser {
            match mount_result {
                Ok(mount) => {
                    // We don't process autofs for now, because it is not
                    // .mount but .automount in systemd.
                    if mount.fstype.to_str() == Some("autofs") {
                        continue;
                    }
                    let unit_name = mount_point_to_unit_name(mount.mount_point.to_str().unwrap());
                    if dead_mount_set.contains(unit_name.as_str()) {
                        dead_mount_set.remove(unit_name.as_str());
                    } else if let Some(unit) = self.load.load_unit(unit_name.as_str()) {
                        match unit.start() {
                            Ok(_) => {
                                log::debug!("{} change to mounted.", unit_name)
                            }
                            Err(_) => {
                                log::error!("Failed to start {}", unit_name)
                            }
                        }
                    }
                }
                Err(err) => {
                    log::error!("Failed to parse /proc/self/mountinfo: {}", err);
                }
            }
        }

        // Finally stop mount point in dead_mount_set.
        for unit_name in dead_mount_set.into_iter() {
            if let Some(unit) = self.db.units_get(unit_name.as_str()) {
                match unit.stop() {
                    Ok(_) => {
                        log::debug!("{} change to dead.", unit_name)
                    }
                    Err(_) => {
                        log::error!("Failed to stop {}.", unit_name)
                    }
                }
            }
        }
        Ok(())
    }

    pub(self) fn new(
        dmr: &Rc<DataManager>,
        eventr: &Rc<Events>,
        configm: &Rc<ManagerConfig>,
    ) -> Rc<UnitManager> {
        let _db = Rc::new(UnitDb::new());
        let _rt = Rc::new(UnitRT::new(&_db));
        let um = Rc::new(UnitManager {
            load: UnitLoad::new(dmr, &_db, &_rt),
            db: Rc::clone(&_db),
            rt: Rc::clone(&_rt),
            jm: JobManager::new(&_db, eventr),
            exec: ExecSpawn::new(),
            events: eventr.clone(),
            config: configm.clone(),
        });
        um.load.set_um(&um);
        um
    }

    fn load_unit(&self, name: &str) -> Option<Rc<UnitX>> {
        self.load.load_unit(name)
    }

    fn notify_message(
        &self,
        ucred: &UnixCredentials,
        messages: &HashMap<&str, &str>,
        fds: &Vec<i32>,
    ) -> Result<(), ServiceError> {
        let unit = self.get_unit_by_pid(Pid::from_raw(ucred.pid()));
        log::debug!("get unit by ucred pid: {}", ucred.pid());

        if unit.is_some() {
            unit.unwrap().notify_message(ucred, messages, fds)?;
            return Ok(());
        }

        log::warn!("Not found the unit for pid: {}", ucred.pid());
        Ok(())
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

        for other in self
            .db
            .dep_gets_atom(&unitx, UnitRelationAtom::UnitAtomTriggeredBy)
        {
            other.trigger(&unitx);
        }
    }

    fn remove_states(&self, _source: &str) {
        todo!();
    }
}

pub trait UnitMngUtil {
    fn attach(&self, um: Rc<UnitManager>);
}

pub trait UnitSubClass: UnitObj + UnitMngUtil {
    fn into_unitobj(self: Box<Self>) -> Box<dyn UnitObj>;
}

// #[macro_use]
// use crate::unit_name_to_type;
//unitManager composition of units with hash map
#[macro_export]
macro_rules! declure_unitobj_plugin {
    ($unit_type:ty, $constructor:path, $name:expr, $level:expr) => {
        // method for create the unit instance
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

mod unit_load {
    use super::UnitManager;
    use crate::manager::data::{DataManager, UnitDepConf};
    use crate::manager::table::{TableOp, TableSubscribe};
    use crate::manager::unit::uload_util::UnitFile;
    use crate::manager::unit::unit_base::{self, UnitType};
    use crate::manager::unit::unit_datastore::UnitDb;
    use crate::manager::unit::unit_entry::UnitX;
    use crate::manager::unit::unit_runtime::UnitRT;
    use crate::plugin::Plugin;
    use std::cell::RefCell;
    use std::rc::{Rc, Weak};

    //#[derive(Debug)]
    pub(super) struct UnitLoad {
        sub_name: String, // key for table-subscriber: UnitDepConf
        data: Rc<UnitLoadData>,
    }

    impl UnitLoad {
        pub(super) fn new(dmr: &Rc<DataManager>, dbr: &Rc<UnitDb>, rtr: &Rc<UnitRT>) -> UnitLoad {
            let load = UnitLoad {
                sub_name: String::from("UnitLoad"),
                data: Rc::new(UnitLoadData::new(dmr, dbr, rtr)),
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

        fn register(&self, dm: &DataManager) {
            let subscriber = Rc::clone(&self.data);
            let ret = dm.register_ud_config(&self.sub_name, subscriber);
            if let Some(_r) = ret {
                log::info!("TableSubcribe for {} is already register", &self.sub_name);
            } else {
                log::info!("register  TableSubcribe for {}  successful", &self.sub_name);
            }
        }
    }

    //#[derive(Debug)]
    struct UnitLoadData {
        // associated objects
        dm: Rc<DataManager>,
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
            dbr: &Rc<UnitDb>,
            rtr: &Rc<UnitRT>,
        ) -> UnitLoadData {
            log::debug!("UnitLoadData db count is {}", Rc::strong_count(dbr));
            let file = Rc::new(UnitFile::new());
            UnitLoadData {
                dm: Rc::clone(dmr),
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
                    return None;
                }
            }
        }

        pub(self) fn push_dep_unit_into_load_queue(&self, name: &str) -> Option<Rc<UnitX>> {
            if let Some(unit) = self.db.units_get(name) {
                return Some(Rc::clone(&unit));
            };
            let unit = self.prepare_unit(name);
            unit
        }

        pub(self) fn load_unit(&self, name: &str) -> Option<Rc<UnitX>> {
            if let Some(unit) = self.db.units_get(name) {
                return Some(Rc::clone(&unit));
            };
            let unit = self.prepare_unit(name);
            let u = if let Some(u) = unit {
                u
            } else {
                return None;
            };
            log::info!("begin dispatch unit in  load queue");
            self.rt.dispatch_load_queue();
            Some(Rc::clone(&u))
        }

        pub(self) fn set_um(&self, um: &Rc<UnitManager>) {
            self.um.replace(Rc::downgrade(um));
        }

        fn try_new_unit(&self, name: &str) -> Option<Rc<UnitX>> {
            let unit_type = unit_base::unit_name_to_type(name);
            if unit_type == UnitType::UnitTypeInvalid {
                return None;
            }

            log::info!(
                "begin create obj for type {}, name {} by plugin",
                unit_type.to_string(),
                name
            );
            let plugins = Plugin::get_instance();
            let subclass = match plugins.create_unit_obj(unit_type) {
                Ok(sub) => sub,
                Err(_e) => {
                    log::error!("Failed to create unit_obj!");
                    return None;
                }
            };

            subclass.attach(self.um.clone().into_inner().upgrade().unwrap());

            Some(Rc::new(UnitX::new(
                &self.dm,
                &self.file,
                unit_type,
                name,
                subclass.into_unitobj(),
            )))
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
                        //此处不能直接调用unit_load，会嵌套
                        tmp_unit = Rc::clone(&o_unit);
                    } else {
                        log::error!("create unit obj error in unit manager");
                        return;
                    }

                    if let Err(_e) =
                        self.db
                            .dep_insert(Rc::clone(&unit), *relation, tmp_unit, true, 0)
                    //依赖关系插入，但是未判断是否load成功，如果unit无法load，是否应该记录依赖关系
                    {
                        log::debug!("add dependency relation failed for source unit is {},dependency unit is {}",unit.get_id(),o_name);
                        return;
                    }
                }
            }
        }

        fn remove_udconf(&self, _source: &str) {
            todo!();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use event::Events;
    use nix::errno::Errno;
    use nix::sys::signal::Signal;
    use std::thread;
    use std::time::Duration;
    use utils::logger;

    use crate::mount::mount_setup;

    fn init_dm_for_test() -> (Rc<DataManager>, Rc<Events>, Rc<UnitManager>) {
        logger::init_log_with_console("manager test", 4);
        let dm_manager = Rc::new(DataManager::new());
        let _event = Rc::new(Events::new().unwrap());
        let configm = Rc::new(ManagerConfig::new());
        let um = UnitManager::new(&dm_manager, &_event, &configm);
        (dm_manager, _event, um)
    }

    fn setup_mount_point() -> Result<(), Errno> {
        mount_setup::mount_setup()?;

        Ok(())
    }

    #[test]
    fn test_service_unit_load() {
        logger::init_log_with_console("test_service_unit_load", 4);
        let configm = Rc::new(ManagerConfig::new());
        let dm_manager = Rc::new(DataManager::new());
        let _event = Rc::new(Events::new().unwrap());
        let um = UnitManager::new(&dm_manager, &_event, &configm);

        let unit_name = String::from("config.service");
        let unit = um.load_unit(&unit_name);

        match unit {
            Some(_unit_obj) => assert_eq!(_unit_obj.get_id(), "config.service"),
            None => println!("test unit load, not found unit: {}", unit_name),
        };
    }

    #[test]
    fn test_service_unit_start() {
        let ret = setup_mount_point();
        if ret.is_err() {
            return;
        }

        logger::init_log_with_console("test_service_unit_start", 4);
        let configm = Rc::new(ManagerConfig::new());
        let dm_manager = Rc::new(DataManager::new());
        let _event = Rc::new(Events::new().unwrap());
        let um = UnitManager::new(&dm_manager, &_event, &configm);

        let unit_name = String::from("config.service");
        let unit = um.load_unit(&unit_name);

        assert_eq!(unit.is_some(), true);
        let u = unit.unwrap();

        let ret = u.start();
        assert_eq!(ret.is_err(), false);

        log::debug!("unit start end!");
        let ret = u.stop();
        assert_eq!(ret.is_err(), false);
        log::debug!("unit stop end!");
    }

    #[test]
    fn test_socket_unit_start_and_stop() {
        logger::init_log_with_console("test_socket_unit_start_stop", 4);

        let ret = setup_mount_point();
        if ret.is_err() {
            return;
        }

        let dm = init_dm_for_test();

        let unit_name = String::from("test.socket");
        let unit = dm.2.load_unit(&unit_name);

        assert_eq!(unit.is_some(), true);
        let u = unit.unwrap();

        let ret = u.start();
        log::debug!("socket start ret is: {:?}", ret);
        assert_eq!(ret.is_err(), false);

        thread::sleep(Duration::from_secs(4));
        u.sigchld_events(Pid::from_raw(-1), 0, Signal::SIGCHLD);
        assert_eq!(u.active_state(), UnitActiveState::UnitActive);

        let ret = u.stop();
        log::debug!("socket stop ret is: {:?}", ret);
        assert_eq!(ret.is_err(), false);

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
        match confilict_unit {
            Ok(_v) => {
                assert!(true, "conflict unit start successful");
            }
            Err(e) => {
                assert!(false, "load unit failed {},{:?}", conflict_unit_name, e);
            }
        }
    }

    #[test]
    fn test_units_load() {
        logger::init_log_with_console("test_units_load", 4);
        let mut unit_name_lists: Vec<String> = Vec::new();
        let configm = Rc::new(ManagerConfig::new());
        let dm_manager = Rc::new(DataManager::new());
        let _event = Rc::new(Events::new().unwrap());
        let um = UnitManager::new(&dm_manager, &_event, &configm);

        unit_name_lists.push("config.service".to_string());
        // unit_name_lists.push("testsunit.target".to_string());
        for u_name in unit_name_lists.iter() {
            let unit = um.load_unit(u_name);
            match unit {
                Some(_unit_obj) => assert_eq!(_unit_obj.get_id(), u_name),
                None => println!("test unit load, not found unit: {}", u_name),
            };
        }
    }
    #[test]
    fn test_target_unit_load() {
        logger::init_log_with_console("test_target_unit_load", 4);
        let configm = Rc::new(ManagerConfig::new());
        let mut unit_name_lists: Vec<String> = Vec::new();
        let dm_manager = Rc::new(DataManager::new());
        let _event = Rc::new(Events::new().unwrap());
        let um = UnitManager::new(&dm_manager, &_event, &configm);

        unit_name_lists.push("testsunit.target".to_string());
        // unit_name_lists.push("testsunit.target".to_string());
        for u_name in unit_name_lists.iter() {
            let unit = um.load_unit(u_name);
            match unit {
                Some(_unit_obj) => {
                    println!(
                        "{:?}",
                        _unit_obj.get_config().config_data().borrow().Unit.Requires
                    );
                    assert_eq!(_unit_obj.get_id(), u_name);
                }
                None => println!("test unit load, not found unit: {}", u_name),
            };
        }
    }
}
