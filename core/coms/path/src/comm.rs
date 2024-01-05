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

//!  The comm module provides management of common objects, mainly including weak references to UnitManager and Unit objects.
//!  The method provided by the public object needs to be called.
//!
use super::rentry::{PathRe, PathResult, PathState, SectionPath};
use core::rel::Reliability;
use core::unit::{PathType, UmIf, UnitBase};
use once_cell::sync::Lazy;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::{Arc, RwLock};

pub(super) struct PathUnitComm {
    owner: RefCell<Option<Weak<dyn UnitBase>>>,
    umcomm: Arc<PathUmComm>,
}

impl PathUnitComm {
    pub(super) fn new() -> Self {
        PathUnitComm {
            owner: RefCell::new(None),
            umcomm: PathUmComm::get_instance(),
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

    pub(super) fn rentry_conf_insert(&self, path: &SectionPath) {
        if let Some(u) = self.owner() {
            self.rentry().conf_insert(&u.id(), path)
        }
    }

    pub(super) fn rentry_conf_get(&self) -> Option<SectionPath> {
        self.owner().map(|u| self.rentry().conf_get(&u.id()))?
    }

    pub(super) fn rentry_mng_insert(
        &self,
        state: PathState,
        result: PathResult,
        path_spec: Vec<(PathType, bool, String)>,
    ) {
        if let Some(u) = self.owner() {
            self.rentry().mng_insert(&u.id(), state, result, path_spec)
        }
    }

    pub(super) fn rentry_mng_get(&self) -> Option<(PathState, PathResult)> {
        self.owner().map(|u| self.rentry().mng_get(&u.id()))?
    }

    pub(super) fn _reli(&self) -> Rc<Reliability> {
        self.umcomm._reli()
    }

    fn rentry(&self) -> Rc<PathRe> {
        self.umcomm.rentry()
    }
}

static PATH_UM_COMM: Lazy<Arc<PathUmComm>> = Lazy::new(|| {
    let comm = PathUmComm::new();
    Arc::new(comm)
});

pub(super) struct PathUmComm {
    data: RwLock<PathUmCommData>,
}

unsafe impl Send for PathUmComm {}

unsafe impl Sync for PathUmComm {}

impl PathUmComm {
    pub(super) fn new() -> Self {
        PathUmComm {
            data: RwLock::new(PathUmCommData::new()),
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

    pub(super) fn get_instance() -> Arc<PathUmComm> {
        PATH_UM_COMM.clone()
    }

    pub(super) fn _reli(&self) -> Rc<Reliability> {
        let rdata = self.data.read().unwrap();
        rdata._reli()
    }

    pub(super) fn um(&self) -> Rc<dyn UmIf> {
        let rdata = self.data.read().unwrap();
        rdata.um().unwrap()
    }

    pub(super) fn rentry(&self) -> Rc<PathRe> {
        let rdata = self.data.read().unwrap();
        rdata.rentry()
    }
}

struct PathUmCommData {
    // associated objects
    um: Option<Rc<dyn UmIf>>,
    reli: Weak<Reliability>,
    rentry: Option<Rc<PathRe>>,
}

// the declaration "pub(self)" is for identification only.
impl PathUmCommData {
    pub(self) fn new() -> PathUmCommData {
        PathUmCommData {
            um: None,
            reli: Weak::new(),
            rentry: None,
        }
    }

    pub(self) fn attach_um(&mut self, um: Rc<dyn UmIf>) {
        if self.um.is_none() {
            log::debug!("PathUmCommData attach_um action.");
            self.um = Some(um);
        }
    }

    pub(self) fn attach_reli(&mut self, reli: Rc<Reliability>) {
        let old = self.reli.clone().upgrade();
        if old.is_none() {
            log::debug!("PathUmCommData attach_reli action.");
            self.reli = Rc::downgrade(&reli);
            self.rentry.replace(Rc::new(PathRe::new(&reli)));
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

    pub(self) fn rentry(&self) -> Rc<PathRe> {
        self.rentry.as_ref().cloned().unwrap()
    }
}
