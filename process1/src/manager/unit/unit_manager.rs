use super::job::{JobAffect, JobConf, JobKind, JobManager};
use super::unit_datastore::UnitDb;
use super::unit_file::UnitFile;
use super::unit_load::UnitLoad;
use super::unit_parser_mgr::{UnitConfigParser, UnitParserMgr};
use super::unit_relation_atom::UnitRelationAtom;
use super::unit_runtime::UnitRT;
use super::UnitObj;
use crate::manager::data::{DataManager, JobMode, UnitState};
use crate::manager::table::{TableOp, TableSubscribe};
use crate::manager::MngErrno;
use event::Events;
use nix::unistd::Pid;
use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;

// #[macro_use]
// use crate::unit_name_to_type;
//unitManger composition of units with hash map

pub trait UnitMngUtil {
    fn attach(&mut self, um: Rc<UnitManager>);
}

pub trait UnitSubClass: UnitObj + UnitMngUtil {
    fn into_unitobj(self: Box<Self>) -> Box<dyn UnitObj>;
}

#[macro_export]
macro_rules! declure_unitobj_plugin {
    ($unit_type:ty, $constructor:path, $name:expr, $level:expr) => {
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

//#[derive(Debug)]
pub struct UnitManagerX {
    states: Rc<UnitStates>,
    data: Rc<UnitManager>,
}

impl UnitManagerX {
    pub(in crate::manager) fn new(dm: Rc<DataManager>, event: Rc<RefCell<Events>>) -> UnitManagerX {
        let _dm = Rc::clone(&dm);
        let _um = UnitManager::new(Rc::clone(&_dm), event);
        UnitManagerX {
            states: Rc::new(UnitStates::new(Rc::clone(&_dm), Rc::clone(&_um))),
            data: _um,
        }
    }

    pub fn start_unit(&self, name: &str) -> Result<(), MngErrno> {
        self.data.start_unit(name)
    }

    pub fn stop_unit(&self, name: &str) -> Result<(), MngErrno> {
        self.data.stop_unit(name)
    }

    pub fn child_dispatch_sigchld(&self) -> Result<(), Box<dyn Error>> {
        self.data.db.child_dispatch_sigchld()
    }

    pub fn dispatch_load_queue(&self) {
        self.data.rt.dispatch_load_queue()
    }
}

//#[derive(Debug)]
pub struct UnitManager {
    // associated objects
    dm: Rc<DataManager>,
    event: Rc<RefCell<Events>>,

    file: Rc<UnitFile>,
    load: Rc<UnitLoad>,
    db: Rc<UnitDb>, // ALL UNIT STORE IN UNITDB,AND OTHER USE REF
    rt: Rc<UnitRT>,
    jm: Rc<JobManager>,
    unit_conf_parser_mgr: Rc<UnitParserMgr<UnitConfigParser>>,
}

impl UnitManager {
    pub fn child_watch_pid(&self, pid: Pid, id: &str) {
        self.db.child_add_watch_pid(pid, id)
    }

    pub fn child_unwatch_pid(&self, pid: Pid) {
        self.db.child_unwatch_pid(pid)
    }

    pub fn start_unit(&self, name: &str) -> Result<(), MngErrno> {
        if let Some(unit) = self.load.load_unit(name) {
            self.jm.exec(
                &JobConf::new(Rc::clone(&unit), JobKind::JobStart),
                JobMode::JobReplace,
                &mut JobAffect::new(false),
            )?;
            Ok(())
        } else {
            return Err(MngErrno::MngErrInternel);
        }
    }

    pub fn stop_unit(&self, name: &str) -> Result<(), MngErrno> {
        if let Some(unit) = self.load.load_unit(name) {
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

    pub(in crate::manager) fn new(
        dm: Rc<DataManager>,
        event: Rc<RefCell<Events>>,
    ) -> Rc<UnitManager> {
        let _dm = Rc::clone(&dm);
        let _event = Rc::clone(&event);
        let _file = Rc::new(UnitFile::new());
        let _db = Rc::new(UnitDb::new());
        let rt = Rc::new(UnitRT::new());
        let unit_conf_parser_mgr = Rc::new(UnitParserMgr::default());

        let _load = Rc::new(UnitLoad::new(
            Rc::clone(&_dm),
            Rc::clone(&_file),
            Rc::clone(&_db),
            Rc::clone(&rt),
            Rc::clone(&unit_conf_parser_mgr),
        ));

        let um = Rc::new(UnitManager {
            dm,
            event,

            file: Rc::clone(&_file),
            load: Rc::clone(&_load),
            db: Rc::clone(&_db),
            rt: Rc::clone(&rt),
            jm: Rc::new(JobManager::new(Rc::clone(&_db), Rc::clone(&_event))),
            unit_conf_parser_mgr: Rc::clone(&unit_conf_parser_mgr),
        });

        _load.set_um(um.clone());
        um
    }
}

//#[derive(Debug)]
struct UnitStates {
    name: String,            // key for table-subscriber
    data: Rc<UnitStatesSub>, // data for table-subscriber
}

impl UnitStates {
    pub(self) fn new(dm: Rc<DataManager>, um: Rc<UnitManager>) -> UnitStates {
        let us = UnitStates {
            name: String::from("UnitStates"),
            data: Rc::new(UnitStatesSub::new(um)),
        };
        us.register(&dm);
        us
    }

    fn register(&self, dm: &DataManager) {
        let subscriber = Rc::clone(&self.data);
        dm.register_unit_state(self.name.clone(), subscriber)
            .expect("unit dependency has been registered.");
    }
}

//#[derive(Debug)]
struct UnitStatesSub {
    um: Rc<UnitManager>,
}

impl TableSubscribe<String, UnitState> for UnitStatesSub {
    fn filter(&self, _op: &TableOp<String, UnitState>) -> bool {
        // everything is allowed
        true
    }

    fn notify(&self, op: &TableOp<String, UnitState>) {
        match op {
            TableOp::TableInsert(name, config) => self.insert_states(name, config),
            TableOp::TableRemove(name, _) => self.remove_states(name),
        }
    }
}

// the declaration "pub(self)" is for identification only.
impl UnitStatesSub {
    pub(self) fn new(um: Rc<UnitManager>) -> UnitStatesSub {
        UnitStatesSub { um }
    }

    pub(self) fn insert_states(&self, source: &str, state: &UnitState) {
        let unitx = if let Some(u) = self.um.db.units_get(source) {
            u
        } else {
            return;
        };

        self.um
            .jm
            .clone()
            .try_finish(&unitx, state.get_os(), state.get_ns(), state.get_flags())
            .unwrap();

        for other in self
            .um
            .db
            .dep_gets_atom(&unitx, UnitRelationAtom::UnitAtomTriggeredBy)
        {
            other.trigger(&unitx);
        }
    }

    pub(self) fn remove_states(&self, _source: &str) {
        todo!();
    }
}

#[cfg(test)]
mod tests {
    // use services::service::ServiceUnit;

    use utils::logger;

    use super::*;
    use crate::manager::unit::unit_parser_mgr::UnitParserMgr;
    use event::Events;

    #[test]
    fn test_unit_load() {
        logger::init_log_with_console("test_unit_load", 4);
        log::info!("test");
        let dm_manager = Rc::new(DataManager::new());
        let um = UnitManager::new(
            dm_manager.clone(),
            Rc::new(RefCell::new(Events::new().unwrap())),
        );
        um.file.init_lookup_path();
        let load = Rc::clone(&um.load);
        let unit_name = String::from("config.service");
        let loaded_unit = load.load_unit(&unit_name);

        match um.db.units_get(&unit_name) {
            Some(_unit_obj) => assert_eq!(_unit_obj.get_id(), loaded_unit.unwrap().get_id()),
            None => assert!(false, "not fount unit: {}", unit_name),
        };
    }

    #[test]
    fn test_unit_start() {
        let dm_manager = Rc::new(DataManager::new());
        let um = UnitManager::new(
            dm_manager.clone(),
            Rc::new(RefCell::new(Events::new().unwrap())),
        );
        um.file.init_lookup_path();
        let load = Rc::clone(&um.load);
        let unit_name = String::from("config.service");
        load.load_unit(&unit_name);

        match um.db.units_get(&unit_name) {
            Some(_unit_obj) => println!("found unit obj {}", unit_name),
            None => assert!(false, "not fount unit: {}", unit_name),
        };
    }
}
