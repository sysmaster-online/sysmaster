use super::unit_datastore::UnitDb;
use super::unit_entry::UnitX;
use super::unit_file::UnitFile;
use super::unit_parser_mgr::{UnitConfigParser, UnitParserMgr};
use super::unit_runtime::UnitRT;
use crate::manager::data::{DataManager, UnitConfig};
use std::cell::RefCell;
use std::rc::Rc;

//#[derive(Debug)]
pub(super) struct UnitLoad {
    data: RefCell<UnitLoadData>,
}

impl UnitLoad {
    pub(super) fn new(
        dm: Rc<DataManager>,
        file: Rc<UnitFile>,
        unitdb: Rc<UnitDb>,
        rt: Rc<UnitRT>,
        unit_conf_parser_mgr: Rc<UnitParserMgr<UnitConfigParser>>,
    ) -> UnitLoad {
        UnitLoad {
            data: RefCell::new(UnitLoadData::new(
                dm,
                file,
                unitdb,
                rt,
                unit_conf_parser_mgr,
            )),
        }
    }

    pub(super) fn load_unit(&self, name: &str) -> Option<Rc<UnitX>> {
        self.data.borrow_mut().load_unit(name)
    }
}

//#[derive(Debug)]
struct UnitLoadData {
    // associated objects
    dm: Rc<DataManager>,
    db: Rc<UnitDb>,
    file: Rc<UnitFile>,
    rt: Rc<UnitRT>,
    unit_conf_parser_mgr: Rc<UnitParserMgr<UnitConfigParser>>,
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
        }
    }

    pub(self) fn prepare_unit(&mut self, name: &str) -> Option<Rc<UnitX>> {
        let u_config = UnitConfig::new();
        //need parse unit file here,but subclass need unit file parser in load unit again
        //self.file.build_name_map();
        //self.file.get_unit_file_path(name);//if not parse here cannot get dependency
        self.dm.insert_unit_config(name.to_string(), u_config);
        match self.db.units_get(name) {
            Some(unit) => {
                let rc_unit = Rc::new(unit);
                self.db.units_insert(name.to_string(), Rc::clone(&rc_unit));
                self.rt.push_load_queue(Rc::clone(&rc_unit));
                Some(Rc::clone(&rc_unit))
            }
            None => {
                log::error!("create unit obj failed.");
                return None;
            }
        }
    }

    pub(self) fn load_unit(&mut self, name: &str) -> Option<Rc<UnitX>> {
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

#[cfg(test)]
mod tests {
    // use services::service::ServiceUnit;

    use crate::manager::unit::unit_parser_mgr::UnitParserMgr;

    use super::*;
    use utils::logger;

    #[test]
    fn test_unit_load() {
        logger::init_log_with_console("test", 4);
        log::info!("test");
        let dm_manager = Rc::new(DataManager::new());
        let file = Rc::new(UnitFile::new());
        let db = Rc::new(UnitDb::new());
        let rt = Rc::new(UnitRT::new());
        let unit_conf_parser_mgr = Rc::new(UnitParserMgr::default());
        let load = UnitLoad::new(dm_manager, file, db, rt, unit_conf_parser_mgr);
        load.data.borrow_mut().file.init_lookup_path();

        let unit_name = String::from("config.service");
        load.load_unit(&unit_name);

        //assert_ne!(unit_manager.units.borrow().len(), 0);

        match load.data.borrow().db.units_get(&unit_name) {
            Some(_unit_obj) => println!("found unit obj {}", unit_name),
            None => println!("not fount unit: {}", unit_name),
        };
    }

    #[test]
    fn test_unit_start() {
        let dm_manager = Rc::new(DataManager::new());
        let file = Rc::new(UnitFile::new());
        let db = Rc::new(UnitDb::new());
        let rt = Rc::new(UnitRT::new());
        let unit_parser_mgr = Rc::new(UnitParserMgr::default());
        let load = UnitLoad::new(dm_manager, file, db, rt, unit_parser_mgr);
        load.data.borrow_mut().file.init_lookup_path();

        let unit_name = String::from("config.service");
        load.load_unit(&unit_name);

        match load.data.borrow().db.units_get(&unit_name) {
            Some(_unit_obj) => println!("found unit obj {}", unit_name),
            None => println!("not fount unit: {}", unit_name),
        };
    }
}
