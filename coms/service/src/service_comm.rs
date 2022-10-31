use super::service_rentry::{
    NotifyState, SectionService, ServiceCommand, ServiceRe, ServiceResult, ServiceState,
};
use libsysmaster::manager::{Unit, UnitManager};
use libsysmaster::Reliability;
use nix::unistd::Pid;
use once_cell::sync::Lazy;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::{Arc, RwLock};

pub(super) struct ServiceUnitComm {
    data: RefCell<ServiceUnitCommData>,
    umcomm: Arc<ServiceUmComm>,
}

impl ServiceUnitComm {
    pub(super) fn new() -> Self {
        ServiceUnitComm {
            data: RefCell::new(ServiceUnitCommData::new()),
            umcomm: ServiceUmComm::get_instance(),
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

    pub(super) fn rentry_conf_insert(&self, service: &SectionService) {
        self.rentry().conf_insert(self.unit().id(), service);
    }

    pub(super) fn rentry_conf_get(&self) -> Option<SectionService> {
        self.rentry().conf_get(self.unit().id())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn rentry_mng_insert(
        &self,
        state: ServiceState,
        result: ServiceResult,
        main_pid: Option<Pid>,
        control_pid: Option<Pid>,
        main_cmd_len: usize,
        control_cmd_type: Option<ServiceCommand>,
        control_cmd_len: usize,
        notify_state: NotifyState,
    ) {
        self.rentry().mng_insert(
            self.unit().id(),
            state,
            result,
            main_pid,
            control_pid,
            main_cmd_len,
            control_cmd_type,
            control_cmd_len,
            notify_state,
        );
    }

    #[allow(clippy::type_complexity)]
    pub(super) fn rentry_mng_get(
        &self,
    ) -> Option<(
        ServiceState,
        ServiceResult,
        Option<Pid>,
        Option<Pid>,
        usize,
        Option<ServiceCommand>,
        usize,
        NotifyState,
    )> {
        self.rentry().mng_get(self.unit().id())
    }

    pub(super) fn _reli(&self) -> Rc<Reliability> {
        self.umcomm._reli()
    }

    fn rentry(&self) -> Rc<ServiceRe> {
        self.umcomm.rentry()
    }
}

struct ServiceUnitCommData {
    unit: Weak<Unit>,
}

impl ServiceUnitCommData {
    pub(self) fn new() -> ServiceUnitCommData {
        ServiceUnitCommData { unit: Weak::new() }
    }

    fn attach_unit(&mut self, unit: Rc<Unit>) {
        self.unit = Rc::downgrade(&unit);
    }

    pub(self) fn unit(&self) -> Rc<Unit> {
        self.unit.clone().upgrade().unwrap()
    }
}

static SERVICE_UM_COMM: Lazy<Arc<ServiceUmComm>> = Lazy::new(|| {
    let comm = ServiceUmComm::new();
    Arc::new(comm)
});

pub(super) struct ServiceUmComm {
    data: RwLock<ServiceUmCommData>,
}

unsafe impl Send for ServiceUmComm {}

unsafe impl Sync for ServiceUmComm {}

impl ServiceUmComm {
    pub(super) fn new() -> Self {
        ServiceUmComm {
            data: RwLock::new(ServiceUmCommData::new()),
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

    pub(super) fn get_instance() -> Arc<ServiceUmComm> {
        SERVICE_UM_COMM.clone()
    }

    pub(super) fn _reli(&self) -> Rc<Reliability> {
        let rdata = self.data.read().unwrap();
        rdata._reli()
    }

    pub(super) fn um(&self) -> Rc<UnitManager> {
        let rdata = self.data.read().unwrap();
        rdata.um()
    }

    pub(super) fn rentry(&self) -> Rc<ServiceRe> {
        let rdata = self.data.read().unwrap();
        rdata.rentry()
    }
}

struct ServiceUmCommData {
    // associated objects
    um: Weak<UnitManager>,
    _reli: Weak<Reliability>,
    rentry: Option<Rc<ServiceRe>>,
}

// the declaration "pub(self)" is for identification only.
impl ServiceUmCommData {
    pub(self) fn new() -> ServiceUmCommData {
        ServiceUmCommData {
            um: Weak::new(),
            _reli: Weak::new(),
            rentry: None,
        }
    }

    pub(self) fn attach_um(&mut self, um: Rc<UnitManager>) {
        let old = self.um.clone().upgrade();
        if old.is_none() {
            log::debug!("ServiceUmComm attach_um action.");
            self.um = Rc::downgrade(&um);
        }
    }

    pub(self) fn attach_reli(&mut self, reli: Rc<Reliability>) {
        let old = self._reli.clone().upgrade();
        if old.is_none() {
            log::debug!("ServiceUmComm attach_reli action.");
            self._reli = Rc::downgrade(&reli);
            self.rentry.replace(Rc::new(ServiceRe::new(&reli)));
        }
    }

    pub(self) fn um(&self) -> Rc<UnitManager> {
        self.um.clone().upgrade().unwrap()
    }

    pub(self) fn _reli(&self) -> Rc<Reliability> {
        self._reli.clone().upgrade().unwrap()
    }

    pub(self) fn rentry(&self) -> Rc<ServiceRe> {
        self.rentry.as_ref().cloned().unwrap()
    }
}
