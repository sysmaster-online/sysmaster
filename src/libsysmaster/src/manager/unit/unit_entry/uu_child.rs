use super::uu_base::UeBase;
use crate::reliability::ReStation;
use nix::unistd::Pid;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

pub(super) struct UeChild {
    data: RefCell<UeChildData>,
}

impl ReStation for UeChild {
    // no input, no compensate

    // data
    fn db_map(&self) {
        self.data.borrow_mut().db_map();
    }

    fn db_insert(&self) {
        self.data.borrow().db_insert();
    }

    // reload: no external connections, no entry
}

impl UeChild {
    pub(super) fn new(baser: &Rc<UeBase>) -> UeChild {
        let child = UeChild {
            data: RefCell::new(UeChildData::new(baser)),
        };
        child.db_insert();
        child
    }

    pub(super) fn add_pids(&self, pid: Pid) {
        self.data.borrow_mut().add_pids(pid);
        self.db_update();
    }

    pub(super) fn remove_pids(&self, pid: Pid) {
        self.data.borrow_mut().remove_pids(pid);
        self.db_update();
    }
}

struct UeChildData {
    // associated objects
    base: Rc<UeBase>,

    // owned objects
    pids: HashSet<Pid>,
    sigchldgen: u64,
}

// the declaration "pub(self)" is for identification only.
impl UeChildData {
    pub(self) fn new(baser: &Rc<UeBase>) -> UeChildData {
        UeChildData {
            base: Rc::clone(baser),
            pids: HashSet::new(),
            sigchldgen: 0,
        }
    }

    pub(self) fn db_map(&mut self) {
        for pid in self.base.rentry_child_get().iter() {
            self.pids.insert(*pid);
        }
    }

    pub(self) fn add_pids(&mut self, pid: Pid) {
        self.pids.insert(pid);
    }

    pub(self) fn remove_pids(&mut self, pid: Pid) {
        self.pids.remove(&pid);
    }

    pub(self) fn db_insert(&self) {
        let pids: Vec<Pid> = self.pids.iter().copied().collect::<_>();
        self.base.rentry_child_insert(&pids);
    }
}
