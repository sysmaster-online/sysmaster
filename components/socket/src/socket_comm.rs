//! socket_comm模块提供公共对象的管理，主要包含对UnitManager和Unit对象的weak引用。
//! 需要调用公共对象提供的方法。
//!

use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use process1::manager::{Unit, UnitManager};

pub(super) struct SocketComm {
    data: RefCell<SocketCommData>,
}

impl SocketComm {
    pub(super) fn new() -> SocketComm {
        SocketComm {
            data: RefCell::new(SocketCommData::new()),
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

struct SocketCommData {
    unit: Weak<Unit>,
    um: Weak<UnitManager>,
}

// the declaration "pub(self)" is for identification only.
impl SocketCommData {
    pub(self) fn new() -> SocketCommData {
        SocketCommData {
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
