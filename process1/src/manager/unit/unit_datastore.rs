use super::unit_entry::UnitX;
use std::cell::RefCell;
use std::rc::Rc;

use crate::manager::table::Table;

//UnitStorage composition of units with hash map
#[derive(Debug)]
pub struct UnitDb {
    unit_tab: RefCell<Table<String, Rc<UnitX>>>, // store all loaded unit
    unit_id_tab: RefCell<Table<String, Rc<String>>>, //store map of unit and config file
}

impl UnitDb {
    pub(in crate::manager) fn new() -> UnitDb {
        UnitDb {
            unit_tab: RefCell::new(Table::new()),
            unit_id_tab: RefCell::new(Table::new()),
        }
    }

    /*pub fn get_instance() -> Rc<RefCell<UnitStorage<K,V>>> {
        static mut PLUGIN: Option<Rc<RefCell<UnitStorage<K,V>>>> = None;
        unsafe {
            PLUGIN
                .get_or_insert_with(|| {
                    let mut unitStorage: UnitStorage<K, V> = Self::new();
                    Rc::new(RefCell::new(unitStorage))
                })
                .clone()
        }
    }*/

    pub fn insert_unit(&self, name: String, unitx: Rc<UnitX>) -> Option<Rc<UnitX>> {
        let mut unit_tab = self.unit_tab.borrow_mut();
        unit_tab.insert(name, unitx)
    }

    pub fn get_unit_by_name(&self, name: &String) -> Option<Rc<UnitX>> {
        let value = self.unit_tab.borrow().get(name).cloned();
        value
    }

    pub fn insert_unit_config_file(&self, name: String, value: Rc<String>) -> Option<Rc<String>> {
        self.unit_id_tab.borrow_mut().insert(name, value)
    }

    pub fn get_unit_config_file(&self, unit_name: &String) -> Option<Rc<String>> {
        let v = self.unit_id_tab.borrow().get(unit_name).cloned();
        v
    }
}
