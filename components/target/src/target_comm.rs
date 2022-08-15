/* 关联unit对象
*需要通知Unit对象，方法变更
*获取unit对象的属性
*调用关系
*target_unit->target_mng->target_comm
*/

use process1::manager::{Unit, UnitManager};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

pub(super) struct TargetComm {
    data: RefCell<TargetCommData>,
}

impl TargetComm {
    pub(super) fn new() -> Self {
        TargetComm {
            data: RefCell::new(TargetCommData::new()),
        }
    }
    pub(super) fn attach_unit(&self, unit: Rc<Unit>) {
        self.data.borrow_mut().attach_unit(unit);
    }

    pub(super) fn unit(&self) -> Option<Rc<Unit>> {
        self.data.borrow().unit()
    }

    pub(super) fn attach_um(&self, um: Rc<UnitManager>) {
        self.data.borrow_mut().attach_um(um)
    }

    pub(super) fn um(&self) -> Option<Rc<UnitManager>> {
        self.data.borrow().um()
    }
}
struct TargetCommData {
    unit: Weak<Unit>,
    um: Weak<UnitManager>,
}

impl TargetCommData {
    pub(self) fn new() -> TargetCommData {
        TargetCommData {
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

    pub(self) fn unit(&self) -> Option<Rc<Unit>> {
        self.unit.clone().upgrade()
    }

    pub(self) fn um(&self) -> Option<Rc<UnitManager>> {
        self.um.clone().upgrade()
    }
}
