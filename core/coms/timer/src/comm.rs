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

//!  The method provided by the public object needs to be called.
//!
use core::rel::Reliability;
use core::unit::{UmIf, UnitBase};
use once_cell::sync::Lazy;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::{Arc, RwLock};

use crate::rentry::{SectionTimer, TimerRe, TimerResult, TimerState};

pub(crate) struct TimerUnitComm {
    owner: RefCell<Option<Weak<dyn UnitBase>>>,
    umcomm: Arc<TimerUmComm>,
}

impl TimerUnitComm {
    pub(super) fn new() -> Self {
        TimerUnitComm {
            owner: RefCell::new(None),
            umcomm: TimerUmComm::get_instance(),
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

    pub(super) fn _reli(&self) -> Rc<Reliability> {
        self.umcomm._reli()
    }

    pub(super) fn rentry(&self) -> Rc<TimerRe> {
        self.umcomm.rentry()
    }

    pub(super) fn rentry_conf_insert(&self, timer: &SectionTimer, service: String) {
        if let Some(u) = self.owner() {
            self.rentry().conf_insert(&u.id(), timer, service)
        }
    }

    pub(super) fn rentry_conf_get(&self) -> Option<(SectionTimer, String)> {
        self.owner().map(|u| self.rentry().conf_get(&u.id()))?
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn rentry_mng_insert(
        &self,
        state: TimerState,
        result: TimerResult,
        last_trigger_realtime: u64,
        last_trigger_monotonic: u64,
    ) {
        if let Some(u) = self.owner() {
            self.rentry().mng_insert(
                &u.id(),
                state,
                result,
                last_trigger_realtime,
                last_trigger_monotonic,
            )
        };
    }

    #[allow(clippy::type_complexity)]
    pub(super) fn rentry_mng_get(&self) -> Option<(TimerState, TimerResult, u64, u64)> {
        self.owner().map(|u| self.rentry().mng_get(&u.id()))?
    }
}

static TIMER_UM_COMM: Lazy<Arc<TimerUmComm>> = Lazy::new(|| {
    let comm = TimerUmComm::new();
    Arc::new(comm)
});

pub(super) struct TimerUmComm {
    data: RwLock<TimerUmCommData>,
}

unsafe impl Send for TimerUmComm {}

unsafe impl Sync for TimerUmComm {}

impl TimerUmComm {
    pub(super) fn new() -> Self {
        TimerUmComm {
            data: RwLock::new(TimerUmCommData::new()),
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

    pub(super) fn get_instance() -> Arc<TimerUmComm> {
        TIMER_UM_COMM.clone()
    }

    pub(super) fn _reli(&self) -> Rc<Reliability> {
        let rdata = self.data.read().unwrap();
        rdata._reli()
    }

    pub(super) fn um(&self) -> Rc<dyn UmIf> {
        let rdata = self.data.read().unwrap();
        rdata.um().unwrap()
    }

    pub(super) fn rentry(&self) -> Rc<TimerRe> {
        let rdata = self.data.read().unwrap();
        rdata.rentry()
    }
}

struct TimerUmCommData {
    // associated objects
    um: Option<Rc<dyn UmIf>>,
    reli: Weak<Reliability>,
    rentry: Option<Rc<TimerRe>>,
}

// the declaration "pub(self)" is for identification only.
impl TimerUmCommData {
    pub(self) fn new() -> TimerUmCommData {
        TimerUmCommData {
            um: None,
            reli: Weak::new(),
            rentry: None,
        }
    }

    pub(self) fn attach_um(&mut self, um: Rc<dyn UmIf>) {
        if self.um.is_none() {
            log::debug!("TimerUmComm attach_um action.");
            self.um = Some(um)
        }
    }

    pub(self) fn attach_reli(&mut self, reli: Rc<Reliability>) {
        let old = self.reli.clone().upgrade();
        if old.is_none() {
            log::debug!("TimerUmComm attach_reli action.");
            self.reli = Rc::downgrade(&reli);
            self.rentry.replace(Rc::new(TimerRe::new(&reli)));
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
        self.reli.clone().upgrade().unwrap()
    }

    pub(self) fn rentry(&self) -> Rc<TimerRe> {
        self.rentry.as_ref().cloned().unwrap()
    }
}
