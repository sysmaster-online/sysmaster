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

use super::datastore::UnitDb;
use super::entry::UnitX;
use super::rentry::UnitLoadState;
use super::rentry::{UnitRe, UnitRePps};
use super::{UnitRelations, UnitType};
use crate::manager::rentry::ReliLastQue;
use crate::utils::table::{TableOp, TableSubscribe};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::convert::TryFrom;
use std::rc::Rc;
use sysmaster::rel::{ReStation, ReliLastFrame, Reliability};
use sysmaster::unit::{UnitDependencyMask, UnitRelationAtom};

//#[derive(Debug)]
pub(super) struct UnitRT {
    sub_name: String, // key for table-subscriber: UnitSets
    data: Rc<UnitRTData>,
}

impl ReStation for UnitRT {
    // input: do nothing

    // compensate
    fn db_compensate_last(&self, lframe: (u32, Option<u32>, Option<u32>), lunit: Option<&String>) {
        self.data.db_compensate_last(lframe, lunit);
    }

    fn do_compensate_last(&self, lframe: (u32, Option<u32>, Option<u32>), lunit: Option<&String>) {
        self.data.do_compensate_last(lframe, lunit);
    }

    // data: special insert
    fn db_map(&self, reload: bool) {
        self.data.db_map(reload);
    }

    // reload
    // repeating protection
    fn entry_clear(&self) {
        self.data.entry_clear();
    }
}

impl Drop for UnitRT {
    fn drop(&mut self) {
        log::debug!("UnitRT drop, clear.");
        // repeating protection
        self.entry_clear();
        self.data.db.clear();
        self.data.reli.clear();
    }
}

impl UnitRT {
    pub(super) fn new(relir: &Rc<Reliability>, rentryr: &Rc<UnitRe>, dbr: &Rc<UnitDb>) -> UnitRT {
        let rt = UnitRT {
            sub_name: String::from("UnitRT"),
            data: Rc::new(UnitRTData::new(relir, rentryr, dbr)),
        };
        rt.register(dbr);
        rt
    }

    pub(super) fn dispatch_load_queue(&self) {
        self.data.dispatch_load_queue();
    }

    pub(super) fn unit_add_dependency(
        &self,
        source: Rc<UnitX>,
        relation: UnitRelations,
        target: Rc<UnitX>,
        add_ref: bool,
        mask: UnitDependencyMask,
    ) {
        self.data
            .unit_add_dependency(source, relation, target, add_ref, mask)
    }

    pub(super) fn push_load_queue(&self, unit: Rc<UnitX>) {
        self.data.push_load_queue(unit);
    }

    fn register(&self, dbr: &Rc<UnitDb>) {
        let subscriber = Rc::clone(&self.data);
        dbr.units_register(&self.sub_name, subscriber);
    }
}

//#[derive(Debug)]
struct UnitRTData {
    // associated objects
    reli: Rc<Reliability>,
    rentry: Rc<UnitRe>,
    db: Rc<UnitDb>,

    // owned objects
    load_queue: RefCell<VecDeque<Rc<UnitX>>>,
    target_dep_queue: RefCell<VecDeque<Rc<UnitX>>>,
}

impl TableSubscribe<String, Rc<UnitX>> for UnitRTData {
    fn notify(&self, op: &TableOp<String, Rc<UnitX>>) {
        match op {
            TableOp::TableInsert(_, _) => {} // do nothing
            TableOp::TableRemove(_, unit) => self.remove_unit(unit),
        }
    }
}

impl UnitRTData {
    fn db_compensate_last(&self, lframe: (u32, Option<u32>, Option<u32>), lunit: Option<&String>) {
        if lunit.is_none() {
            return;
        }

        let (_, queue, _) = lframe;
        if queue.is_none() {
            return;
        }

        if let Ok(que) = ReliLastQue::try_from(queue.unwrap()) {
            let unit_id = lunit.unwrap();
            match que {
                ReliLastQue::Load => self.rc_last_queue_load(unit_id),
                ReliLastQue::TargetDeps => self.rc_last_queue_targetdeps(unit_id),
                _ => todo!(),
            }
        }
    }

