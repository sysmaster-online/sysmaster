use super::unit_config::UnitConfig;
use super::unit_state::UnitState;
use crate::manager::table::{Table, TableSubscribe};
use std::cell::RefCell;
use std::rc::Rc;

pub(in crate::manager) struct DataManager {
    tables: (
        RefCell<Table<String, Rc<UnitConfig>>>, // unit-config
        RefCell<Table<String, UnitState>>,      // unit-state
    ),
}

impl DataManager {
    pub(in crate::manager) fn new() -> DataManager {
        DataManager {
            tables: (RefCell::new(Table::new()), RefCell::new(Table::new())),
        }
    }

    pub(in crate::manager) fn insert_unit_config(
        &self,
        u_name: String,
        u_config: Rc<UnitConfig>,
    ) -> Option<Rc<UnitConfig>> {
        let mut table = self.tables.0.borrow_mut();
        table.insert(u_name, u_config)
    }

    pub(in crate::manager) fn get_unit_config(&self, u_name: String) -> Option<Rc<UnitConfig>> {
        self.tables.0.borrow().get(&u_name).map(|v| Rc::clone(v))
    }
    pub(in crate::manager) fn remove_unit_config(&self, u_name: &String) -> Option<Rc<UnitConfig>> {
        let mut table = self.tables.0.borrow_mut();
        table.remove(u_name)
    }

    pub(in crate::manager) fn register_unit_config(
        &self,
        name: String,
        subscriber: Rc<dyn TableSubscribe<String, Rc<UnitConfig>>>,
    ) -> Option<Rc<dyn TableSubscribe<String, Rc<UnitConfig>>>> {
        let mut table = self.tables.0.borrow_mut();
        table.subscribe(name, subscriber)
    }

    pub(in crate::manager) fn insert_unit_state(
        &self,
        u_name: String,
        u_state: UnitState,
    ) -> Option<UnitState> {
        let mut table = self.tables.1.borrow_mut();
        table.insert(u_name, u_state)
    }

    pub(in crate::manager) fn register_unit_state(
        &self,
        name: String,
        subscriber: Rc<dyn TableSubscribe<String, UnitState>>,
    ) -> Option<Rc<dyn TableSubscribe<String, UnitState>>> {
        let mut table = self.tables.1.borrow_mut();
        table.subscribe(name, subscriber)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manager::data::{UnitActiveState, UnitNotifyFlags};
    use crate::manager::table::TableOp;

    #[test]
    fn dm_unit_config() {
        let dm = DataManager::new();
        let uc_sub = Rc::new(UnitConfigsTest::new());

        let unit_config = UnitConfig::new();
        let old = dm.insert_unit_config(String::from("test"), Rc::new(unit_config));
        assert!(old.is_none());

        let mut unit_config = UnitConfig::new();
        unit_config.set_name(String::from("name1"));
        let old = dm.insert_unit_config(String::from("test"), Rc::new(unit_config));
        assert_eq!(old.unwrap().get_name(), String::from(""));

        let get = dm.get_unit_config(String::from("test"));
        assert_eq!(get.unwrap().get_name(), String::from("name1"));

        let old = dm.remove_unit_config(&String::from("test"));
        assert_eq!(old.unwrap().get_name(), String::from("name1"));

        let sub = Rc::clone(&uc_sub);
        let old = dm.register_unit_config(String::from("config"), sub);
        assert!(old.is_none());

        let mut unit_config = UnitConfig::new();
        unit_config.set_name(String::from("name2"));
        dm.insert_unit_config(String::from("test"), Rc::new(unit_config));
        assert_eq!(uc_sub.get_name(), String::from("name2"));
    }

    #[test]
    fn dm_unit_state() {
        let dm = DataManager::new();
        let os = UnitActiveState::UnitInActive;
        let ns = UnitActiveState::UnitActive;
        let flags = UnitNotifyFlags::UnitNotifyReloadFailure as isize;
        let us_sub = Rc::new(UnitStatesTest::new(ns));

        let old = dm.insert_unit_state(String::from("test"), UnitState::new(os, ns, flags));
        assert!(old.is_none());

        let ns_ing = UnitActiveState::UnitActivating;
        let old = dm.insert_unit_state(String::from("test"), UnitState::new(os, ns_ing, flags));
        assert_eq!(old.unwrap().get_ns(), ns);

        let sub = Rc::clone(&us_sub);
        let old = dm.register_unit_state(String::from("state"), sub);
        assert!(old.is_none());

        let ns_m = UnitActiveState::UnitMaintenance;
        dm.insert_unit_state(String::from("test"), UnitState::new(os, ns_m, flags));
        assert_eq!(us_sub.get_ns(), ns_m);
    }

    struct UnitConfigsTest {
        name: RefCell<String>,
    }

    impl UnitConfigsTest {
        fn new() -> UnitConfigsTest {
            UnitConfigsTest {
                name: RefCell::new(String::from("")),
            }
        }

        fn get_name(&self) -> String {
            self.name.borrow().clone()
        }
    }

    impl TableSubscribe<String, Rc<UnitConfig>> for UnitConfigsTest {
        fn filter(&self, _op: &TableOp<String, Rc<UnitConfig>>) -> bool {
            true
        }

        fn notify(&self, op: &TableOp<String, Rc<UnitConfig>>) {
            match op {
                TableOp::TableInsert(_, config) => {
                    *self.name.borrow_mut() = config.get_name().to_string()
                }
                TableOp::TableRemove(_, _) => *self.name.borrow_mut() = String::from(""),
            }
        }
    }

    struct UnitStatesTest {
        ns: RefCell<UnitActiveState>,
    }

    impl UnitStatesTest {
        fn new(ns: UnitActiveState) -> UnitStatesTest {
            UnitStatesTest {
                ns: RefCell::new(ns),
            }
        }

        fn get_ns(&self) -> UnitActiveState {
            *self.ns.borrow()
        }
    }

    impl TableSubscribe<String, UnitState> for UnitStatesTest {
        fn filter(&self, _op: &TableOp<String, UnitState>) -> bool {
            true
        }

        fn notify(&self, op: &TableOp<String, UnitState>) {
            match op {
                TableOp::TableInsert(_, state) => *self.ns.borrow_mut() = state.get_ns(),
                TableOp::TableRemove(_, _) => {}
            }
        }
    }
}
