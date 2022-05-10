use super::unit_datastore::UnitDb;
use super::unit_entry::UnitX;
use crate::manager::table::{TableOp, TableSubscribe};
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
            data: Rc::new(UnitRTData::new()),
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
    load_queue: RefCell<VecDeque<Rc<UnitX>>>,
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
    pub(self) fn new() -> UnitRTData {
        UnitRTData {
            load_queue: RefCell::new(VecDeque::new()),
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
                    Ok(()) => continue,
                    Err(e) => {
                        log::error!("load unit [{}] failed: {}", unit.get_id(), e.to_string());
                    }
                },
            }
        }
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
    use crate::manager::unit::uload_util::{UnitFile, UnitParserMgr};
    use crate::manager::unit::unit_base::UnitType;
    use crate::plugin::Plugin;
    use utils::logger;

    #[test]
    fn rt_push_load_queue() {
        let db = Rc::new(UnitDb::new());
        let rt = UnitRT::new(&db);
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

    fn create_unit(name: &str) -> Rc<UnitX> {
        logger::init_log_with_console("test_unit_load", 4);
        log::info!("test");
        let dm = Rc::new(DataManager::new());
        let file = Rc::new(UnitFile::new());
        let unit_conf_parser_mgr = Rc::new(UnitParserMgr::default());
        let unit_type = UnitType::UnitService;
        let plugins = Plugin::get_instance();
        let subclass = plugins.create_unit_obj(unit_type).unwrap();
        Rc::new(UnitX::new(
            &dm,
            &file,
            &unit_conf_parser_mgr,
            unit_type,
            name,
            subclass.into_unitobj(),
        ))
    }
}