    fn rc_last_queue_load(&self, lunit: &String) {
        // remove from pps, which would be compensated later(dc_last_queue_load).
        if self.rentry.pps_contains(lunit, UnitRePps::QUEUE_LOAD) {
            self.rentry.pps_clear(lunit, UnitRePps::QUEUE_LOAD);
        }
    }

    fn rc_last_queue_targetdeps(&self, lunit: &String) {
        // remove from pps, which would be compensated later(dc_last_queue_targetdeps).
        if self
            .rentry
            .pps_contains(lunit, UnitRePps::QUEUE_TARGET_DEPS)
        {
            self.rentry.pps_clear(lunit, UnitRePps::QUEUE_TARGET_DEPS);
        }
    }

    fn do_compensate_last(&self, lframe: (u32, Option<u32>, Option<u32>), lunit: Option<&String>) {
        if lunit.is_none() {
            return;
        }

        let (_, queue, _) = lframe;
        if queue.is_none() {
            return;
        }

        if let Ok(que) = ReliLastQue::try_from(queue.unwrap()) {
            let unit_id = lunit.unwrap();
            match que {
                ReliLastQue::Load => self.dc_last_queue_load(unit_id),
                ReliLastQue::TargetDeps => self.dc_last_queue_targetdeps(unit_id),
                _ => todo!(),
            }
        }
    }

    fn dc_last_queue_load(&self, lunit: &str) {
        // retry
        if let Some(unit) = self.db.units_get(lunit) {
            if let Err(e) = unit.load() {
                log::error!(
                    "dispatch dc last queue, load unit [{}] failed: {}",
                    unit.id(),
                    e.to_string()
                );
            }
        }
    }

    fn dc_last_queue_targetdeps(&self, lunit: &str) {
        // retry
        if let Some(unit) = self.db.units_get(lunit) {
            dispatch_target_dep_unit(&self.db, &unit);
        }
    }
}

// the declaration "pub(self)" is for identification only.
impl UnitRTData {
    pub(self) fn new(
        relir: &Rc<Reliability>,
        rentryr: &Rc<UnitRe>,
        dbr: &Rc<UnitDb>,
    ) -> UnitRTData {
        UnitRTData {
            reli: Rc::clone(relir),
            rentry: Rc::clone(rentryr),
            db: Rc::clone(dbr),
            load_queue: RefCell::new(VecDeque::new()),
            target_dep_queue: RefCell::new(VecDeque::new()),
        }
    }

    pub(self) fn entry_clear(&self) {
        self.load_queue.borrow_mut().clear();
        self.target_dep_queue.borrow_mut().clear();
    }

    pub(self) fn db_map(&self, reload: bool) {
        for unit_id in self.rentry.pps_keys().iter() {
            if !reload {
                let load_mask = UnitRePps::QUEUE_LOAD;
                if self.rentry.pps_contains(unit_id, load_mask) {
                    let unit = self.db.units_get(unit_id).unwrap();
                    self.push_load_queue(unit);
                }

                let tardeps_mask = UnitRePps::QUEUE_TARGET_DEPS;
                if self.rentry.pps_contains(unit_id, tardeps_mask) {
                    let unit = self.db.units_get(unit_id).unwrap();
                    self.push_target_dep_queue(unit);
                }
            }
        }
    }

