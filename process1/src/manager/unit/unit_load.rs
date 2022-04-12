use super::unit_datastore::UnitDb;
use super::unit_entry::UnitX;
use super::unit_file::UnitFile;
use super::unit_parser_mgr::{UnitConfigParser, UnitParserMgr};
use super::unit_runtime::UnitRT;
use super::UnitManager;
use crate::manager::data::{DataManager, UnitConfig};
use crate::manager::table::{TableOp, TableSubscribe};
use crate::manager::unit::unit_base;
use crate::manager::UnitType;
use crate::plugin::Plugin;
use std::cell::RefCell;
use std::rc::Rc;

//#[derive(Debug)]
pub(super) struct UnitLoad {
    data: Rc<UnitLoadData>,
    uconf_register: UnitConfigs,
}

impl UnitLoad {
    pub(super) fn new(
        dm: Rc<DataManager>,
        file: Rc<UnitFile>,
        unitdb: Rc<UnitDb>,
        rt: Rc<UnitRT>,
        unit_conf_parser_mgr: Rc<UnitParserMgr<UnitConfigParser>>,
    ) -> UnitLoad {
        let unit_load_data = UnitLoadData::new(dm, file, unitdb, rt, unit_conf_parser_mgr);
        let rc_unit_load_data = Rc::new(unit_load_data);
        UnitLoad {
            data: rc_unit_load_data.clone(),
            uconf_register: UnitConfigs::new(rc_unit_load_data.clone()),
        }
    }

    pub fn set_um(&self, um: Rc<UnitManager>) {
        self.data.set_um(um);
    }
    pub(super) fn load_unit(&self, name: &str) -> Option<Rc<UnitX>> {
        self.data.load_unit(name)
    }
}

//#[derive(Debug)]
struct UnitLoadData {
    // associated objectsd
    dm: Rc<DataManager>,
    db: Rc<UnitDb>,
    file: Rc<UnitFile>,
    rt: Rc<UnitRT>,
    unit_conf_parser_mgr: Rc<UnitParserMgr<UnitConfigParser>>,
    um: RefCell<Option<Rc<UnitManager>>>,
}

// the declaration "pub(self)" is for identification only.
impl UnitLoadData {
    pub(self) fn new(
        dm: Rc<DataManager>,
        file: Rc<UnitFile>,
        db: Rc<UnitDb>,
        rt: Rc<UnitRT>,
        unit_conf_parser_mgr: Rc<UnitParserMgr<UnitConfigParser>>,
    ) -> UnitLoadData {
        UnitLoadData {
            dm,
            db,
            file,
            rt,
            unit_conf_parser_mgr,
            um: RefCell::new(None),
        }
    }

    pub fn set_um(&self, um: Rc<UnitManager>) {
        self.um.replace(Some(um));
    }

    pub(self) fn prepare_unit(&self, name: &str) -> Option<Rc<UnitX>> {
        let u_config = UnitConfig::new();
        self.dm
            .insert_unit_config(name.to_string(), Rc::new(u_config));
        match self.db.units_get(name) {
            Some(unit) => {
                let rc_unit = Rc::new(unit);
                self.rt.push_load_queue(Rc::clone(&rc_unit));
                Some(Rc::clone(&rc_unit))
            }
            None => {
                log::error!("create unit obj failed.");
                return None;
            }
        }
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
        log::info!("push new unit into load queue");
        self.rt.dispatch_load_queue();
        Some(Rc::clone(&u))
    }
}

//#[derive(Debug)]
struct UnitConfigs {
    name: String,             // key for table-subscriber
    data: Rc<UnitConfigsSub>, // data for table-subscriber
}

// the declaration "pub(self)" is for identification only.
impl UnitConfigs {
    pub(self) fn new(unit_load_data: Rc<UnitLoadData>) -> UnitConfigs {
        let _dm = Rc::clone(&unit_load_data.dm);
        let uc = UnitConfigs {
            name: String::from("UnitConfigs"),
            data: Rc::new(UnitConfigsSub::new(unit_load_data)),
        };
        uc.register(&_dm);
        uc
    }

    fn register(&self, dm: &DataManager) {
        let subscriber = Rc::clone(&self.data);
        let register_result = dm.register_unit_config(self.name.clone(), subscriber);
        if let Some(_r) = register_result {
            log::info!("TableSubcribe for {} is already register", &self.name);
        } else {
            log::info!("register  TableSubcribe for {}  sucessfull", &self.name);
        }
    }
}

//#[derive(Debug)]
struct UnitConfigsSub {
    unit_load_data: Rc<UnitLoadData>,
}

impl TableSubscribe<String, Rc<UnitConfig>> for UnitConfigsSub {
    fn filter(&self, _op: &TableOp<String, Rc<UnitConfig>>) -> bool {
        // everything is allowed
        true
    }

