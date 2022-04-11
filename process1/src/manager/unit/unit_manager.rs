use super::job::JobManager;
use super::unit_base::{self};
use super::unit_datastore::UnitDb;
use super::unit_entry::{UnitObj, UnitX};
use super::unit_file::UnitFile;
use super::unit_load::UnitLoad;
use super::unit_parser_mgr::{UnitConfigParser, UnitParserMgr};
use super::unit_relation_atom::UnitRelationAtom;
use super::unit_runtime::UnitRT;
use crate::manager::data::{DataManager, UnitConfig, UnitState, UnitType};
use crate::manager::table::{TableOp, TableSubscribe};
use crate::plugin::Plugin;
use nix::unistd::Pid;
use std::error::Error;
use std::rc::Rc;

// #[macro_use]
// use crate::unit_name_to_type;
//unitManger composition of units with hash map

pub trait UnitMngUtil {
    fn attach(&self, um: Rc<UnitManager>);
}

pub trait UnitSubClass: UnitObj + UnitMngUtil {}

#[macro_export]
macro_rules! declure_unitobj_plugin {
    ($unit_type:ty, $constructor:path) => {
        #[no_mangle]
        pub fn __unit_obj_create() -> *mut dyn $crate::manager::UnitObj {
            let construcotr: fn() -> $unit_type = $constructor;

            let obj = construcotr();
            let boxed: Box<dyn $crate::manager::UnitObj> = Box::new(obj);
            Box::into_raw(boxed)
        }
    };
}

//#[derive(Debug)]
pub struct UnitManagerX {
    // associated objects
    dm: Rc<DataManager>,

    configs: Rc<UnitConfigs>,
    states: Rc<UnitStates>,
    data: Rc<UnitManager>,
}

impl UnitManagerX {
    pub(in crate::manager) fn new(dm: Rc<DataManager>) -> UnitManagerX {
        let _dm = Rc::clone(&dm);
        let _um = Rc::new(UnitManager::new(Rc::clone(&_dm)));
        UnitManagerX {
            dm,
            configs: Rc::new(UnitConfigs::new(Rc::clone(&_dm), Rc::clone(&_um))),
            states: Rc::new(UnitStates::new(Rc::clone(&_dm), Rc::clone(&_um))),
            data: _um,
        }
    }

    pub fn child_dispatch_sigchld(&self) -> Result<(), Box<dyn Error>> {
        self.data.db.child_dispatch_sigchld()
    }
}

//#[derive(Debug)]
pub struct UnitManager {
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

    pub(in crate::manager) fn new(dm: Rc<DataManager>) -> UnitManager {
        let _dm = Rc::clone(&dm);
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
        UnitManager {
            file: Rc::clone(&_file),
            load: Rc::clone(&_load),
            db: Rc::clone(&_db),
            rt,
            jm: Rc::new(JobManager::new(Rc::clone(&_db))),
            unit_conf_parser_mgr: Rc::clone(&unit_conf_parser_mgr),
        }
    }
}

//#[derive(Debug)]
struct UnitConfigs {
    name: String,             // key for table-subscriber
    data: Rc<UnitConfigsSub>, // data for table-subscriber
}

// the declaration "pub(self)" is for identification only.
impl UnitConfigs {
    pub(self) fn new(dm: Rc<DataManager>, um: Rc<UnitManager>) -> UnitConfigs {
        let _dm = Rc::clone(&dm);
        let uc = UnitConfigs {
            name: String::from("UnitConfigs"),
            data: Rc::new(UnitConfigsSub::new(dm, um)),
        };
        uc.register(&_dm);
        uc
    }

    fn register(&self, dm: &DataManager) {
        let subscriber = Rc::clone(&self.data);
        dm.register_unit_config(self.name.clone(), subscriber)
            .expect("unit configs has been registered.");
    }
}

//#[derive(Debug)]
struct UnitConfigsSub {
    dm: Rc<DataManager>,
    um: Rc<UnitManager>,
}

