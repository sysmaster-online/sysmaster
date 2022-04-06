use super::unit_entry::UnitX;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

#[derive(Debug)]
pub(super) struct UnitRT {
    data: RefCell<UnitRTData>,
}

impl UnitRT {
    pub(super) fn new() -> UnitRT {
        UnitRT {
            data: RefCell::new(UnitRTData::new()),
        }
    }

    pub(super) fn dispatch_load_queue(&self) {
        self.data.borrow_mut().dispatch_load_queue()
    }

    pub(super) fn push_load_queue(&self, unit: Rc<UnitX>) {
        self.data.borrow_mut().push_load_queue(unit)
    }
}

#[derive(Debug)]
struct UnitRTData {
    load_queue: VecDeque<Rc<UnitX>>,
}

// the declaration "pub(self)" is for identification only.
impl UnitRTData {
    pub(self) fn new() -> UnitRTData {
        UnitRTData {
            load_queue: VecDeque::new(),
        }
    }

    pub fn dispatch_load_queue(&mut self) {
        log::debug!("dispatch load queue");

        loop {
            match self.load_queue.pop_front() {
                None => break,
                Some(unit) => match unit.load() {
                    Ok(()) => continue,
                    Err(e) => {
                        log::error!("load unit config failed: {}", e.to_string());
                        println!("load unit config failed: {}", e.to_string())
                    }
                },
            }
        }
    }

    pub fn push_load_queue(&mut self, unit: Rc<UnitX>) {
        if unit.in_load_queue() {
            return;
        }
        self.load_queue.push_back(unit);
    }
}
