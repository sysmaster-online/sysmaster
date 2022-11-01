#![allow(unused_imports)]
pub(super) use job_entry::JobConf;
pub(in crate::manager) use job_entry::{JobInfo, JobResult, JobStage};
pub(super) use job_manager::{JobAffect, JobManager};
pub(in crate::manager) use job_rentry::JobKind;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum JobErrno {
    Input,
    Conflict,
    NotExisted,
    Internal,
    NotSupported,
    BadRequest,
}

use crate::manager::MngErrno;
impl From<JobErrno> for MngErrno {
    fn from(err: JobErrno) -> Self {
        match err {
            JobErrno::Input => MngErrno::Input,
            JobErrno::NotExisted => MngErrno::NotExisted,
            JobErrno::NotSupported => MngErrno::NotSupported,
            _ => MngErrno::Internal,
        }
    }
}

// dependency:
// job_rentry -> job_entry ->
// {job_unit_entry | job_alloc} -> job_table ->
// {job_transaction | job_notify | job_stat} -> job_manager
mod job_alloc;
mod job_entry;
mod job_manager;
mod job_notify;
mod job_rentry;
mod job_stat;
mod job_table;
mod job_transaction;
mod job_unit_entry;
