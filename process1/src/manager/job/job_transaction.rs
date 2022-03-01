use std::collections::HashMap;
use crate::manager::unit::{Unit};
use super::{JobError};
use super::job_entry::{JobKind, JobMode, Job};
use super::job_table::{JobTable};

pub(super) fn job_trans_expand(trans:&JobTable, job:&Job) {
    // job + dependency(from db) + unit-list(from db) -> job-list
    // trans.insert(job-list);
    todo!();
}

pub(super) fn job_trans_verify(trans:&JobTable) {
    // job-list + unit-list(from db) -> job-list' => trnas
    todo!();
}

