// Copyright (c) 2022 Huawei Technologies Co.,Ltd. All rights reserved.
//
// sysMaster is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan
// PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY
// KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

//!  socket_ The comm module provides management of common objects, mainly including weak references to UnitManager and Unit objects.
//!  The method provided by the public object needs to be called.
//!
use super::rentry::{PortType, SectionSocket, SocketCommand, SocketRe, SocketResult, SocketState};
use core::rel::Reliability;
use core::unit::{UmIf, UnitBase};
use nix::unistd::Pid;
use once_cell::sync::Lazy;
use std::cell::RefCell;
use std::os::unix::prelude::RawFd;
use std::rc::{Rc, Weak};
use std::sync::{Arc, RwLock};

pub(crate) struct SocketUnitComm {
    owner: RefCell<Option<Weak<dyn UnitBase>>>,
    umcomm: Arc<SocketUmComm>,
}

impl SocketUnitComm {
    pub(super) fn new() -> Self {
        SocketUnitComm {
            owner: RefCell::new(None),
            umcomm: SocketUmComm::get_instance(),
        }
    }

    pub(super) fn attach_unit(&self, unit: Rc<dyn UnitBase>) {
        self.owner.replace(Some(Rc::downgrade(&unit)));
    }

    pub(super) fn attach_um(&self, um: Rc<dyn UmIf>) {
        self.umcomm.attach_um(um)
    }

    pub(super) fn attach_reli(&self, reli: Rc<Reliability>) {
        self.umcomm.attach_reli(reli)
    }

    pub(super) fn owner(&self) -> Option<Rc<dyn UnitBase>> {
        if let Some(ref unit) = *self.owner.borrow() {
            unit.upgrade()
        } else {
            None
        }
    }

    pub(super) fn um(&self) -> Rc<dyn UmIf> {
        self.umcomm.um()
    }

    pub(super) fn reli(&self) -> Rc<Reliability> {
        self.umcomm.reli()
    }

    pub(super) fn rentry(&self) -> Rc<SocketRe> {
        self.umcomm.rentry()
    }

    pub(super) fn rentry_conf_insert(&self, socket: &SectionSocket, service: Option<String>) {
        if let Some(u) = self.owner() {
            self.rentry().conf_insert(&u.id(), socket, service)
        }
    }

    pub(super) fn rentry_conf_get(&self) -> Option<(SectionSocket, Option<String>)> {
        self.owner().map(|u| self.rentry().conf_get(&u.id()))?
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
        ports: Vec<(PortType, String, RawFd)>,
    ) {
        if let Some(u) = self.owner() {
            self.rentry().mng_insert(
                &u.id(),
                state,
                result,
                control_pid,
                control_cmd_type,
                control_cmd_len,
                refused,
                ports,
            )
        };
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
        Vec<(PortType, String, RawFd)>,
    )> {
        self.owner().map(|u| self.rentry().mng_get(&u.id()))?
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

    pub(super) fn attach_um(&self, um: Rc<dyn UmIf>) {
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

    pub(super) fn um(&self) -> Rc<dyn UmIf> {
        let rdata = self.data.read().unwrap();
        rdata.um().unwrap()
    }

    pub(super) fn rentry(&self) -> Rc<SocketRe> {
        let rdata = self.data.read().unwrap();
        rdata.rentry()
    }
}

struct SocketUmCommData {
    // associated objects
    um: Option<Rc<dyn UmIf>>,
    reli: Weak<Reliability>,
    rentry: Option<Rc<SocketRe>>,
}

// the declaration "pub(self)" is for identification only.
impl SocketUmCommData {
    pub(self) fn new() -> SocketUmCommData {
        SocketUmCommData {
            um: None,
            reli: Weak::new(),
            rentry: None,
        }
    }

    pub(self) fn attach_um(&mut self, um: Rc<dyn UmIf>) {
        if self.um.is_none() {
            log::debug!("SocketUmComm attach_um action.");
            self.um = Some(um)
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

    pub(self) fn um(&self) -> Option<Rc<dyn UmIf>> {
        if let Some(ref um) = self.um {
            Some(Rc::clone(um))
        } else {
            None
        }
    }

    pub(self) fn reli(&self) -> Rc<Reliability> {
        self.reli.clone().upgrade().unwrap()
    }

    pub(self) fn rentry(&self) -> Rc<SocketRe> {
        self.rentry.as_ref().cloned().unwrap()
    }
}
