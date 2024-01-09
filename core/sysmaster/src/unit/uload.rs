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

use super::data::{DataManager, UnitDepConf};
use super::datastore::UnitDb;
use super::entry::UnitX;
use super::manager::UnitManager;
use super::rentry::{self, UnitLoadState, UnitRe};
use super::runtime::UnitRT;
use super::util::UnitFile;
use super::util::{self};
use crate::utils::table::{TableOp, TableSubscribe};
use basic::fs::LookupPaths;
use core::unit::{unit_name_is_valid, UnitNameFlags, UnitType};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

//#[derive(Debug)]
pub(super) struct UnitLoad {
    sub_name: String, // key for table-subscriber: UnitDepConf
    data: Rc<UnitLoadData>,
}

impl UnitLoad {
    pub(super) fn new(
        dmr: &Rc<DataManager>,
        rentryr: &Rc<UnitRe>,
        dbr: &Rc<UnitDb>,
        rtr: &Rc<UnitRT>,
        lookup_path: &Rc<LookupPaths>,
    ) -> UnitLoad {
        let load = UnitLoad {
            sub_name: String::from("UnitLoad"),
            data: Rc::new(UnitLoadData::new(dmr, rentryr, dbr, rtr, lookup_path)),
        };
        load.register(dmr);
        load
    }

    pub(super) fn load_unit(&self, name: &str) -> Option<Rc<UnitX>> {
        self.data.load_unit(name)
    }

    pub(super) fn load_update(&self, unit: &Rc<UnitX>) {
        self.data.load_update(unit);
    }

    pub(super) fn set_um(&self, um: &Rc<UnitManager>) {
        self.data.set_um(um);
    }

    pub(super) fn try_new_unit(&self, name: &str) -> Option<Rc<UnitX>> {
        self.data.try_new_unit(name)
    }

    fn register(&self, dm: &DataManager) {
        let subscriber = Rc::clone(&self.data);
        let ret = dm.register_ud_config(&self.sub_name, subscriber);
        assert!(ret.is_none())
    }
}

//#[derive(Debug)]
struct UnitLoadData {
    // associated objects
    dm: Rc<DataManager>,
    rentry: Rc<UnitRe>,
    um: RefCell<Weak<UnitManager>>,
    db: Rc<UnitDb>,
    rt: Rc<UnitRT>,

    // owned objects
    file: Rc<UnitFile>,
}

// the declaration "pub(self)" is for identification only.
impl UnitLoadData {
    pub(self) fn new(
        dmr: &Rc<DataManager>,
        rentryr: &Rc<UnitRe>,
        dbr: &Rc<UnitDb>,
        rtr: &Rc<UnitRT>,
        lookup_path: &Rc<LookupPaths>,
    ) -> UnitLoadData {
        log::debug!("UnitLoadData db count is {}", Rc::strong_count(dbr));
        let file = Rc::new(UnitFile::new(lookup_path));
        UnitLoadData {
            dm: Rc::clone(dmr),
            rentry: Rc::clone(rentryr),
            um: RefCell::new(Weak::new()),
            db: Rc::clone(dbr),
            rt: Rc::clone(rtr),
            file: Rc::clone(&file),
        }
    }

    pub(self) fn prepare_unit(&self, name: &str) -> Option<Rc<UnitX>> {
        if !unit_name_is_valid(name, UnitNameFlags::PLAIN | UnitNameFlags::INSTANCE) {
            return None;
        }

        if let Some(u) = self.db.units_get(name) {
            if u.load_state() != UnitLoadState::NotFound {
                return Some(Rc::clone(&u));
            } else {
                self.rt.push_load_queue(Rc::clone(&u));
                return Some(Rc::clone(&u));
            }
        }

        match self.try_new_unit(name) {
            Some(unit) => {
                self.db.units_insert(name.to_string(), Rc::clone(&unit));
                self.rt.push_load_queue(Rc::clone(&unit));
                Some(Rc::clone(&unit))
            }
            None => {
                log::error!(
                    "create unit obj failed,name is {},{}",
                    name,
                    Rc::strong_count(&self.db)
                );
                None
            }
        }
    }

