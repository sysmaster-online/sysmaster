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

use crate::unit::util::{self};

use core::rel::Reliability;
use core::unit::{UmIf, UnitManagerObj, UnitType};
use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::rc::{Rc, Weak};

#[allow(dead_code)]
pub(super) struct UnitSubManagers {
    // associated objects
    reli: Rc<Reliability>,
    target: String,
    file_size: u32,
    file_number: u32,

    // owned objects
    um: RefCell<Option<Weak<dyn UmIf>>>,
    db: RefCell<HashMap<UnitType, Box<dyn UnitManagerObj>>>,
}

impl UnitSubManagers {
    pub(super) fn new(
        relir: &Rc<Reliability>,
        targetr: &str,
        file_size: u32,
        file_number: u32,
    ) -> UnitSubManagers {
        UnitSubManagers {
            reli: Rc::clone(relir),
            target: String::from(targetr),
            file_size,
            file_number,
            um: RefCell::new(None),
            db: RefCell::new(HashMap::new()),
        }
    }

    pub(super) fn set_um(&self, um: Rc<dyn UmIf>) {
        // update um
        self.um.replace(Some(Rc::downgrade(&um)));

        // fill all unit-types
        for ut in 0..UnitType::UnitTypeMax as u32 {
            self.add_sub(UnitType::try_from(ut).ok().unwrap());
        }
    }

    pub(super) fn enumerate(&self) {
        for (_, sub) in self.db.borrow().iter() {
            sub.enumerate();
        }
    }

    pub(super) fn private_section(&self, unit_type: UnitType) -> String {
        if let Some(sub) = self.db.borrow().get(&unit_type) {
            sub.private_section(unit_type)
        } else {
            String::from("")
        }
    }

    pub(super) fn can_transient(&self, unit_type: UnitType) -> bool {
        if let Some(sub) = self.db.borrow().get(&unit_type) {
            sub.can_transient(unit_type)
        } else {
            false
        }
    }

    pub(super) fn input_rebuild(&self) {
        for (_, sub) in self.db.borrow().iter() {
            sub.input_rebuild();
        }
    }

    pub(super) fn db_map(&self, reload: bool) {
        for (_, sub) in self.db.borrow().iter() {
            sub.db_map(reload);
        }
    }

    pub(super) fn db_insert(&self) {
        for (_, sub) in self.db.borrow().iter() {
            sub.db_insert();
        }
    }

    pub(super) fn db_compensate_last(
        &self,
        lframe: (u32, Option<u32>, Option<u32>),
        lunit: Option<&String>,
    ) {
        let utype = self.last_unittype(lframe);
        if utype.is_none() || lunit.is_none() {
            return;
        }

        let unit_type = utype.unwrap();
        if let Some(sub) = self.db.borrow().get(&unit_type) {
            sub.db_compensate_last(lframe, lunit);
        }
    }

    pub(super) fn do_compensate_last(
        &self,
        lframe: (u32, Option<u32>, Option<u32>),
        lunit: Option<&String>,
    ) {
        let utype = self.last_unittype(lframe);
        if utype.is_none() || lunit.is_none() {
            return;
        }

        let unit_type = utype.unwrap();
        if let Some(sub) = self.db.borrow().get(&unit_type) {
            sub.do_compensate_last(lframe, lunit);
        }
    }

    fn add_sub(&self, unit_type: UnitType) {
        assert!(!self.db.borrow().contains_key(&unit_type));

        let sub = self.new_sub(unit_type);
        if let Some(s) = sub {
            self.db.borrow_mut().insert(unit_type, s);
        }
    }

    fn new_sub(&self, unit_type: UnitType) -> Option<Box<dyn UnitManagerObj>> {
        let um = self.um();
        log::info!(
            "Creating UnitManagerObj for {:?} by __um_obj_create()",
            unit_type
        );
        let sub =
            match util::create_um_obj(unit_type, &self.target, self.file_size, self.file_number) {
                Err(_) => {
                    log::info!("__um_obj_create() of {:?} is not found", unit_type);
                    return None;
                }
                Ok(v) => v,
            };

        sub.attach_um(um);
        sub.attach_reli(Rc::clone(&self.reli));
        Some(sub)
    }

    fn last_unittype(&self, lframe: (u32, Option<u32>, Option<u32>)) -> Option<UnitType> {
        let (_, utype, _) = lframe;
        utype?;

        let ut = utype.unwrap();
        if ut > UnitType::UnitTypeMax as u32 {
            // error
            return None;
        }

        Some(UnitType::try_from(ut).ok().unwrap())
    }

    fn um(&self) -> Rc<dyn UmIf> {
        self.um.clone().into_inner().unwrap().upgrade().unwrap()
    }
}
