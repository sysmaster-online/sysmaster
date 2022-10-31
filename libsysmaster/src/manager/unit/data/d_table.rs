use super::unit_dep_conf::UnitDepConf;
use super::unit_state::UnitState;
use crate::manager::table::{Table, TableSubscribe};
use crate::reliability::ReStation;
use std::cell::RefCell;
use std::rc::Rc;

#[allow(clippy::type_complexity)]
pub struct DataManager {
    tables: (
        RefCell<Table<String, UnitDepConf>>, // [0]unit-dep-config
        RefCell<Table<String, UnitState>>,   // [1]unit-state
    ),
}

impl ReStation for DataManager {
    // no input, no compensate
    // no data

    // reload
    fn entry_clear(&self) {
        self.tables.0.borrow_mut().data_clear();
        self.tables.1.borrow_mut().data_clear();
    }
}

impl Drop for DataManager {
    fn drop(&mut self) {
        log::debug!("DataManager drop, clear.");
        // repeating protection
        self.clear();
    }
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
        {
            let old = self.tables.0.borrow_mut().insert(u_name, ud_config);
            old
        }
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

    // repeating protection
    pub(in crate::manager) fn clear(&self) {
        self.tables.0.borrow_mut().clear();
        self.tables.1.borrow_mut().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manager::table::TableOp;
    use crate::manager::unit::data::{UnitActiveState, UnitNotifyFlags};
    use crate::manager::UnitRelations;

    #[test]
    fn dm_unit_dep_config() {
        let dm = DataManager::new();
        let udc_sub = Rc::new(UnitDepConfigsTest::new());

        let ud_config = UnitDepConf::new();
        let old = dm.insert_ud_config(String::from("test"), ud_config);
        assert!(old.is_none());

        let mut ud_config = UnitDepConf::new();
        let vec = vec!["name".to_string()];
        ud_config.deps.insert(UnitRelations::UnitAfter, vec);
        let old = dm.insert_ud_config(String::from("test"), ud_config);
        assert_eq!(old.unwrap().deps.len(), 0);

        let sub = Rc::clone(&udc_sub);
        let old = dm.register_ud_config(&String::from("config"), sub);
        assert!(old.is_none());

        let mut ud_config = UnitDepConf::new();
        let vec = vec!["name".to_string()];
        ud_config.deps.insert(UnitRelations::UnitAfter, vec);
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
