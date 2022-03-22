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

    pub(super) fn remove(&mut self, name: &str) -> Option<Rc<UnitX>> {
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
