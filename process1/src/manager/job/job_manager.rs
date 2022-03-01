use std::collections::{LinkedList};
use crate::manager::unit::{Unit};
use super::{JobError};
use super::job_entry::{JobKind, JobMode, Job};
use super::job_table::{JobTable};
use super::job_transaction;

struct JobStat {
    // stage
    init:u32,
    install:u32,
    running:u32,
    end:u32,

    // result
    done:u32,
    cancelled:u32,
    failed:u32,
}

pub(in crate::manager) struct JobManager<'a> {
    // control
    cur_idx:u32,

    // data
    /* stable */
    jobs:JobTable<'a>,
    wait:LinkedList<&'a Job<'a>>,

    /* temporary */
    trans:JobTable<'a>,

    // statistics
    stat:JobStat,
}

pub(in crate::manager) struct JobAffect<'a, T> {
    pub(in crate::manager) input:&'a Job<'a>,
    pub(in crate::manager) others:T,
}

impl<'a> JobManager<'a> {
    pub(in crate::manager) fn new() -> Box<JobManager<'a>> {
        Box::new(JobManager {
            cur_idx:0,

            jobs:JobTable::new(),
            wait:LinkedList::new(),
            trans:JobTable::new(),

            stat:JobStat{
                init:0,
                install:0,
                running:0,
                end:0,

                done:0,
                cancelled:0,
                failed:0,
            }
        })
    }

    fn trans_add(&self, unit:&Unit, kind:JobKind) -> Result<&Job, JobError> {
        // id = self.id++; job = Job::new(id, unit, kind); self.trans.insert(job);
        todo!();
    }

    fn trans_expand(&self, job:&Job, mode:JobMode) -> Result<(), JobError> {
        job_transaction::job_trans_expand(&self.trans, job);
        todo!();
    }

    fn trans_verify(&self) -> Result<(), JobError> {
        job_transaction::job_trans_verify(&self.trans);
        todo!();
    }

    fn trans_commit<T:IntoIterator>(&self, affect:&mut JobAffect<T>) -> Result<u32, JobError> {
        // self.trans <-exchange-> self.jobs
        // self.trans.clear();
        // update self.stat
        todo!();
    }

    pub(in crate::manager) fn add_job<T:IntoIterator>(&self, unit:&Unit, kind:JobKind, mode:JobMode, affect:&mut JobAffect<T>) -> Result<u32, JobError> {
        // input-check
        // job = trans_add
        // trans_expand + trans_verify + trans_commit
        // apply job-list: fail --> wait
        todo!();
    }

    pub(in crate::manager) fn del_job(&self, job:&Job) -> Result<u32, JobError> {
        // self.jobs.remove(job.id); update self.stat
        todo!();
    }

    pub(in crate::manager) fn get(&self, id:u32) -> Option<&Job> {
        // self.jobs.get(id);
        todo!();
    }

    pub(in crate::manager) fn get_mut(&self, id:u32) -> Option<&mut Job> {
        // self.jobs.get_mut(id);
        todo!();
    }
}


