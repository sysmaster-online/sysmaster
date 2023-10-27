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

use super::super::entry::UnitX;
use super::ReStation;
use crate::utils::table::{Table, TableSubscribe};
use std::cell::RefCell;
use std::rc::Rc;

pub(super) struct UnitSets {
    t: RefCell<Table<String, Rc<UnitX>>>,
}

impl ReStation for UnitSets {
    // no input, no compensate

    // data: special map

    // reload
    fn entry_clear(&self) {
        // unit_entry
        for unit in self.t.borrow().get_all().iter() {
            unit.entry_clear();
        }

        // table
        self.t.borrow_mut().data_clear();
    }
}

impl UnitSets {
    pub(super) fn new() -> UnitSets {
        UnitSets {
            t: RefCell::new(Table::new()),
        }
    }

    pub(super) fn insert(&self, name: String, unit: Rc<UnitX>) -> Option<Rc<UnitX>> {
        self.t.borrow_mut().insert(name, unit)
    }

    #[allow(dead_code)]
    pub(super) fn remove(&self, name: &str) -> Option<Rc<UnitX>> {
        self.t.borrow_mut().remove(&name.to_string())
    }

    pub(super) fn get(&self, name: &str) -> Option<Rc<UnitX>> {
        self.t.borrow().get(&name.to_string())
    }

    pub(super) fn get_all(&self) -> Vec<Rc<UnitX>> {
        self.t
            .borrow()
            .get_all()
            .iter()
            .map(Rc::clone)
            .collect::<Vec<_>>()
    }

    pub(super) fn register(
        &self,
        sub_name: &str,
        subscriber: Rc<dyn TableSubscribe<String, Rc<UnitX>>>,
    ) -> Option<Rc<dyn TableSubscribe<String, Rc<UnitX>>>> {
        self.t
            .borrow_mut()
            .subscribe(sub_name.to_string(), subscriber)
    }

    pub(super) fn clear(&self) {
        self.t.borrow_mut().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manager::rentry::RELI_HISTORY_MAX_DBS;
    use crate::unit::data::DataManager;
    use crate::unit::rentry::UnitRe;
    use crate::unit::test::test_utils;
    use core::rel::{ReliConf, Reliability};

    #[test]
    fn sets_insert() {
        let dm = Rc::new(DataManager::new());
        let reli = Rc::new(Reliability::new(
            ReliConf::new().set_max_dbs(RELI_HISTORY_MAX_DBS),
        ));
        let rentry = Rc::new(UnitRe::new(&reli));
        let sets = UnitSets::new();
        let name_test1 = String::from("test1.service");
        let unit_test1 = create_unit(&dm, &reli, &rentry, &name_test1);
        let name_test2 = String::from("test2.service");
        let unit_test2 = create_unit(&dm, &reli, &rentry, &name_test2);

        let old = sets.insert(name_test1.clone(), Rc::clone(&unit_test1));
        assert!(old.is_none());

        let old = sets.insert(name_test1, Rc::clone(&unit_test2));
        assert!(Rc::ptr_eq(&old.unwrap(), &unit_test1));

        let old = sets.insert(name_test2, Rc::clone(&unit_test2));
        assert!(old.is_none());
    }

    #[test]
    fn sets_remove() {
        let dm = Rc::new(DataManager::new());
        let reli = Rc::new(Reliability::new(
            ReliConf::new().set_max_dbs(RELI_HISTORY_MAX_DBS),
        ));
        let rentry = Rc::new(UnitRe::new(&reli));
        let sets = UnitSets::new();
        let name_test1 = String::from("test1.service");
        let unit_test1 = create_unit(&dm, &reli, &rentry, &name_test1);
        let name_test2 = String::from("test2.service");
        let unit_test2 = create_unit(&dm, &reli, &rentry, &name_test2);
        let name_test3 = String::from("test3.service");

        let old = sets.remove(&name_test1);
        assert!(old.is_none());

        sets.insert(name_test1.clone(), Rc::clone(&unit_test1));
        let old = sets.remove(&name_test1);
        assert!(Rc::ptr_eq(&old.unwrap(), &unit_test1));

        sets.insert(name_test1, Rc::clone(&unit_test1));
        sets.insert(name_test2.clone(), Rc::clone(&unit_test2));
        let old = sets.remove(&name_test3);
        assert!(old.is_none());
        let old = sets.remove(&name_test2);
        assert!(Rc::ptr_eq(&old.unwrap(), &unit_test2));
    }

    #[test]
    fn sets_get() {
        let dm = Rc::new(DataManager::new());
        let reli = Rc::new(Reliability::new(
            ReliConf::new().set_max_dbs(RELI_HISTORY_MAX_DBS),
        ));
        let rentry = Rc::new(UnitRe::new(&reli));
        let sets = UnitSets::new();
        let name_test1 = String::from("test1.service");
        let unit_test1 = create_unit(&dm, &reli, &rentry, &name_test1);
        let name_test2 = String::from("test2.service");
        let unit_test2 = create_unit(&dm, &reli, &rentry, &name_test2);

        let value = sets.get(&name_test1);
        assert!(value.is_none());

        sets.insert(name_test1.clone(), Rc::clone(&unit_test1));
        let value = sets.get(&name_test1);
        assert!(Rc::ptr_eq(&value.unwrap(), &unit_test1));
        let value = sets.get(&name_test2);
        assert!(value.is_none());

        sets.insert(name_test2.clone(), Rc::clone(&unit_test2));
        let value = sets.get(&name_test2);
        assert!(Rc::ptr_eq(&value.unwrap(), &unit_test2));
    }

    #[test]
    fn sets_getall() {
        let dm = Rc::new(DataManager::new());
        let reli = Rc::new(Reliability::new(
            ReliConf::new().set_max_dbs(RELI_HISTORY_MAX_DBS),
        ));
        let rentry = Rc::new(UnitRe::new(&reli));
        let sets = UnitSets::new();
        let name_test1 = String::from("test1.service");
        let unit_test1 = create_unit(&dm, &reli, &rentry, &name_test1);
        let name_test2 = String::from("test2.service");
        let unit_test2 = create_unit(&dm, &reli, &rentry, &name_test2);

        let units = sets.get_all();
        assert_eq!(units.len(), 0);

        sets.insert(name_test1.clone(), Rc::clone(&unit_test1));
        let units = sets.get_all();
        assert_eq!(units.len(), 1);
        assert!(contain_unit(&units, &unit_test1));
        sets.remove(&name_test1);
        let units = sets.get_all();
        assert_eq!(units.len(), 0);

        sets.insert(name_test1, Rc::clone(&unit_test1));
        sets.insert(name_test2, Rc::clone(&unit_test2));
        let units = sets.get_all();
        assert_eq!(units.len(), 2);
        assert!(contain_unit(&units, &unit_test1));
        assert!(contain_unit(&units, &unit_test2));
    }

    fn create_unit(
        dmr: &Rc<DataManager>,
        relir: &Rc<Reliability>,
        rentryr: &Rc<UnitRe>,
        name: &str,
    ) -> Rc<UnitX> {
        log::init_log_to_console("create_unit", log::Level::Trace);
        log::info!("test");
        test_utils::create_unit_for_test_pub(dmr, relir, rentryr, name)
    }

    fn contain_unit(units: &[Rc<UnitX>], unit: &Rc<UnitX>) -> bool {
        for u in units.iter() {
            if Rc::ptr_eq(u, unit) {
                return true;
            }
        }

        false
    }
}
