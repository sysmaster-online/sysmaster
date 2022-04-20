use crate::manager::unit::unit_entry::UnitX;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub(super) struct UnitSets {
    data: RefCell<UnitSetsData>,
}

impl UnitSets {
    pub(super) fn new() -> UnitSets {
        UnitSets {
            data: RefCell::new(UnitSetsData::new()),
        }
    }

    pub(super) fn insert(&self, name: String, unit: Rc<UnitX>) -> Option<Rc<UnitX>> {
        self.data.borrow_mut().insert(name, unit)
    }

    pub(super) fn remove(&self, name: &str) -> Option<Rc<UnitX>> {
        self.data.borrow_mut().remove(name)
    }

    pub(super) fn get(&self, name: &str) -> Option<Rc<UnitX>> {
        self.data.borrow().get(name)
    }

    pub(super) fn get_all(&self) -> Vec<Rc<UnitX>> {
        self.data.borrow().get_all()
    }
}

struct UnitSetsData {
    t: HashMap<String, Rc<UnitX>>, // key: string, value: unit
}

// the declaration "pub(self)" is for identification only.
impl UnitSetsData {
    pub(self) fn new() -> UnitSetsData {
        UnitSetsData { t: HashMap::new() }
    }

    pub(self) fn insert(&mut self, name: String, unit: Rc<UnitX>) -> Option<Rc<UnitX>> {
        self.t.insert(name, unit)
    }

    pub(self) fn remove(&mut self, name: &str) -> Option<Rc<UnitX>> {
        self.t.remove(name)
    }

    pub(self) fn get(&self, name: &str) -> Option<Rc<UnitX>> {
        self.t.get(name).cloned()
    }

    pub(self) fn get_all(&self) -> Vec<Rc<UnitX>> {
        self.t
            .iter()
            .map(|(_, ur)| Rc::clone(ur))
            .collect::<Vec<_>>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manager::data::{DataManager, UnitType};
    use crate::manager::unit::unit_file::UnitFile;
    use crate::manager::unit::unit_parser_mgr::UnitParserMgr;
    use crate::plugin::Plugin;
    use std::path::PathBuf;
    use utils::logger;

    #[test]
    fn sets_insert() {
        let sets = UnitSets::new();
        let name_test1 = String::from("test1.service");
        let unit_test1 = create_unit(&name_test1);
        let name_test2 = String::from("test2.service");
        let unit_test2 = create_unit(&name_test2);

        let old = sets.insert(name_test1.clone(), Rc::clone(&unit_test1));
        assert!(old.is_none());

        let old = sets.insert(name_test1.clone(), Rc::clone(&unit_test2));
        assert!(Rc::ptr_eq(&old.unwrap(), &unit_test1));

        let old = sets.insert(name_test2.clone(), Rc::clone(&unit_test2));
        assert!(old.is_none());
    }

    #[test]
    fn sets_remove() {
        let sets = UnitSets::new();
        let name_test1 = String::from("test1.service");
        let unit_test1 = create_unit(&name_test1);
        let name_test2 = String::from("test2.service");
        let unit_test2 = create_unit(&name_test2);
        let name_test3 = String::from("test3.service");

        let old = sets.remove(&name_test1);
        assert!(old.is_none());

        sets.insert(name_test1.clone(), Rc::clone(&unit_test1));
        let old = sets.remove(&name_test1);
        assert!(Rc::ptr_eq(&old.unwrap(), &unit_test1));

        sets.insert(name_test1.clone(), Rc::clone(&unit_test1));
        sets.insert(name_test2.clone(), Rc::clone(&unit_test2));
        let old = sets.remove(&name_test3);
        assert!(old.is_none());
        let old = sets.remove(&name_test2);
        assert!(Rc::ptr_eq(&old.unwrap(), &unit_test2));
    }

    #[test]
    fn sets_get() {
        let sets = UnitSets::new();
        let name_test1 = String::from("test1.service");
        let unit_test1 = create_unit(&name_test1);
        let name_test2 = String::from("test2.service");
        let unit_test2 = create_unit(&name_test2);

        let value = sets.get(&name_test1);
        assert!(value.is_none());

        sets.insert(name_test1.clone(), Rc::clone(&unit_test1));
        let value = sets.get(&name_test1);
        assert!(Rc::ptr_eq(&value.unwrap(), &unit_test1));
        let value = sets.get(&name_test2);
        assert!(value.is_none());

        sets.insert(name_test2.clone(), Rc::clone(&unit_test2));
        let value = sets.get(&name_test2);
        assert!(Rc::ptr_eq(&value.unwrap(), &unit_test2));
    }

    #[test]
    fn sets_getall() {
        let sets = UnitSets::new();
        let name_test1 = String::from("test1.service");
        let unit_test1 = create_unit(&name_test1);
        let name_test2 = String::from("test2.service");
        let unit_test2 = create_unit(&name_test2);

        let units = sets.get_all();
        assert_eq!(units.len(), 0);

        sets.insert(name_test1.clone(), Rc::clone(&unit_test1));
        let units = sets.get_all();
        assert_eq!(units.len(), 1);
        assert!(contain_unit(&units, &unit_test1));
        sets.remove(&name_test1);
        let units = sets.get_all();
        assert_eq!(units.len(), 0);

        sets.insert(name_test1.clone(), Rc::clone(&unit_test1));
        sets.insert(name_test2.clone(), Rc::clone(&unit_test2));
        let units = sets.get_all();
        assert_eq!(units.len(), 2);
        assert!(contain_unit(&units, &unit_test1));
        assert!(contain_unit(&units, &unit_test2));
    }

    fn create_unit(name: &str) -> Rc<UnitX> {
        logger::init_log_with_console("test_unit_load", 4);
        log::info!("test");
        let dm = Rc::new(DataManager::new());
        let file = Rc::new(UnitFile::new());
        let unit_conf_parser_mgr = Rc::new(UnitParserMgr::default());
        let unit_type = UnitType::UnitService;
        let plugins = Rc::clone(&Plugin::get_instance());
        let mut config_path1 = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        config_path1.push("../target/debug");
        plugins
            .borrow_mut()
            .set_library_dir(&config_path1.to_str().unwrap());
        plugins.borrow_mut().load_lib();
        let mut config_path2 = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        config_path2.push("../target/release");
        plugins
            .borrow_mut()
            .set_library_dir(&config_path2.to_str().unwrap());
        plugins.borrow_mut().load_lib();
        let subclass = plugins.borrow().create_unit_obj(unit_type).unwrap();
        Rc::new(UnitX::new(
            dm,
            file,
            unit_conf_parser_mgr,
            unit_type,
            name,
            subclass.into_unitobj(),
        ))
    }

    fn contain_unit(units: &Vec<Rc<UnitX>>, unit: &Rc<UnitX>) -> bool {
        for u in units.iter() {
            if Rc::ptr_eq(u, unit) {
                return true;
            }
        }

        false
    }
}
