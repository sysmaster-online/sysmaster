pub(in crate::manager) use job_entry::{Job, JobKind, JobMode, JobResult, JobStage};
pub(in crate::manager) use job_manager::JobManager;

pub(in crate::manager) enum JobError {
    JobOk,
    JobErrConflict,
    JobErrInternel,
}

mod job_entry;
mod job_manager;
mod job_table;
mod job_transaction;
