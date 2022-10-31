//!  socket_ The comm module provides management of common objects, mainly including weak references to UnitManager and Unit objects.
//!  The method provided by the public object needs to be called.
//!
use super::socket_rentry::{SectionSocket, SocketCommand, SocketRe, SocketResult, SocketState};
use libsysmaster::manager::{Unit, UnitManager};
use libsysmaster::Reliability;
use nix::unistd::Pid;
use once_cell::sync::Lazy;
use std::cell::RefCell;
use std::os::unix::prelude::RawFd;
use std::rc::{Rc, Weak};
use std::sync::{Arc, RwLock};

pub(super) struct SocketUnitComm {
    data: RefCell<SocketUnitCommData>,
    umcomm: Arc<SocketUmComm>,
}

impl SocketUnitComm {
    pub(super) fn new() -> Self {
        SocketUnitComm {
            data: RefCell::new(SocketUnitCommData::new()),
            umcomm: SocketUmComm::get_instance(),
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

    pub(super) fn um(&self) -> Rc<UnitManager> {
        self.umcomm.um()
    }

    pub(super) fn reli(&self) -> Rc<Reliability> {
        self.umcomm.reli()
    }

    pub(super) fn rentry(&self) -> Rc<SocketRe> {
        self.umcomm.rentry()
    }

    pub(super) fn rentry_conf_insert(&self, socket: &SectionSocket, service: Option<String>) {
        self.rentry().conf_insert(self.unit().id(), socket, service);
    }

    pub(super) fn rentry_conf_get(&self) -> Option<(SectionSocket, Option<String>)> {
        self.rentry().conf_get(self.unit().id())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn rentry_mng_insert(
        &self,
        state: SocketState,
        result: SocketResult,
        control_pid: Option<Pid>,
        control_cmd_type: Option<SocketCommand>,
        control_cmd_len: usize,
        refused: i32,
        ports: Vec<RawFd>,
    ) {
        self.rentry().mng_insert(
            self.unit().id(),
            state,
            result,
            control_pid,
            control_cmd_type,
            control_cmd_len,
            refused,
            ports,
        );
    }

    #[allow(clippy::type_complexity)]
    pub(super) fn rentry_mng_get(
        &self,
    ) -> Option<(
        SocketState,
        SocketResult,
        Option<Pid>,
        Option<SocketCommand>,
        usize,
        i32,
        Vec<RawFd>,
    )> {
        self.rentry().mng_get(self.unit().id())
    }
}

struct SocketUnitCommData {
    unit: Weak<Unit>,
}

impl SocketUnitCommData {
    pub(self) fn new() -> SocketUnitCommData {
        SocketUnitCommData { unit: Weak::new() }
    }

    fn attach_unit(&mut self, unit: Rc<Unit>) {
        self.unit = Rc::downgrade(&unit);
    }

    pub(self) fn unit(&self) -> Rc<Unit> {
        self.unit.clone().upgrade().unwrap()
    }
}

static SOCKET_UM_COMM: Lazy<Arc<SocketUmComm>> = Lazy::new(|| {
    let comm = SocketUmComm::new();
    Arc::new(comm)
});

pub(super) struct SocketUmComm {
    data: RwLock<SocketUmCommData>,
}

unsafe impl Send for SocketUmComm {}

unsafe impl Sync for SocketUmComm {}

impl SocketUmComm {
    pub(super) fn new() -> Self {
        SocketUmComm {
            data: RwLock::new(SocketUmCommData::new()),
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

    pub(super) fn get_instance() -> Arc<SocketUmComm> {
        SOCKET_UM_COMM.clone()
    }

    pub(super) fn reli(&self) -> Rc<Reliability> {
        let rdata = self.data.read().unwrap();
        rdata.reli()
    }

    pub(super) fn um(&self) -> Rc<UnitManager> {
        let rdata = self.data.read().unwrap();
        rdata.um()
    }

    pub(super) fn rentry(&self) -> Rc<SocketRe> {
        let rdata = self.data.read().unwrap();
        rdata.rentry()
    }
}

struct SocketUmCommData {
    // associated objects
    um: Weak<UnitManager>,
    reli: Weak<Reliability>,
    rentry: Option<Rc<SocketRe>>,
}

// the declaration "pub(self)" is for identification only.
impl SocketUmCommData {
    pub(self) fn new() -> SocketUmCommData {
        SocketUmCommData {
            um: Weak::new(),
            reli: Weak::new(),
            rentry: None,
        }
    }

    pub(self) fn attach_um(&mut self, um: Rc<UnitManager>) {
        let old = self.um.clone().upgrade();
        if old.is_none() {
            log::debug!("SocketUmComm attach_um action.");
            self.um = Rc::downgrade(&um);
        }
    }

    pub(self) fn attach_reli(&mut self, reli: Rc<Reliability>) {
        let old = self.reli.clone().upgrade();
        if old.is_none() {
            log::debug!("SocketUmComm attach_reli action.");
            self.reli = Rc::downgrade(&reli);
            self.rentry.replace(Rc::new(SocketRe::new(&reli)));
        }
    }

    pub(self) fn um(&self) -> Rc<UnitManager> {
        self.um.clone().upgrade().unwrap()
    }

    pub(self) fn reli(&self) -> Rc<Reliability> {
        self.reli.clone().upgrade().unwrap()
    }

    pub(self) fn rentry(&self) -> Rc<SocketRe> {
        self.rentry.as_ref().cloned().unwrap()
    }
}
