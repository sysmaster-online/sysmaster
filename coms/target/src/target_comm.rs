/*Associate the unit object
*You need to notify the Unit object and change the method
*Get the attributes of the unit object
*Call relation
*target_ unit->target_ mng->target_ comm
*/
use super::target_rentry::{TargetRe, TargetState};
use libsysmaster::manager::{UmIf, Unit};
use libsysmaster::Reliability;
use once_cell::sync::Lazy;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::{Arc, RwLock};

pub(super) struct TargetUnitComm {
    owner: RefCell<Option<Rc<Unit>>>,
    umcomm: Arc<TargetUmComm>,
}

impl TargetUnitComm {
    pub(super) fn new() -> Self {
        TargetUnitComm {
            owner: RefCell::new(None),
            umcomm: TargetUmComm::get_instance(),
        }
    }

    pub(super) fn attach_um(&self, um: Rc<dyn UmIf>) {
        self.umcomm.attach_um(um)
    }

    pub(super) fn attach_unit(&self, unit: Rc<libsysmaster::manager::Unit>) {
        self.owner.replace(Some(unit));
    }

    pub(super) fn owner(&self) -> Option<Rc<Unit>> {
        if let Some(ref unit) = *self.owner.borrow() {
            Some(Rc::clone(unit))
        } else {
            None
        }
    }

    pub(super) fn attach_reli(&self, reli: Rc<Reliability>) {
        self.umcomm.attach_reli(reli);
    }

    pub(super) fn rentry_mng_insert(&self, state: TargetState) {
        self.owner()
            .map(|u| self.rentry().mng_insert(u.id(), state));
    }

    pub(super) fn rentry_mng_get(&self) -> Option<TargetState> {
        let ret = self.owner().map(|u| self.rentry().mng_get(u.id()));
        ret.unwrap_or(None)
    }

    pub(super) fn rentry(&self) -> Rc<TargetRe> {
        self.umcomm.rentry()
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

    pub(super) fn rentry(&self) -> Rc<TargetRe> {
        let rdata = self.data.read().unwrap();
        rdata.rentry()
    }

    pub(super) fn attach_um(&self, um: Rc<dyn UmIf>) {
        let mut wdata = self.data.write().unwrap();
        wdata.attach_um(um);
    }
}

struct TargetUmCommData {
    // associated objects
    um: Option<Rc<dyn UmIf>>,
    _reli: Weak<Reliability>,
    rentry: Option<Rc<TargetRe>>,
}

// the declaration "pub(self)" is for identification only.
impl TargetUmCommData {
    pub(self) fn new() -> TargetUmCommData {
        TargetUmCommData {
            um: None,
            _reli: Weak::new(),
            rentry: None,
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

    pub(self) fn attach_um(&mut self, um: Rc<dyn UmIf>) {
        if self.um.is_none() {
            log::debug!("TargetUmComm attach_um action.");
            self.um = Some(um)
        }
    }

    pub(self) fn _reli(&self) -> Rc<Reliability> {
        self._reli.clone().upgrade().unwrap()
    }

    pub(self) fn rentry(&self) -> Rc<TargetRe> {
        self.rentry.as_ref().cloned().unwrap()
    }
}
