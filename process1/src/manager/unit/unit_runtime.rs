use super::unit_entry::UnitX;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

//#[derive(Debug)]
pub(super) struct UnitRT {
    data: UnitRTData,
}

impl UnitRT {
    pub(super) fn new() -> UnitRT {
        UnitRT {
            data: UnitRTData::new(),
        }
    }

    pub(super) fn dispatch_load_queue(&self) {
        self.data.dispatch_load_queue();
    }

    pub(super) fn push_load_queue(&self, unit: Rc<UnitX>) {
        self.data.push_load_queue(unit);
    }
}

//#[derive(Debug)]
struct UnitRTData {
    load_queue: RefCell<VecDeque<Rc<UnitX>>>,
}

// the declaration "pub(self)" is for identification only.
impl UnitRTData {
    pub(self) fn new() -> UnitRTData {
        UnitRTData {
            load_queue: RefCell::new(VecDeque::new()),
        }
    }

    pub fn dispatch_load_queue(&self) {
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

    pub fn push_load_queue(&self, unit: Rc<UnitX>) {
        if unit.in_load_queue() {
            return;
        }
        unit.set_in_load_queue(true);
        self.load_queue.borrow_mut().push_back(unit);
    }
}
