use std::rc::Rc;
use std::cell::RefCell;
use std::collections::{HashMap};
use super::unit_entry::*;

#[derive(Debug)]
pub struct UnitSets {
    units: Rc<RefCell<HashMap<String, Rc<RefCell<Rc<UnitX>>>>>>,
}

impl UnitSets {
    pub fn new() -> UnitSets {
        UnitSets {
            units: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    pub fn insert_unit(&self, name: String, unit: Rc<RefCell<Rc<UnitX>>>) {
	    let mut units = self.units.borrow_mut();
	    units.insert(name, unit);
    }

    pub fn get_unit_on_name(&self, name: &str) -> Option<Rc<RefCell<Rc<UnitX>>>> {
        self.units.borrow_mut().get(name).and_then(|u| Some(u.clone()))
    }
}




