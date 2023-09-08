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

use super::dep_conf::UnitDepConf;
use super::state::UnitState;
use crate::job::JobResult;
use crate::unit::entry::StartLimitResult;
use crate::utils::table::{Table, TableSubscribe};
use core::rel::ReStation;
use std::rc::Rc;

#[allow(clippy::type_complexity)]
pub struct DataManager {
    tables: (
        Table<String, UnitDepConf>,      // [0]unit-dep-config
        Table<String, UnitState>,        // [1]unit-state
        Table<String, StartLimitResult>, // [2]unit-start-limit-hit
        Table<String, JobResult>,        // [3] unit-job-timeout
    ),
}

impl ReStation for DataManager {
    // no input, no compensate
    // no data

    // reload
    fn entry_clear(&self) {
        self.tables.0.data_clear();
        self.tables.1.data_clear();
    }
}

impl Drop for DataManager {
    fn drop(&mut self) {
        log::debug!("DataManager drop, clear.");
        // repeating protection
        self.clear();
    }
}

impl DataManager {
    pub fn new() -> DataManager {
        DataManager {
            tables: (Table::new(), Table::new(), Table::new(), Table::new()),
        }
    }

    pub(crate) fn insert_ud_config(
        &self,
        u_name: String,
        ud_config: UnitDepConf,
    ) -> Option<UnitDepConf> {
        {
            self.tables.0.insert(u_name, ud_config)
        }
    }

    pub(crate) fn register_ud_config(
        &self,
        name: &str,
        subscriber: Rc<dyn TableSubscribe<String, UnitDepConf>>,
    ) -> Option<Rc<dyn TableSubscribe<String, UnitDepConf>>> {
        self.tables.0.subscribe(name.to_string(), subscriber)
    }

    pub(crate) fn insert_unit_state(
        &self,
        u_name: String,
        u_state: UnitState,
    ) -> Option<UnitState> {
        self.tables.1.insert(u_name, u_state)
    }

    pub(crate) fn register_unit_state(
        &self,
        name: &str,
        subscriber: Rc<dyn TableSubscribe<String, UnitState>>,
    ) -> Option<Rc<dyn TableSubscribe<String, UnitState>>> {
        self.tables.1.subscribe(name.to_string(), subscriber)
    }

    pub(crate) fn insert_start_limit_result(
        &self,
        u_name: String,
        start_limit_res: StartLimitResult,
    ) -> Option<StartLimitResult> {
        self.tables.2.insert(u_name, start_limit_res)
    }

    pub(crate) fn register_start_limit_result(
        &self,
        name: &str,
        subscriber: Rc<dyn TableSubscribe<String, StartLimitResult>>,
    ) -> Option<Rc<dyn TableSubscribe<String, StartLimitResult>>> {
        self.tables.2.subscribe(name.to_string(), subscriber)
    }

    pub(crate) fn insert_job_result(
        &self,
        u_name: String,
        job_result: JobResult,
    ) -> Option<JobResult> {
        self.tables.3.insert(u_name, job_result)
    }

    pub(crate) fn register_job_result(
        &self,
        name: &str,
        subscriber: Rc<dyn TableSubscribe<String, JobResult>>,
    ) -> Option<Rc<dyn TableSubscribe<String, JobResult>>> {
        self.tables.3.subscribe(name.to_string(), subscriber)
    }

    // repeating protection
    pub(crate) fn clear(&self) {
        self.tables.0.clear();
        self.tables.1.clear();
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::unit::UnitRelations;
    use crate::utils::table::TableOp;
    use core::unit::{UnitActiveState, UnitNotifyFlags};

    #[test]
    fn dm_unit_dep_config() {
        let dm = DataManager::new();
        let udc_sub = Rc::new(UnitDepConfigsTest::new());

        let ud_config = UnitDepConf::new();
        let old = dm.insert_ud_config(String::from("test"), ud_config);
        assert!(old.is_none());

        let mut ud_config = UnitDepConf::new();
        let vec = vec!["name".to_string()];
        ud_config.deps.insert(UnitRelations::UnitAfter, vec);
        let old = dm.insert_ud_config(String::from("test"), ud_config);
        assert_eq!(old.unwrap().deps.len(), 0);

        let sub = Rc::clone(&udc_sub);
        let old = dm.register_ud_config(&String::from("config"), sub);
        assert!(old.is_none());

        let mut ud_config = UnitDepConf::new();
        let vec = vec!["name".to_string()];
        ud_config.deps.insert(UnitRelations::UnitAfter, vec);
        dm.insert_ud_config(String::from("test"), ud_config);
        assert_eq!(udc_sub.len(), 1);
    }
    #[test]
    fn dm_unit_state() {
        let dm = DataManager::new();
        let os = UnitActiveState::InActive;
        let ns = UnitActiveState::Active;
        let flags = UnitNotifyFlags::RELOAD_FAILURE;
        let us_sub = Rc::new(UnitStatesTest::new(ns));

        let old = dm.insert_unit_state(String::from("test"), UnitState::new(os, ns, flags));
        assert!(old.is_none());

        let ns_ing = UnitActiveState::Activating;
        let old = dm.insert_unit_state(String::from("test"), UnitState::new(os, ns_ing, flags));
        assert_eq!(old.unwrap().ns, ns);

        let sub = Rc::clone(&us_sub);
        let old = dm.register_unit_state(&String::from("state"), sub);
        assert!(old.is_none());

        let ns_m = UnitActiveState::Maintenance;
        dm.insert_unit_state(String::from("test"), UnitState::new(os, ns_m, flags));
        assert_eq!(us_sub.get_ns(), ns_m);
    }

    struct UnitDepConfigsTest {
        len: RefCell<usize>,
    }

    impl UnitDepConfigsTest {
        fn new() -> UnitDepConfigsTest {
            UnitDepConfigsTest {
                len: RefCell::new(0),
            }
        }

        fn len(&self) -> usize {
            *self.len.borrow()
        }
    }

    impl TableSubscribe<String, UnitDepConf> for UnitDepConfigsTest {
        fn notify(&self, op: &TableOp<String, UnitDepConf>) {
            match op {
                TableOp::TableInsert(_, ud_conf) => {
                    *self.len.borrow_mut() = ud_conf.deps.len();
                }
                TableOp::TableRemove(_, _) => *self.len.borrow_mut() = 0,
            }
        }
    }

    struct UnitStatesTest {
        ns: RefCell<UnitActiveState>,
    }

    impl UnitStatesTest {
        fn new(ns: UnitActiveState) -> UnitStatesTest {
            UnitStatesTest {
                ns: RefCell::new(ns),
            }
        }

        fn get_ns(&self) -> UnitActiveState {
            *self.ns.borrow()
        }
    }

    impl TableSubscribe<String, UnitState> for UnitStatesTest {
        fn notify(&self, op: &TableOp<String, UnitState>) {
            match op {
                TableOp::TableInsert(_, state) => *self.ns.borrow_mut() = state.ns,
                TableOp::TableRemove(_, _) => {}
            }
        }
    }
}
