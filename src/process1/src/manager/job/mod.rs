pub(in crate::manager) use job_entry::{JobKind, JobMode, JobResult, JobStage, Job};
pub(in crate::manager) use job_manager::{JobManager};

pub(in crate::manager) enum JobError {
    JobOk,
    JobErrConflict,
    JobErrInternel,
}

mod job_entry;
mod job_table;
mod job_transaction;
mod job_manager;
