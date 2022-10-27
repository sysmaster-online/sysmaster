#![warn(unused_imports)]
use super::job_entry::{Job, JobConf};
use super::job_rentry::{JobKind, JobRe};
use crate::manager::unit::unit_entry::UnitX;
use crate::reliability::Reliability;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

pub(super) struct JobAlloc {
    // associated objects
    reli: Rc<Reliability>,
    rentry: Rc<JobRe>,

    // owned objects
    data: RefCell<JobAllocData>,
}

impl JobAlloc {
    pub(super) fn new(relir: &Rc<Reliability>, rentryr: &Rc<JobRe>) -> JobAlloc {
        JobAlloc {
            reli: Rc::clone(relir),
            rentry: Rc::clone(rentryr),
            data: RefCell::new(JobAllocData::new()),
        }
    }

    pub(super) fn clear(&self) {
        self.data.borrow_mut().clear();
    }

    pub(super) fn alloc(&self, config: &JobConf) -> Rc<Job> {
        let unit = config.get_unit();
        let kind = config.get_kind();
        self.data
            .borrow_mut()
            .alloc(&self.reli, &self.rentry, Rc::clone(unit), kind)
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

    pub(self) fn clear(&mut self) {
        self.ids.clear();
        self.last_id = 0;
    }

    pub(self) fn alloc(
        &mut self,
        relir: &Rc<Reliability>,
        rentryr: &Rc<JobRe>,
        unit: Rc<UnitX>,
        kind: JobKind,
    ) -> Rc<Job> {
        Rc::new(Job::new(relir, rentryr, self.alloc_id(), unit, kind))
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
