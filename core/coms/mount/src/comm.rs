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

//!  Mount association unit object
//! *  You need to notify the Unit object and change the method
//! *  Get the attributes of the unit object
//! *  Call relation: mount_ unit->mount_ mng->mount_ comm

use crate::rentry::SectionMount;

use super::rentry::{MountRe, MountState};
use core::rel::Reliability;
use core::unit::{UmIf, UnitBase};
use once_cell::sync::Lazy;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::{Arc, RwLock};

pub(super) struct MountUnitComm {
    owner: RefCell<Option<Weak<dyn UnitBase>>>,
    umcomm: Arc<MountUmComm>,
}

impl MountUnitComm {
    pub(super) fn new() -> Self {
        MountUnitComm {
            owner: RefCell::new(None),
            umcomm: MountUmComm::get_instance(),
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

    pub(super) fn get_owner_id(&self) -> String {
        self.owner().map_or_else(|| "None".to_string(), |u| u.id())
    }

    pub(super) fn rentry_conf_insert(&self, mount: &SectionMount) {
        if let Some(u) = self.owner() {
            self.rentry().conf_insert(&u.id(), mount)
        }
    }

    pub(super) fn rentry_conf_get(&self) -> Option<SectionMount> {
        self.owner().map(|u| self.rentry().conf_get(&u.id()))?
    }

    pub(super) fn rentry_mng_insert(&self, state: MountState) {
        self.rentry().mng_insert(&self.get_owner_id(), state)
    }

    pub(super) fn rentry_mng_get(&self) -> Option<MountState> {
        self.rentry().mng_get(&self.get_owner_id())
    }

    fn rentry(&self) -> Rc<MountRe> {
        self.umcomm.rentry()
    }

    pub(super) fn um(&self) -> Rc<dyn UmIf> {
        self.umcomm.um()
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

    pub(super) fn attach_um(&self, um: Rc<dyn UmIf>) {
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

    pub(super) fn um(&self) -> Rc<dyn UmIf> {
        let rdata = self.data.read().unwrap();
        rdata.um().unwrap()
    }

    pub(super) fn rentry(&self) -> Rc<MountRe> {
        let rdata = self.data.read().unwrap();
        rdata.rentry()
    }
}

struct MountUmCommData {
    // associated objects
    um: Option<Rc<dyn UmIf>>,
    reli: Weak<Reliability>,
    rentry: Option<Rc<MountRe>>,
}

// the declaration "pub(self)" is for identification only.
impl MountUmCommData {
    pub(self) fn new() -> MountUmCommData {
        MountUmCommData {
            um: None,
            reli: Weak::new(),
            rentry: None,
        }
    }

    pub(self) fn attach_um(&mut self, um: Rc<dyn UmIf>) {
        if self.um.is_none() {
            log::debug!("MountUmComm attach_um action.");
            self.um = Some(um);
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

    pub(self) fn rentry(&self) -> Rc<MountRe> {
        self.rentry.as_ref().cloned().unwrap()
    }
}