impl TableSubscribe<String, UnitConfig> for UnitConfigsSub {
    fn filter(&self, _op: &TableOp<String, UnitConfig>) -> bool {
        // everything is allowed
        true
    }

    fn notify(&self, op: &TableOp<String, UnitConfig>) {
        match op {
            TableOp::TableInsert(name, config) => self.insert_config(name, config),
            TableOp::TableRemove(_, _) => {} // self.remove_config(name)
        }
    }
}

// the declaration "pub(self)" is for identification only.
impl UnitConfigsSub {
    pub(self) fn new(dm: Rc<DataManager>, um: Rc<UnitManager>) -> UnitConfigsSub {
        UnitConfigsSub { dm, um }
    }

    pub(self) fn insert_config(&self, name: &str, config: &UnitConfig) {
        //hash map insert return is old value,need reconstruct
        let unit = match self.try_new_unit(name) {
            Some(u) => u,
            None => {
                log::error!("create unit obj error in unit manger");
                return;
            } // load
        };
        //log::debug!("");
        self.um.db.units_insert(name.to_string(), Rc::clone(&unit));

        // config
        unit.set_config(config);

        // dependency
        for (relation, name) in config.deps.iter() {
            let tmp_unit: Rc<UnitX>;
            if let Some(unit) = self.um.db.units_get(name) {
                tmp_unit = Rc::clone(&unit);
            } else {
                tmp_unit = match self.try_new_unit(name) {
                    Some(u) => u,
                    None => {
                        log::error!("create unit obj error in unit manger");
                        return;
                    }
                };
                self.um
                    .db
                    .units_insert(name.to_string(), Rc::clone(&tmp_unit));
                self.um.rt.push_load_queue(Rc::clone(&tmp_unit));
            }

            if let Err(_e) = self
                .um
                .db
                .dep_insert(Rc::clone(&unit), *relation, tmp_unit, true, 0)
            {
                // debug
            }
        }
    }

    pub(self) fn remove_config(&self, _source: &str) {
        todo!();
    }

    fn try_new_unit(&self, name: &str) -> Option<Rc<UnitX>> {
        let unit_type = unit_base::unit_name_to_type(name);
        if unit_type == UnitType::UnitTypeInvalid {
            return None;
        }

        if let Some(unit) = self.um.db.units_get(name) {
            return Some(unit);
        }
        log::info!("begin create {} obj by plugin", name);
        let plugins = Rc::clone(&Plugin::get_instance());
        plugins.borrow_mut().set_library_dir("../target/debug");
        plugins.borrow_mut().load_lib();
        let subclass = match plugins.borrow().create_unit_obj(unit_type) {
            Ok(sub) => sub,
            Err(_e) => return None,
        };
        subclass.get_private_conf_section_name().map(|s| {
            self.um
                .unit_conf_parser_mgr
                .register_parser_by_private_section_name(unit_type.to_string(), s.to_string())
        });
        Some(Rc::new(UnitX::new(
            Rc::clone(&self.dm),
            Rc::clone(&self.um.file),
            Rc::clone(&self.um.unit_conf_parser_mgr),
            unit_type,
            name,
            subclass,
        )))
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
            TableOp::TableInsert(name, config) => self.insert_config(name, config),
            TableOp::TableRemove(_, _) => {} // self.remove_config(name)
        }
    }
}

// the declaration "pub(self)" is for identification only.
impl UnitStatesSub {
    pub(self) fn new(um: Rc<UnitManager>) -> UnitStatesSub {
        UnitStatesSub { um }
    }

    pub(self) fn insert_config(&self, source: &str, _state: &UnitState) {
        let unitx = self.um.db.units_get(source).unwrap();
        for other in self
            .um
            .db
            .dep_gets_atom(&unitx, UnitRelationAtom::UnitAtomTriggeredBy)
        {
            other.trigger(&unitx);
        }
    }

    pub(self) fn remove_config(&self, _source: &str) {
        todo!();
    }
}
