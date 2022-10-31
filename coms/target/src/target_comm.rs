/*Associate the unit object
*You need to notify the Unit object and change the method
*Get the attributes of the unit object
*Call relation
*target_ unit->target_ mng->target_ comm
*/
use super::target_rentry::{TargetRe, TargetState};
use libsysmaster::manager::{Unit, UnitManager};
use libsysmaster::Reliability;
use once_cell::sync::Lazy;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::{Arc, RwLock};

pub(super) struct TargetUnitComm {
    data: RefCell<TargetUnitCommData>,
    umcomm: Arc<TargetUmComm>,
}

impl TargetUnitComm {
    pub(super) fn new() -> Self {
        TargetUnitComm {
            data: RefCell::new(TargetUnitCommData::new()),
            umcomm: TargetUmComm::get_instance(),
        }
    }

    pub(super) fn attach_unit(&self, unit: Rc<Unit>) {
        self.data.borrow_mut().attach_unit(unit);
    }

    pub(super) fn attach_um(&self, um: Rc<UnitManager>) {
        self.umcomm.attach_um(um)
    }

    pub(super) fn attach_reli(&self, reli: Rc<Reliability>) {
        self.umcomm.attach_reli(reli);
    }

    pub(super) fn unit(&self) -> Rc<Unit> {
        self.data.borrow().unit()
    }

    pub(super) fn um(&self) -> Rc<UnitManager> {
        self.umcomm.um()
    }

    pub(super) fn rentry_mng_insert(&self, state: TargetState) {
        self.rentry().mng_insert(self.unit().id(), state)
    }

    pub(super) fn rentry_mng_get(&self) -> Option<TargetState> {
        self.rentry().mng_get(self.unit().id())
    }

    fn rentry(&self) -> Rc<TargetRe> {
        self.umcomm.rentry()
    }
}
struct TargetUnitCommData {
    unit: Weak<Unit>,
}

impl TargetUnitCommData {
    pub(self) fn new() -> TargetUnitCommData {
        TargetUnitCommData { unit: Weak::new() }
    }

    pub(self) fn attach_unit(&mut self, unit: Rc<Unit>) {
        self.unit = Rc::downgrade(&unit);
    }

    pub(self) fn unit(&self) -> Rc<Unit> {
        self.unit.clone().upgrade().unwrap()
    }
}

static TARGET_UM_COMM: Lazy<Arc<TargetUmComm>> = Lazy::new(|| {
    let comm = TargetUmComm::new();
    Arc::new(comm)
});
pub(super) struct TargetUmComm {
    data: RwLock<TargetUmCommData>,
}

unsafe impl Send for TargetUmComm {}

unsafe impl Sync for TargetUmComm {}

impl TargetUmComm {
    pub(super) fn new() -> Self {
        TargetUmComm {
            data: RwLock::new(TargetUmCommData::new()),
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

    pub(super) fn get_instance() -> Arc<TargetUmComm> {
        TARGET_UM_COMM.clone()
    }

    pub(super) fn _reli(&self) -> Rc<Reliability> {
        let rdata = self.data.read().unwrap();
        rdata._reli()
    }

    pub(super) fn um(&self) -> Rc<UnitManager> {
        let rdata = self.data.read().unwrap();
        rdata.um()
    }

    pub(super) fn rentry(&self) -> Rc<TargetRe> {
        let rdata = self.data.read().unwrap();
        rdata.rentry()
    }
}

struct TargetUmCommData {
    // associated objects
    um: Weak<UnitManager>,
    _reli: Weak<Reliability>,
    rentry: Option<Rc<TargetRe>>,
}

// the declaration "pub(self)" is for identification only.
impl TargetUmCommData {
    pub(self) fn new() -> TargetUmCommData {
        TargetUmCommData {
            um: Weak::new(),
            _reli: Weak::new(),
            rentry: None,
        }
    }

    pub(self) fn attach_um(&mut self, um: Rc<UnitManager>) {
        let old = self.um.clone().upgrade();
        if old.is_none() {
            log::debug!("TargetUmComm attach_um action.");
            self.um = Rc::downgrade(&um);
        }
    }

    pub(self) fn attach_reli(&mut self, reli: Rc<Reliability>) {
        let old = self._reli.clone().upgrade();
        if old.is_none() {
            log::debug!("TargetUmComm attach_reli action.");
            self._reli = Rc::downgrade(&reli);
            self.rentry.replace(Rc::new(TargetRe::new(&reli)));
        }
    }

    pub(self) fn um(&self) -> Rc<UnitManager> {
        self.um.clone().upgrade().unwrap()
    }

    pub(self) fn _reli(&self) -> Rc<Reliability> {
        self._reli.clone().upgrade().unwrap()
    }

    pub(self) fn rentry(&self) -> Rc<TargetRe> {
        self.rentry.as_ref().cloned().unwrap()
    }
}