    fn notify(&self, op: &TableOp<String, Rc<UnitConfig>>) {
        match op {
            TableOp::TableInsert(name, config) => self.insert_config(name, config),
            TableOp::TableRemove(_, _) => {} // self.remove_config(name)
        }
    }
}

// the declaration "pub(self)" is for identification only.
impl UnitConfigsSub {
    pub(self) fn new(unit_load_data: Rc<UnitLoadData>) -> UnitConfigsSub {
        UnitConfigsSub { unit_load_data }
    }

    pub(self) fn insert_config(&self, name: &str, config: &Rc<UnitConfig>) {
        //hash map insert return is old value,need reconstruct
        let unit = match self.try_new_unit(name) {
            Some(u) => u,
            None => {
                log::error!("create unit obj error in unit manger");
                return;
            } // load
        };
        self.unit_load_data
            .db
            .units_insert(name.to_string(), Rc::clone(&unit));

        // config
        unit.set_config(config);

        // dependency
        for (relation, name) in config.deps.iter() {
            let tmp_unit: Rc<UnitX>;
            if let Some(unit) = self.unit_load_data.db.units_get(name) {
                tmp_unit = Rc::clone(&unit);
            } else {
                tmp_unit = match self.try_new_unit(name) {
                    Some(u) => Rc::clone(&u),
                    None => {
                        log::error!("create unit obj error in unit manger");
                        return;
                    }
                };

                self.unit_load_data
                    .db
                    .units_insert(name.to_string(), Rc::clone(&tmp_unit));
                self.unit_load_data.rt.push_load_queue(Rc::clone(&tmp_unit)); //cannot be invok embended,because cannot be  borrow two reference
            }

            if let Err(_e) =
                self.unit_load_data
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

        if let Some(unit) = self.unit_load_data.db.units_get(name) {
            return Some(unit);
        }
        log::info!(
            "begin create obj for  type {}, name {} by plugin",
            unit_type.to_string(),
            name
        );
        let plugins = Rc::clone(&Plugin::get_instance());
        plugins.borrow_mut().set_library_dir("../target/debug");
        plugins.borrow_mut().load_lib();
        let mut subclass = match plugins.borrow().create_unit_obj(unit_type) {
            Ok(sub) => sub,
            Err(_e) => return None,
        };
        subclass.get_private_conf_section_name().map(|s| {
            self.unit_load_data
                .unit_conf_parser_mgr
                .register_parser_by_private_section_name(unit_type.to_string(), s.to_string())
        });

        subclass.attach(self.unit_load_data.um.borrow().as_ref().cloned().unwrap());

        Some(Rc::new(UnitX::new(
            Rc::clone(&self.unit_load_data.dm),
            Rc::clone(&self.unit_load_data.file),
            Rc::clone(&self.unit_load_data.unit_conf_parser_mgr),
            unit_type,
            name,
            subclass.into_unitobj(),
        )))
    }
}

#[cfg(test)]
mod tests {
    // use services::service::ServiceUnit;

    use utils::logger;

    use super::*;
    use crate::manager::unit::unit_parser_mgr::UnitParserMgr;

    #[test]
    fn test_unit_load() {
        logger::init_log_with_console("test_unit_load", 4);
        log::info!("test");
        let dm_manager = Rc::new(DataManager::new());
        let um = UnitManager::new(dm_manager.clone());
        let file = Rc::new(UnitFile::new());
        let db = Rc::new(UnitDb::new());
        let rt = Rc::new(UnitRT::new());
        let unit_conf_parser_mgr = Rc::new(UnitParserMgr::default());
        let load = UnitLoad::new(dm_manager, file, db, rt, unit_conf_parser_mgr);
        load.data.file.init_lookup_path();
        load.set_um(um);

        let unit_name = String::from("config.service");
        let loaded_unit = load.load_unit(&unit_name);

        match load.data.db.units_get(&unit_name) {
            Some(_unit_obj) => assert_eq!(_unit_obj.get_id(), loaded_unit.unwrap().get_id()),
            None => println!("not fount unit: {}", unit_name),
        };
    }

    #[test]
    fn test_unit_start() {
        let dm_manager = Rc::new(DataManager::new());
        let um = UnitManager::new(dm_manager.clone());
        let file = Rc::new(UnitFile::new());
        let db = Rc::new(UnitDb::new());
        let rt = Rc::new(UnitRT::new());
        let unit_parser_mgr = Rc::new(UnitParserMgr::default());
        let load = UnitLoad::new(dm_manager, file, db, rt, unit_parser_mgr);
        load.data.file.init_lookup_path();
        load.set_um(um);

        let unit_name = String::from("config.service");
        load.load_unit(&unit_name);

        match load.data.db.units_get(&unit_name) {
            Some(_unit_obj) => println!("found unit obj {}", unit_name),
            None => println!("not fount unit: {}", unit_name),
        };
    }
}