    pub(self) fn push_dep_unit_into_load_queue(&self, name: &str) -> Option<Rc<UnitX>> {
        if let Some(unit) = self.db.units_get(name) {
            return Some(Rc::clone(&unit));
        };

        self.prepare_unit(name)
    }

    pub(self) fn load_unit(&self, name: &str) -> Option<Rc<UnitX>> {
        let mut u = match self.prepare_unit(name) {
            None => return None,
            Some(v) => v,
        };
        self.rt.dispatch_load_queue();
        while u.load_state() == UnitLoadState::Merged {
            match u.merged_into() {
                None => return None,
                Some(v) => u = v,
            }
        }
        Some(u)
    }

    pub(self) fn load_update(&self, unit: &Rc<UnitX>) {
        self.rt.push_load_queue(Rc::clone(unit));
        self.rt.dispatch_load_queue();
    }

    pub(self) fn set_um(&self, um: &Rc<UnitManager>) {
        self.um.replace(Rc::downgrade(um));
    }

    pub(self) fn try_new_unit(&self, name: &str) -> Option<Rc<UnitX>> {
        let unit_type = rentry::unit_name_to_type(name);
        if unit_type == UnitType::UnitTypeInvalid {
            return None;
        }

        let um = self.um();
        let um_rc = Rc::clone(&um);
        log::debug!(
            "Creating SubUnit for {:?}: {} by __subunit_create_with_params()",
            unit_type,
            name
        );
        let subclass = match util::create_subunit_with_um(unit_type, um_rc) {
            Ok(sub) => sub,
            Err(_e) => {
                log::error!("Failed to create unit_obj!{}", _e);
                return None;
            }
        };

        let reli = um.reliability();
        subclass.attach_um(um);
        subclass.attach_reli(reli);

        Some(Rc::new(UnitX::new(
            &self.dm,
            &self.rentry,
            &self.file,
            unit_type,
            name,
            subclass,
        )))
    }

    fn um(&self) -> Rc<UnitManager> {
        self.um.clone().into_inner().upgrade().unwrap()
    }
}

impl TableSubscribe<String, UnitDepConf> for UnitLoadData {
    fn notify(&self, op: &TableOp<String, UnitDepConf>) {
        match op {
            TableOp::TableInsert(name, config) => self.insert_udconf(name, config),
            TableOp::TableRemove(_, _) => {} // self.remove_udconf(name)
        }
    }
}

impl UnitLoadData {
    fn insert_udconf(&self, name: &str, config: &UnitDepConf) {
        //hash map insert return is old value,need reconstruct
        let unit = match self.db.units_get(name) {
            Some(u) => u,
            None => {
                log::error!("create unit obj error in unit manager");
                return;
            } // load
        };

        // dependency
        for (relation, list) in config.deps.iter() {
            for o_name in list {
                let tmp_unit: Rc<UnitX> =
                    if let Some(o_unit) = self.push_dep_unit_into_load_queue(o_name) {
                        //can not call unit_load directly, will be nested.
                        Rc::clone(&o_unit)
                    } else {
                        log::error!("create unit obj error in unit manager");
                        return;
                    };

                if let Err(_e) = self
                    .db
                    .dep_insert(Rc::clone(&unit), *relation, tmp_unit, true, 0)
                //insert the dependency, but not judge loaded success, if loaded failed, whether record the dependency.
                {
                    log::debug!("add dependency relation failed for source unit is {},dependency unit is {}",unit.id(),o_name);
                    return;
                }
            }
        }
    }

    #[allow(dead_code)]
    fn remove_udconf(&self, _source: &str) {
        todo!();
    }
}