    pub(self) fn dispatch_load_queue(&self) {
        log::debug!("dispatch load queue");

        self.reli
            .set_last_frame2(ReliLastFrame::Queue as u32, ReliLastQue::Load as u32);
        loop {
            //Limit the scope of borrow of load queue
            //unitX pop from the load queue and then no need the ref of load queue
            //the unitX load process will borrow load queue as mut again
            // pop
            let first_unit = self.load_queue.borrow_mut().pop_front();
            match first_unit {
                None => break,
                Some(unit) => {
                    // record + action
                    self.reli.set_last_unit(unit.id());
                    match unit.load() {
                        Ok(()) => {
                            let load_state = unit.load_state();
                            if load_state == UnitLoadState::Loaded {
                                self.push_target_dep_queue(Rc::clone(&unit));
                            }
                        }
                        Err(e) => {
                            log::error!("load unit [{}] failed: {}", unit.id(), e.to_string());
                        }
                    }
                    self.reli.clear_last_unit();
                }
            }
        }
        self.reli.clear_last_frame();

        log::debug!("dispatch target dep queue");
        self.reli
            .set_last_frame2(ReliLastFrame::Queue as u32, ReliLastQue::TargetDeps as u32);
        self.dispatch_target_dep_queue();
        self.reli.clear_last_frame();
    }

    pub(self) fn unit_add_dependency(
        &self,
        source: Rc<UnitX>,
        relation: UnitRelations,
        target: Rc<UnitX>,
        add_ref: bool,
        _mask: UnitDependencyMask,
    ) {
        if let Err(e) = self
            .db
            .dep_insert(source, relation, target, add_ref, 1 << 2)
        {
            log::error!("dispatch_target_dep_queue add default dep err {:?}", e);
        }
    }

    fn dispatch_target_dep_queue(&self) {
        loop {
            let first_unit = self.target_dep_queue.borrow_mut().pop_front();
            match first_unit {
                None => break,
                Some(unit) => {
                    self.reli.set_last_unit(unit.id());
                    dispatch_target_dep_unit(&self.db, &unit);
                    self.reli.clear_last_unit();
                }
            }
        }
    }

    fn push_target_dep_queue(&self, unit: Rc<UnitX>) {
        if unit.in_target_dep_queue() {
            return;
        }
        log::debug!("push unit [{}] into target dep queue", unit.id());
        unit.set_in_target_dep_queue(true);
        self.target_dep_queue.borrow_mut().push_back(unit);
    }

    pub(self) fn push_load_queue(&self, unit: Rc<UnitX>) {
        if unit.in_load_queue() {
            return;
        }
        unit.set_in_load_queue(true);
        self.load_queue.borrow_mut().push_back(unit);
    }

    fn remove_unit(&self, _unit: &Rc<UnitX>) {}
}

