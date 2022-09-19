//! mount关联unit对象
//! * 需要通知Unit对象，方法变更
//! * 获取unit对象的属性
//! * 调用关系: mount_unit->mount_mng->mount_comm

use process1::manager::Unit;
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

pub(super) struct MountComm {
    data: RefCell<MountCommData>,
}

impl MountComm {
    pub(super) fn new() -> Self {
        MountComm {
            data: RefCell::new(MountCommData::new()),
        }
    }
    pub(super) fn attach_unit(&self, unit: Rc<Unit>) {
        self.data.borrow_mut().attach_unit(unit);
    }

    pub(super) fn unit(&self) -> Option<Rc<Unit>> {
        self.data.borrow().unit()
    }
}
struct MountCommData {
    unit: Weak<Unit>,
}

impl MountCommData {
    pub(self) fn new() -> MountCommData {
        MountCommData { unit: Weak::new() }
    }

    fn attach_unit(&mut self, unit: Rc<Unit>) {
        self.unit = Rc::downgrade(&unit);
    }

    pub(self) fn unit(&self) -> Option<Rc<Unit>> {
        self.unit.clone().upgrade()
    }
}
