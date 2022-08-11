use super::unit_base::UnitRelationAtom;
use super::unit_datastore::UnitDb;
use super::unit_entry::UnitX;
use super::UnitType;
use crate::manager::table::{TableOp, TableSubscribe};
use crate::manager::unit::unit_base::UnitLoadState;
use crate::manager::UnitRelations;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

//#[derive(Debug)]
pub(super) struct UnitRT {
    // associated objects
    db: Rc<UnitDb>,

    // owned objects
    sub_name: String, // key for table-subscriber: UnitSets
    data: Rc<UnitRTData>,
}

impl UnitRT {
    pub(super) fn new(dbr: &Rc<UnitDb>) -> UnitRT {
        let rt = UnitRT {
            db: Rc::clone(dbr),
            sub_name: String::from("UnitRT"),
            data: Rc::new(UnitRTData::new(dbr)),
        };
        rt.register(dbr);
        rt
    }

    pub(super) fn dispatch_load_queue(&self) {
        self.data.dispatch_load_queue();
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
    db: Rc<UnitDb>,
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

// the declaration "pub(self)" is for identification only.
impl UnitRTData {
    pub(self) fn new(dbr: &Rc<UnitDb>) -> UnitRTData {
        UnitRTData {
            db: Rc::clone(dbr),
            load_queue: RefCell::new(VecDeque::new()),
            target_dep_queue: RefCell::new(VecDeque::new()),
        }
    }

    pub(self) fn dispatch_load_queue(&self) {
        log::debug!("dispatch load queue");

        loop {
            //Limit the scope of borrow of load queue
            //unitX pop from the load queue and then no need the ref of load queue
            //the unitX load process will borrow load queue as mut again
            let first_unit = self.load_queue.borrow_mut().pop_front();
            match first_unit {
                None => break,
                Some(unit) => match unit.load() {
                    Ok(()) => {
                        let load_state = unit.load_state();
                        if load_state == UnitLoadState::UnitLoaded {
                            self.push_target_dep_queue(Rc::clone(&unit));
                        }
                    }
                    Err(e) => {
                        log::error!("load unit [{}] failed: {}", unit.get_id(), e.to_string());
                    }
                },
            }
        }
        self.dispatch_target_dep_queue();
    }

    fn dispatch_target_dep_queue(&self) {
        loop {
            let first_unit = self.target_dep_queue.borrow_mut().pop_front();
            match first_unit {
                None => break,
                Some(unit) => {
                    unit.set_in_target_dep_queue(false);
                    for dep_target in self
                        .db
                        .dep_gets_atom(&unit, UnitRelationAtom::UnitAtomDefaultTargetDependencies)
                    {
                        if dep_target.unit_type() != UnitType::UnitTarget {
                            log::debug!("dep unit type is not target, continue");
                            continue;
                        }
                        if unit.load_state() != UnitLoadState::UnitLoaded
                            || dep_target.load_state() != UnitLoadState::UnitLoaded
                        {
                            log::debug!("dep unit  is not loaded, continue");
                            continue;
                        }

                        if self.db.dep_is_dep_atom_with(
                            &unit,
                            UnitRelationAtom::UnitAtomBefore,
                            &dep_target,
                        ) {
                            continue;
                        }

                        if let Err(_e) = self.db.dep_insert(
                            Rc::clone(&unit),
                            UnitRelations::UnitAfter,
                            dep_target,
                            true,
                            1 << 2,
                        ) {
                            log::error!("dispatch_target_dep_queue add defalt dep err {:?}", _e);
                            return;
                        }
                    }
                }
            }
        }
    }

    fn push_target_dep_queue(&self, unit: Rc<UnitX>) {
        if unit.in_target_dep_queue() {
            return;
        }
        log::debug!("push unit [{}] into target dep queue", unit.get_id());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manager::data::DataManager;
    use crate::manager::unit::uload_util::UnitFile;
    use crate::manager::unit::unit_base::{self};
    use crate::plugin::Plugin;
    use utils::logger;

    fn init_rt() -> UnitRT {
        let db = Rc::new(UnitDb::new());
        return UnitRT::new(&db);
    }

    #[test]
    fn rt_push_load_queue() {
        let rt = init_rt();
        let name_test1 = String::from("test1.service");
        let unit_test1 = create_unit(&name_test1);
        let name_test2 = String::from("test2.service");
        let unit_test2 = create_unit(&name_test2);

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
        let rt = init_rt();
        let service_name = String::from("config.service");
        let service_unit = create_unit(&service_name);
        rt.push_load_queue(Rc::clone(&service_unit));
        rt.data
            .db
            .units_insert((&service_name).to_string(), service_unit);
        rt.dispatch_load_queue(); // do not register dep notify so cannot parse dependency
        let unit = rt.data.db.units_get(&service_name);
        assert_eq!(unit.unwrap().load_state(), UnitLoadState::UnitLoaded);
    }

    #[test]
    fn rt_dispatch_target_dep_queue() {
        let rt = init_rt();
        let test_service_name = String::from("test.service");
        let test_service_unit = create_unit(&test_service_name);
        rt.data.db.units_insert(
            (&test_service_name).to_string(),
            Rc::clone(&test_service_unit),
        );
        rt.push_load_queue(Rc::clone(&test_service_unit));
        let service_name = String::from("config.service");
        let service_unit = create_unit(&service_name);
        rt.data
            .db
            .units_insert((&service_name).to_string(), Rc::clone(&service_unit));
        rt.push_load_queue(Rc::clone(&service_unit));
        let target_name = String::from("testsunit.target");
        let target_unit = create_unit(&target_name);
        rt.data
            .db
            .units_insert((&target_name).to_string(), Rc::clone(&target_unit));
        rt.push_load_queue(Rc::clone(&target_unit));
        rt.dispatch_load_queue();
    }

    fn create_unit(name: &str) -> Rc<UnitX> {
        logger::init_log_with_console("unit_runtime", 4);
        let dm = Rc::new(DataManager::new());
        let file = Rc::new(UnitFile::new());
        let unit_type = unit_base::unit_name_to_type(name);
        let plugins = Plugin::get_instance();
        let subclass = plugins.create_unit_obj(unit_type).unwrap();
        Rc::new(UnitX::new(
            &dm,
            &file,
            unit_type,
            name,
            subclass.into_unitobj(),
        ))
    }
}
