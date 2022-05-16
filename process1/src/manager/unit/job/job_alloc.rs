#![warn(unused_imports)]
use super::job_entry::{Job, JobKind};
use crate::manager::unit::unit_base::JobMode;
use crate::manager::unit::unit_entry::UnitX;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

pub(super) struct JobAlloc {
    data: RefCell<JobAllocData>,
}

impl JobAlloc {
    pub(super) fn new() -> JobAlloc {
        JobAlloc {
            data: RefCell::new(JobAllocData::new()),
        }
    }

    pub(super) fn alloc(&self, unit: Rc<UnitX>, kind: JobKind, mode: JobMode) -> Rc<Job> {
        self.data.borrow_mut().alloc(unit, kind, mode)
    }
}

#[derive(Debug)]
struct JobAllocData {
    // data
    ids: HashSet<u32>,

    // status
    last_id: u32,
}

// the declaration "pub(self)" is for identification only.
impl JobAllocData {
    pub(self) fn new() -> JobAllocData {
        JobAllocData {
            ids: HashSet::new(),
            last_id: 0,
        }
    }

    pub(self) fn alloc(&mut self, unit: Rc<UnitX>, kind: JobKind, mode: JobMode) -> Rc<Job> {
        let job = Rc::new(Job::new(self.alloc_id(), unit, kind));
        job.init_attr(mode);
        job
    }

    fn alloc_id(&mut self) -> u32 {
        loop {
            let (id, _) = self.last_id.overflowing_add(1); // ++
            self.last_id = id;
            if !self.ids.insert(id) {
                // the 'id' has been allocated
                continue; // try next ++
            }

            return id;
        }
    }
}
