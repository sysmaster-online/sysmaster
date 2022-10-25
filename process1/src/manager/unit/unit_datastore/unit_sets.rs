use crate::manager::table::{Table, TableSubscribe};
use crate::manager::unit::unit_entry::UnitX;
use crate::reliability::ReStation;
use std::cell::RefCell;
use std::rc::Rc;

pub(super) struct UnitSets {
    t: RefCell<Table<String, Rc<UnitX>>>,
}

impl ReStation for UnitSets {
    // no input, no compensate

    // data: special map

    // reload
    fn entry_clear(&self) {
        // unit_entry
        for unit in self.t.borrow().get_all().iter() {
            unit.entry_clear();
        }

        // table
        self.t.borrow_mut().data_clear();
    }
}

impl UnitSets {
    pub(super) fn new() -> UnitSets {
        UnitSets {
            t: RefCell::new(Table::new()),
        }
    }

    pub(super) fn insert(&self, name: String, unit: Rc<UnitX>) -> Option<Rc<UnitX>> {
        self.t.borrow_mut().insert(name, unit)
    }

    pub(super) fn remove(&self, name: &str) -> Option<Rc<UnitX>> {
        self.t.borrow_mut().remove(&name.to_string())
    }

    pub(super) fn get(&self, name: &str) -> Option<Rc<UnitX>> {
        self.t.borrow().get(&name.to_string()).cloned()
    }

    pub(super) fn get_all(&self) -> Vec<Rc<UnitX>> {
        self.t
            .borrow()
            .get_all()
            .iter()
            .map(|ur| Rc::clone(ur))
            .collect::<Vec<_>>()
    }

    pub(super) fn register(
        &self,
        sub_name: &str,
        subscriber: Rc<dyn TableSubscribe<String, Rc<UnitX>>>,
    ) -> Option<Rc<dyn TableSubscribe<String, Rc<UnitX>>>> {
        self.t
            .borrow_mut()
            .subscribe(sub_name.to_string(), subscriber)
    }

    pub(super) fn clear(&self) {
        self.t.borrow_mut().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manager::rentry::RELI_HISTORY_MAX_DBS;
    use crate::manager::unit::data::DataManager;
    use crate::manager::unit::uload_util::UnitFile;
    use crate::manager::unit::unit_rentry::{UnitRe, UnitType};
    use crate::plugin::Plugin;
    use crate::reliability::Reliability;
    use utils::logger;

    #[test]
    fn sets_insert() {
        let dm = Rc::new(DataManager::new());
        let reli = Rc::new(Reliability::new(RELI_HISTORY_MAX_DBS));
        let rentry = Rc::new(UnitRe::new(&reli));
        let sets = UnitSets::new();
        let name_test1 = String::from("test1.service");
        let unit_test1 = create_unit(&dm, &reli, &rentry, &name_test1);
        let name_test2 = String::from("test2.service");
        let unit_test2 = create_unit(&dm, &reli, &rentry, &name_test2);

        let old = sets.insert(name_test1.clone(), Rc::clone(&unit_test1));
        assert!(old.is_none());

        let old = sets.insert(name_test1, Rc::clone(&unit_test2));
        assert!(Rc::ptr_eq(&old.unwrap(), &unit_test1));

        let old = sets.insert(name_test2, Rc::clone(&unit_test2));
        assert!(old.is_none());
    }

    #[test]
    fn sets_remove() {
        let dm = Rc::new(DataManager::new());
        let reli = Rc::new(Reliability::new(RELI_HISTORY_MAX_DBS));
        let rentry = Rc::new(UnitRe::new(&reli));
        let sets = UnitSets::new();
        let name_test1 = String::from("test1.service");
        let unit_test1 = create_unit(&dm, &reli, &rentry, &name_test1);
        let name_test2 = String::from("test2.service");
        let unit_test2 = create_unit(&dm, &reli, &rentry, &name_test2);
        let name_test3 = String::from("test3.service");

        let old = sets.remove(&name_test1);
        assert!(old.is_none());

        sets.insert(name_test1.clone(), Rc::clone(&unit_test1));
        let old = sets.remove(&name_test1);
        assert!(Rc::ptr_eq(&old.unwrap(), &unit_test1));

        sets.insert(name_test1, Rc::clone(&unit_test1));
        sets.insert(name_test2.clone(), Rc::clone(&unit_test2));
        let old = sets.remove(&name_test3);
        assert!(old.is_none());
        let old = sets.remove(&name_test2);
        assert!(Rc::ptr_eq(&old.unwrap(), &unit_test2));
    }

    #[test]
    fn sets_get() {
        let dm = Rc::new(DataManager::new());
        let reli = Rc::new(Reliability::new(RELI_HISTORY_MAX_DBS));
        let rentry = Rc::new(UnitRe::new(&reli));
        let sets = UnitSets::new();
        let name_test1 = String::from("test1.service");
        let unit_test1 = create_unit(&dm, &reli, &rentry, &name_test1);
        let name_test2 = String::from("test2.service");
        let unit_test2 = create_unit(&dm, &reli, &rentry, &name_test2);

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
        let dm = Rc::new(DataManager::new());
        let reli = Rc::new(Reliability::new(RELI_HISTORY_MAX_DBS));
        let rentry = Rc::new(UnitRe::new(&reli));
        let sets = UnitSets::new();
        let name_test1 = String::from("test1.service");
        let unit_test1 = create_unit(&dm, &reli, &rentry, &name_test1);
        let name_test2 = String::from("test2.service");
        let unit_test2 = create_unit(&dm, &reli, &rentry, &name_test2);

        let units = sets.get_all();
        assert_eq!(units.len(), 0);

        sets.insert(name_test1.clone(), Rc::clone(&unit_test1));
        let units = sets.get_all();
        assert_eq!(units.len(), 1);
        assert!(contain_unit(&units, &unit_test1));
        sets.remove(&name_test1);
        let units = sets.get_all();
        assert_eq!(units.len(), 0);

        sets.insert(name_test1, Rc::clone(&unit_test1));
        sets.insert(name_test2, Rc::clone(&unit_test2));
        let units = sets.get_all();
        assert_eq!(units.len(), 2);
        assert!(contain_unit(&units, &unit_test1));
        assert!(contain_unit(&units, &unit_test2));
    }

    fn create_unit(
        dmr: &Rc<DataManager>,
        relir: &Rc<Reliability>,
        rentryr: &Rc<UnitRe>,
        name: &str,
    ) -> Rc<UnitX> {
        logger::init_log_with_console("test_unit_load", 4);
        log::info!("test");
        let file = Rc::new(UnitFile::new());
        let unit_type = UnitType::UnitService;
        let plugins = Plugin::get_instance();
        let subclass = plugins.create_unit_obj(unit_type).unwrap();
        subclass.attach_reli(Rc::clone(relir));
        Rc::new(UnitX::new(
            dmr,
            rentryr,
            &file,
            unit_type,
            name,
            subclass.into_unitobj(),
        ))
    }

    fn contain_unit(units: &[Rc<UnitX>], unit: &Rc<UnitX>) -> bool {
        for u in units.iter() {
            if Rc::ptr_eq(u, unit) {
                return true;
            }
        }

        false
    }
}
