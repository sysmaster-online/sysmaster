#![allow(unused_imports)]
pub use job_entry::{JobConf, JobInfo, JobKind, JobResult, JobStage};
pub use job_manager::{JobAffect, JobManager};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum JobErrno {
    JobErrInput,
    JobErrConflict,
    JobErrNotExisted,
    JobErrInternel,
    JobErrNotSupported,
    JobErrBadRequest,
}

use crate::manager::MngErrno;
impl From<JobErrno> for MngErrno {
    fn from(err: JobErrno) -> Self {
        match err {
            _ => MngErrno::MngErrInternel,
        }
    }
}

// dependency: job_entry -> {job_unit_entry | job_alloc} -> job_table -> {job_transaction | job_notify | job_stat} -> job_manager
mod job_alloc;
mod job_entry;
mod job_manager;
mod job_notify;
mod job_stat;
mod job_table;
mod job_transaction;
mod job_unit_entry;
