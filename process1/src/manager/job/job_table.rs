use super::job_entry::Job;
use super::JobError;
use crate::manager::unit::Unit;
use std::collections::{HashMap, LinkedList};

pub(super) struct JobTable<'a> {
    t_id: HashMap<u32, Box<Job<'a>>>, // key: job-id, data: job
    t_unit: HashMap<&'a Unit, LinkedList<&'a Job<'a>>>, // key: &unit, data: (&job)-list
}

impl<'a> JobTable<'a> {
    pub(super) fn new() -> JobTable<'a> {
        JobTable {
            t_id: HashMap::new(),
            t_unit: HashMap::new(),
        }
    }

    pub(super) fn insert(&self, job: Job) -> Option<u32> {
        // self.t_id.insert(job.id, job);
        // ll = self.t_unit.get(job.unit); if ll == None {ll = LinkedList::new();
        // self.t_unit.insert(job.unit, ll)}; ll.append(job);
        // job.id
        todo!();
    }

    pub(super) fn remove(&self, job: &Job) -> Option<u32> {
        // id = job.id
        // ll = self.t_unit.get(job.unit); if ll != None {ll.remove(job)} else
        // {self.t_unit.remove(job.unit)};
        // self.t_id.remove(id);
        // id
        todo!();
    }

    pub(super) fn get(&self, id: u32) -> Option<&Job> {
        // self.t_id.get(id);
        todo!();
    }

    pub(super) fn get_mut(&self, id: u32) -> Option<&Job> {
        // self.t_id.get_mut(id);
        todo!();
    }

    pub(super) fn get_jobs<T: IntoIterator>(&self, unit: &Unit, jobs: &mut T) -> Option<u32> {
        // ll = self.t_unit.get(unit); ll => jobs;
        todo!();
    }

    pub(super) fn clear(&self) {
        // self.t_unit.clear();
        // self.t_id.clear();
        todo!();
    }
}
