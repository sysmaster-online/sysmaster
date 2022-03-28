use super::job_entry::{Job, JobKind, JobMode};
use super::job_table::JobTable;
use super::JobError;
use crate::manager::unit::Unit;
use std::collections::HashMap;

pub(super) fn job_trans_expand(trans: &JobTable, job: &Job) {
    // job + dependency(from db) + unit-list(from db) -> job-list
    // trans.insert(job-list);
    todo!();
}

pub(super) fn job_trans_verify(trans: &JobTable) {
    // job-list + unit-list(from db) -> job-list' => trnas
    todo!();
}
