use process1::manager::{Unit, UnitManager};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

pub(super) struct ServiceComm {
    data: RefCell<ServiceCommData>,
}

impl ServiceComm {
    pub(super) fn new() -> ServiceComm {
        ServiceComm {
            data: RefCell::new(ServiceCommData::new()),
        }
    }

    pub(super) fn attach_unit(&self, unit: Rc<Unit>) {
        self.data.borrow_mut().attach_unit(unit)
    }

    pub(super) fn attach_um(&self, um: Rc<UnitManager>) {
        self.data.borrow_mut().attach_um(um)
    }

    pub(super) fn unit(&self) -> Rc<Unit> {
        self.data.borrow().unit()
    }

    pub(super) fn um(&self) -> Rc<UnitManager> {
        self.data.borrow().um()
    }
}

struct ServiceCommData {
    unit: Weak<Unit>,
    um: Weak<UnitManager>,
}

// the declaration "pub(self)" is for identification only.
impl ServiceCommData {
    pub(self) fn new() -> ServiceCommData {
        ServiceCommData {
            unit: Weak::new(),
            um: Weak::new(),
        }
    }

    pub(self) fn attach_unit(&mut self, unit: Rc<Unit>) {
        self.unit = Rc::downgrade(&unit);
    }

    pub(self) fn attach_um(&mut self, um: Rc<UnitManager>) {
        self.um = Rc::downgrade(&um);
    }

    pub(self) fn unit(&self) -> Rc<Unit> {
        self.unit.clone().upgrade().unwrap()
    }

    pub(self) fn um(&self) -> Rc<UnitManager> {
        self.um.clone().upgrade().unwrap()
    }
}
