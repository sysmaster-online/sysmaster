use super::unit_config::UnitConfig;
use super::unit_state::UnitState;
use crate::manager::table::{Table, TableSubscribe};
use std::cell::RefCell;
use std::rc::Rc;

pub(in crate::manager) struct DataManager {
    tables: (
        RefCell<Table<String, Rc<UnitConfig>>>, // unit-config
        RefCell<Table<String, UnitState>>,  // unit-state
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

    pub (in crate::manager) fn get_unit_config(&self,u_name:String)-> Option<Rc<UnitConfig>>{
        self.tables.0.borrow().get(&u_name).map(|v|Rc::clone(v))
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

    pub(in crate::manager) fn remove_unit_state(&self, u_name: &String) -> Option<UnitState> {
        let mut table = self.tables.1.borrow_mut();
        table.remove(u_name)
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
