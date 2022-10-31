//!  Mount association unit object
//! *  You need to notify the Unit object and change the method
//! *  Get the attributes of the unit object
//! *  Call relation: mount_ unit->mount_ mng->mount_ comm

use super::mount_rentry::{MountRe, MountState};
use libsysmaster::manager::{Unit, UnitManager};
use libsysmaster::Reliability;
use once_cell::sync::Lazy;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::{Arc, RwLock};

pub(super) struct MountUnitComm {
    data: RefCell<MountUnitCommData>,
    umcomm: Arc<MountUmComm>,
}

impl MountUnitComm {
    pub(super) fn new() -> Self {
        MountUnitComm {
            data: RefCell::new(MountUnitCommData::new()),
            umcomm: MountUmComm::get_instance(),
        }
    }

    pub(super) fn attach_unit(&self, unit: Rc<Unit>) {
        self.data.borrow_mut().attach_unit(unit);
    }

    pub(super) fn attach_um(&self, um: Rc<UnitManager>) {
        self.umcomm.attach_um(um)
    }

    pub(super) fn attach_reli(&self, reli: Rc<Reliability>) {
        self.umcomm.attach_reli(reli)
    }

    pub(super) fn unit(&self) -> Rc<Unit> {
        self.data.borrow().unit()
    }

    pub(super) fn rentry_mng_insert(&self, state: MountState) {
        self.rentry().mng_insert(self.unit().id(), state)
    }

    pub(super) fn rentry_mng_get(&self) -> Option<MountState> {
        self.rentry().mng_get(self.unit().id())
    }

    fn rentry(&self) -> Rc<MountRe> {
        self.umcomm.rentry()
    }

    pub(super) fn _um(&self) -> Rc<UnitManager> {
        self.umcomm.um()
    }
}
struct MountUnitCommData {
    unit: Weak<Unit>,
}

impl MountUnitCommData {
    pub(self) fn new() -> MountUnitCommData {
        MountUnitCommData { unit: Weak::new() }
    }

    fn attach_unit(&mut self, unit: Rc<Unit>) {
        self.unit = Rc::downgrade(&unit);
    }

    pub(self) fn unit(&self) -> Rc<Unit> {
        self.unit.clone().upgrade().unwrap()
    }
}

static MOUNT_UM_COMM: Lazy<Arc<MountUmComm>> = Lazy::new(|| {
    let comm = MountUmComm::new();
    Arc::new(comm)
});

pub(super) struct MountUmComm {
    data: RwLock<MountUmCommData>,
}

unsafe impl Send for MountUmComm {}

unsafe impl Sync for MountUmComm {}

impl MountUmComm {
    pub(super) fn new() -> Self {
        MountUmComm {
            data: RwLock::new(MountUmCommData::new()),
        }
    }

    pub(super) fn attach_um(&self, um: Rc<UnitManager>) {
        let mut wdata = self.data.write().unwrap();
        wdata.attach_um(um);
    }

    pub(super) fn attach_reli(&self, reli: Rc<Reliability>) {
        let mut wdata = self.data.write().unwrap();
        wdata.attach_reli(reli);
    }

    pub(super) fn get_instance() -> Arc<MountUmComm> {
        MOUNT_UM_COMM.clone()
    }

    pub(super) fn reli(&self) -> Rc<Reliability> {
        let rdata = self.data.read().unwrap();
        rdata.reli()
    }

    pub(super) fn um(&self) -> Rc<UnitManager> {
        let rdata = self.data.read().unwrap();
        rdata.um()
    }

    pub(super) fn rentry(&self) -> Rc<MountRe> {
        let rdata = self.data.read().unwrap();
        rdata.rentry()
    }
}

struct MountUmCommData {
    // associated objects
    um: Weak<UnitManager>,
    reli: Weak<Reliability>,
    rentry: Option<Rc<MountRe>>,
}

// the declaration "pub(self)" is for identification only.
impl MountUmCommData {
    pub(self) fn new() -> MountUmCommData {
        MountUmCommData {
            um: Weak::new(),
            reli: Weak::new(),
            rentry: None,
        }
    }

    pub(self) fn attach_um(&mut self, um: Rc<UnitManager>) {
        let old = self.um.clone().upgrade();
        if old.is_none() {
            log::debug!("MountUmComm attach_um action.");
            self.um = Rc::downgrade(&um);
        }
    }

    pub(self) fn attach_reli(&mut self, reli: Rc<Reliability>) {
        let old = self.reli.clone().upgrade();
        if old.is_none() {
            log::debug!("MountUmComm attach_reli action.");
            self.reli = Rc::downgrade(&reli);
            self.rentry.replace(Rc::new(MountRe::new(&reli)));
        }
    }

    pub(self) fn um(&self) -> Rc<UnitManager> {
        self.um.clone().upgrade().unwrap()
    }

    pub(self) fn reli(&self) -> Rc<Reliability> {
        self.reli.clone().upgrade().unwrap()
    }

    pub(self) fn rentry(&self) -> Rc<MountRe> {
        self.rentry.as_ref().cloned().unwrap()
    }
}
