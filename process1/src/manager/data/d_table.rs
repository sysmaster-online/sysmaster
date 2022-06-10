use super::unit_dep_conf::UnitDepConf;
use super::unit_state::UnitState;
use crate::manager::table::{Table, TableSubscribe};
use std::cell::RefCell;
use std::rc::Rc;

pub struct DataManager {
    tables: (
        RefCell<Table<String, UnitDepConf>>, // unit-dep-config
        RefCell<Table<String, UnitState>>,   // unit-state
    ),
}

impl DataManager {
    pub fn new() -> DataManager {
        DataManager {
            tables: (RefCell::new(Table::new()), RefCell::new(Table::new())),
        }
    }

    pub(in crate::manager) fn insert_ud_config(
        &self,
        u_name: String,
        ud_config: UnitDepConf,
    ) -> Option<UnitDepConf> {
        let mut table = self.tables.0.borrow_mut();
        table.insert(u_name, ud_config)
    }

    pub(in crate::manager) fn register_ud_config(
        &self,
        name: &str,
        subscriber: Rc<dyn TableSubscribe<String, UnitDepConf>>,
    ) -> Option<Rc<dyn TableSubscribe<String, UnitDepConf>>> {
        let mut table = self.tables.0.borrow_mut();
        table.subscribe(name.to_string(), subscriber)
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
        name: &str,
        subscriber: Rc<dyn TableSubscribe<String, UnitState>>,
    ) -> Option<Rc<dyn TableSubscribe<String, UnitState>>> {
        let mut table = self.tables.1.borrow_mut();
        table.subscribe(name.to_string(), subscriber)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manager::data::{UnitActiveState, UnitNotifyFlags, UnitRelations};
    use crate::manager::table::TableOp;

    #[test]
    fn dm_unit_dep_config() {
        let dm = DataManager::new();
        let udc_sub = Rc::new(UnitDepConfigsTest::new());

        let ud_config = UnitDepConf::new();
        let old = dm.insert_ud_config(String::from("test"), ud_config);
        assert!(old.is_none());

        let mut ud_config = UnitDepConf::new();
        ud_config
            .deps
            .push((UnitRelations::UnitAfter, String::from("name")));
        let old = dm.insert_ud_config(String::from("test"), ud_config);
        assert_eq!(old.unwrap().deps.len(), 0);

        let sub = Rc::clone(&udc_sub);
        let old = dm.register_ud_config(&String::from("config"), sub);
        assert!(old.is_none());

        let mut ud_config = UnitDepConf::new();
        ud_config
            .deps
            .push((UnitRelations::UnitAfter, String::from("name")));
        dm.insert_ud_config(String::from("test"), ud_config);
        assert_eq!(udc_sub.len(), 1);
    }

    #[test]
    fn dm_unit_state() {
        let dm = DataManager::new();
        let os = UnitActiveState::UnitInActive;
        let ns = UnitActiveState::UnitActive;
        let flags = UnitNotifyFlags::UNIT_NOTIFY_RELOAD_FAILURE;
        let us_sub = Rc::new(UnitStatesTest::new(ns));

        let old = dm.insert_unit_state(String::from("test"), UnitState::new(os, ns, flags));
        assert!(old.is_none());

        let ns_ing = UnitActiveState::UnitActivating;
        let old = dm.insert_unit_state(String::from("test"), UnitState::new(os, ns_ing, flags));
        assert_eq!(old.unwrap().ns, ns);

        let sub = Rc::clone(&us_sub);
        let old = dm.register_unit_state(&String::from("state"), sub);
        assert!(old.is_none());

        let ns_m = UnitActiveState::UnitMaintenance;
        dm.insert_unit_state(String::from("test"), UnitState::new(os, ns_m, flags));
        assert_eq!(us_sub.get_ns(), ns_m);
    }

    struct UnitDepConfigsTest {
        len: RefCell<usize>,
    }

    impl UnitDepConfigsTest {
        fn new() -> UnitDepConfigsTest {
            UnitDepConfigsTest {
                len: RefCell::new(0),
            }
        }

        fn len(&self) -> usize {
            *self.len.borrow()
        }
    }

    impl TableSubscribe<String, UnitDepConf> for UnitDepConfigsTest {
        fn notify(&self, op: &TableOp<String, UnitDepConf>) {
            match op {
                TableOp::TableInsert(_, ud_conf) => {
                    *self.len.borrow_mut() = ud_conf.deps.len();
                }
                TableOp::TableRemove(_, _) => *self.len.borrow_mut() = 0,
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
        fn notify(&self, op: &TableOp<String, UnitState>) {
            match op {
                TableOp::TableInsert(_, state) => *self.ns.borrow_mut() = state.ns,
                TableOp::TableRemove(_, _) => {}
            }
        }
    }
}