fn dispatch_target_dep_unit(db: &Rc<UnitDb>, unit: &Rc<UnitX>) {
    unit.set_in_target_dep_queue(false);
    let atom = UnitRelationAtom::UnitAtomDefaultTargetDependencies;
    let b_atom = UnitRelationAtom::UnitAtomBefore;
    let after = UnitRelations::UnitAfter;
    let mask = UnitDependencyMask::Default;
    for dep_target in db.dep_gets_atom(unit, atom) {
        if dep_target.unit_type() != UnitType::UnitTarget {
            log::debug!("dep unit type is not target, continue");
            return;
        }
        if unit.load_state() != UnitLoadState::Loaded
            || dep_target.load_state() != UnitLoadState::Loaded
        {
            log::debug!("dep unit  is not loaded, continue");
            return;
        }
        if !unit.default_dependencies() || !dep_target.default_dependencies() {
            log::debug!("default dependencies option is false");
            return;
        }
        if db.dep_is_dep_atom_with(&dep_target, b_atom, unit) {
            return;
        }

        if let Err(_e) = db.dep_insert(dep_target, after, Rc::clone(unit), true, mask as u16) {
            log::error!("dispatch_target_dep_queue add default dep err {:?}", _e);
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manager::RELI_HISTORY_MAX_DBS;
    use crate::unit::data::DataManager;
    use crate::unit::rentry::UnitRe;
    use crate::unit::test;
    use basic::logger;
    use sysmaster::rel::Reliability;

    #[test]
    fn rt_push_load_queue() {
        let dm = Rc::new(DataManager::new());
        let reli = Rc::new(Reliability::new(RELI_HISTORY_MAX_DBS));
        let rentry = Rc::new(UnitRe::new(&reli));
        let db = Rc::new(UnitDb::new(&rentry));
        let rt = UnitRT::new(&reli, &rentry, &db);
        let name_test1 = String::from("test1.service");
        let unit_test1 = create_unit(&dm, &reli, &rentry, &name_test1);
        let name_test2 = String::from("test2.service");
        let unit_test2 = create_unit(&dm, &reli, &rentry, &name_test2);

        assert_eq!(rt.data.load_queue.borrow().len(), 0);
        assert!(!unit_test1.in_load_queue());
        assert!(!unit_test2.in_load_queue());

        rt.push_load_queue(Rc::clone(&unit_test1));
        assert_eq!(rt.data.load_queue.borrow().len(), 1);
        assert!(unit_test1.in_load_queue());
        assert!(!unit_test2.in_load_queue());

        rt.push_load_queue(Rc::clone(&unit_test2));
        assert_eq!(rt.data.load_queue.borrow().len(), 2);
        assert!(unit_test1.in_load_queue());
        assert!(unit_test2.in_load_queue());
    }

    #[test]
    fn rt_dispatch_load_queue() {
        let dm = Rc::new(DataManager::new());
        let reli = Rc::new(Reliability::new(RELI_HISTORY_MAX_DBS));
        let rentry = Rc::new(UnitRe::new(&reli));
        let db = Rc::new(UnitDb::new(&rentry));
        let rt = UnitRT::new(&reli, &rentry, &db);
        let service_name = String::from("config.service");
        let service_unit = create_unit(&dm, &reli, &rentry, &service_name);
        rt.push_load_queue(Rc::clone(&service_unit));
        rt.data
            .db
            .units_insert(service_name.to_string(), service_unit);
        rt.dispatch_load_queue(); // do not register dep notify so cannot parse dependency
        let unit = rt.data.db.units_get(&service_name);
        assert_eq!(unit.unwrap().load_state(), UnitLoadState::Loaded);
    }

    #[test]
    fn rt_dispatch_target_dep_queue() {
        let dm = Rc::new(DataManager::new());
        let reli = Rc::new(Reliability::new(RELI_HISTORY_MAX_DBS));
        let rentry = Rc::new(UnitRe::new(&reli));
        let db = Rc::new(UnitDb::new(&rentry));
        let rt = UnitRT::new(&reli, &rentry, &db);
        let test_service_name = String::from("test.service");
        let test_service_unit = create_unit(&dm, &reli, &rentry, &test_service_name);
        rt.data
            .db
            .units_insert(test_service_name, Rc::clone(&test_service_unit));
        rt.push_load_queue(Rc::clone(&test_service_unit));
        let service_name = String::from("config.service");
        let service_unit = create_unit(&dm, &reli, &rentry, &service_name);
        rt.data
            .db
            .units_insert(service_name, Rc::clone(&service_unit));
        rt.push_load_queue(Rc::clone(&service_unit));
        let target_name = String::from("testsunit.target");
        let target_unit = create_unit(&dm, &reli, &rentry, &target_name);
        rt.data
            .db
            .units_insert(target_name, Rc::clone(&target_unit));
        rt.push_load_queue(Rc::clone(&target_unit));
        rt.dispatch_load_queue();
    }

    fn create_unit(
        dmr: &Rc<DataManager>,
        relir: &Rc<Reliability>,
        rentryr: &Rc<UnitRe>,
        name: &str,
    ) -> Rc<UnitX> {
        logger::init_log_to_console("test_unit_load", log::LevelFilter::Trace);
        log::info!("test");
        test::test_utils::create_unit_for_test_pub(dmr, relir, rentryr, name)
    }
}
