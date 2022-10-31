use super::unit_base::{UnitDependencyMask, UnitRelationAtom};
use super::unit_datastore::UnitDb;
use super::unit_entry::UnitX;
use super::unit_rentry::{UnitLoadState, UnitType};
use super::unit_rentry::{UnitRe, UnitRePps};
use crate::manager::rentry::{ReliLastFrame, ReliLastQue};
use crate::manager::table::{TableOp, TableSubscribe};
use crate::manager::UnitRelations;
use crate::reliability::{ReStation, Reliability};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::convert::TryFrom;
use std::rc::Rc;

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
    fn db_map(&self) {
        self.data.db_map();
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

    pub(super) fn get_dependency_list(
        &self,
        source: &UnitX,
        atom: UnitRelationAtom,
    ) -> Vec<Rc<UnitX>> {
        self.data.get_dependency_list(source, atom)
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
        self.rentry.pps_clear(lunit, UnitRePps::QUEUE_LOAD);
    }

    fn rc_last_queue_targetdeps(&self, lunit: &String) {
        // remove from pps, which would be compensated later(dc_last_queue_targetdeps).
        self.rentry.pps_clear(lunit, UnitRePps::QUEUE_TARGET_DEPS);
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
        let unit = self.db.units_get(lunit).unwrap();
        if let Err(e) = unit.load() {
            log::error!("load unit [{}] failed: {}", unit.id(), e.to_string());
        }
    }

    fn dc_last_queue_targetdeps(&self, lunit: &str) {
        // retry
        let unit = self.db.units_get(lunit).unwrap();
        dispatch_target_dep_unit(&self.db, &unit);
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

    pub(self) fn db_map(&self) {
        for unit_id in self.rentry.pps_keys().iter() {
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
                            if load_state == UnitLoadState::UnitLoaded {
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

    pub(self) fn get_dependency_list(
        &self,
        source: &UnitX,
        atom: UnitRelationAtom,
    ) -> Vec<Rc<UnitX>> {
        self.db.dep_gets_atom(source, atom)
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

    fn remove_unit(&self, _unit: &UnitX) {
        todo!();
    }
}

fn dispatch_target_dep_unit(db: &Rc<UnitDb>, unit: &Rc<UnitX>) {
    unit.set_in_target_dep_queue(false);
    let atom = UnitRelationAtom::UnitAtomDefaultTargetDependencies;
    let b_atom = UnitRelationAtom::UnitAtomBefore;
    let after = UnitRelations::UnitAfter;
    let mask = UnitDependencyMask::UnitDependencyDefault;
    for dep_target in db.dep_gets_atom(unit, atom) {
        if dep_target.unit_type() != UnitType::UnitTarget {
            log::debug!("dep unit type is not target, continue");
            return;
        }
        if unit.load_state() != UnitLoadState::UnitLoaded
            || dep_target.load_state() != UnitLoadState::UnitLoaded
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

        if let Err(_e) = db.dep_insert(Rc::clone(unit), after, dep_target, true, mask as u16) {
            log::error!("dispatch_target_dep_queue add default dep err {:?}", _e);
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manager::rentry::RELI_HISTORY_MAX_DBS;
    use crate::manager::unit::data::DataManager;
    use crate::manager::unit::uload_util::UnitFile;
    use crate::manager::unit::unit_rentry::UnitRe;
    use crate::plugin::Plugin;
    use libutils::logger;
    use libutils::path_lookup::LookupPaths;

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
        assert_eq!(unit.unwrap().load_state(), UnitLoadState::UnitLoaded);
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
        logger::init_log_with_console("test_unit_load", 4);
        log::info!("test");

        let mut l_path = LookupPaths::new();
        l_path.init_lookup_paths();
        let lookup_path = Rc::new(l_path);

        let file = Rc::new(UnitFile::new(&lookup_path));
        let unit_type = UnitType::UnitService;

        let plugins = Plugin::get_instance();
        let subclass = plugins.create_unit_obj(unit_type).unwrap();
        subclass.attach_reli(Rc::clone(relir));
        Rc::new(UnitX::new(
            dmr,
            rentryr,
            &file,
            unit_type,
            name,
            subclass.into_unitobj(),
        ))
    }
}
