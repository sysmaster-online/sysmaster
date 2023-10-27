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

use super::rentry::{
    NotifyState, SectionService, ServiceCommand, ServiceRe, ServiceResult, ServiceState,
};
use crate::monitor::ServiceMonitor;
use crate::rentry::ExitStatus;
use core::rel::Reliability;
use core::unit::{UmIf, UnitBase};
use log::Level;
use nix::unistd::Pid;
use once_cell::sync::Lazy;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::{Arc, RwLock};

pub(super) struct ServiceUnitComm {
    owner: RefCell<Option<Weak<dyn UnitBase>>>,
    umcomm: Arc<ServiceUmComm>,
}

impl ServiceUnitComm {
    pub(super) fn new() -> Self {
        ServiceUnitComm {
            owner: RefCell::new(None),
            umcomm: ServiceUmComm::get_instance(),
        }
    }

    pub(super) fn attach_unit(&self, unit: Rc<dyn UnitBase>) {
        self.owner.replace(Some(Rc::downgrade(&unit)));
    }

    pub(super) fn attach_um(&self, um: Rc<dyn UmIf>) {
        self.umcomm.attach_um(um)
    }

    pub(super) fn attach_reli(&self, reli: Rc<Reliability>) {
        self.umcomm.attach_reli(reli);
    }

    pub(super) fn owner(&self) -> Option<Rc<dyn UnitBase>> {
        if let Some(ref unit) = *self.owner.borrow() {
            unit.upgrade()
        } else {
            None
        }
    }

    pub(super) fn get_owner_id(&self) -> String {
        self.owner().map_or_else(|| "None".to_string(), |u| u.id())
    }
    pub(super) fn um(&self) -> Rc<dyn UmIf> {
        self.umcomm.um()
    }

    pub(super) fn rentry_conf_insert(&self, service: &SectionService) {
        if let Some(u) = self.owner() {
            self.rentry().conf_insert(&u.id(), service)
        }
    }

    pub(super) fn rentry_conf_get(&self) -> Option<SectionService> {
        self.owner().map(|u| self.rentry().conf_get(&u.id()))?
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
        forbid_restart: bool,
        reset_restart: bool,
        restarts: u32,
        exit_status: ExitStatus,
        monitor: ServiceMonitor,
    ) {
        if let Some(u) = self.owner() {
            self.rentry().mng_insert(
                &u.id(),
                state,
                result,
                main_pid,
                control_pid,
                main_cmd_len,
                control_cmd_type,
                control_cmd_len,
                notify_state,
                forbid_restart,
                reset_restart,
                restarts,
                exit_status,
                monitor,
            )
        }
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
        bool,
        bool,
        u32,
        ExitStatus,
        ServiceMonitor,
    )> {
        self.owner().map(|u| self.rentry().mng_get(&u.id()))?
    }

    pub(super) fn _reli(&self) -> Rc<Reliability> {
        self.umcomm._reli()
    }

    pub(super) fn log(&self, level: Level, msg: &str) {
        match level {
            Level::Error => log::error!("unit:{}, message: {}", self.id(), msg),
            Level::Warn => log::warn!("unit:{}, message: {}", self.id(), msg),
            Level::Info => log::info!("unit:{}, message: {}", self.id(), msg),
            Level::Debug => log::debug!("unit:{}, message: {}", self.id(), msg),
            Level::Trace => log::trace!("unit:{}, message: {}", self.id(), msg),
        }
    }

    fn id(&self) -> String {
        self.owner().map_or_else(|| "".to_string(), |u| u.id())
    }

    fn rentry(&self) -> Rc<ServiceRe> {
        self.umcomm.rentry()
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

    pub(super) fn attach_um(&self, um: Rc<dyn UmIf>) {
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

    pub(super) fn um(&self) -> Rc<dyn UmIf> {
        let rdata = self.data.read().unwrap();
        rdata.um().unwrap()
    }

    pub(super) fn rentry(&self) -> Rc<ServiceRe> {
        let rdata = self.data.read().unwrap();
        rdata.rentry()
    }
}

struct ServiceUmCommData {
    // associated objects
    um: Option<Rc<dyn UmIf>>,
    _reli: Weak<Reliability>,
    rentry: Option<Rc<ServiceRe>>,
}

// the declaration "pub(self)" is for identification only.
impl ServiceUmCommData {
    pub(self) fn new() -> ServiceUmCommData {
        ServiceUmCommData {
            um: None,
            _reli: Weak::new(),
            rentry: None,
        }
    }

    pub(self) fn attach_um(&mut self, um: Rc<dyn UmIf>) {
        if self.um.is_none() {
            log::debug!("ServiceUmComm attach_um action.");
            self.um = Some(um);
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

    pub(self) fn um(&self) -> Option<Rc<dyn UmIf>> {
        if let Some(ref um) = self.um {
            Some(Rc::clone(um))
        } else {
            None
        }
    }

    pub(self) fn _reli(&self) -> Rc<Reliability> {
        self._reli.clone().upgrade().unwrap()
    }

    pub(self) fn rentry(&self) -> Rc<ServiceRe> {
        self.rentry.as_ref().cloned().unwrap()
    }
}
