use super::JobError;
use crate::manager::unit::Unit;

pub(in crate::manager) enum JobKind {
    // 'type' is better, but it's keyword in rust
    JobStart,
    JobStop,
    JobReload,
    JobNop,
}

pub(in crate::manager) enum JobMode {
    JobFail,
    JobReplace,
}

pub(in crate::manager) enum JobResult {
    JobDone,
    JobCancelled,
    JobFailed,
}

pub(in crate::manager) enum JobStage {
    JobInit,
    JobInstall,
    JobRunning,
    JobEnd(JobResult),
}

pub(in crate::manager) struct Job<'a> {
    // key: input
    id: u32,

    // data
    /* config: input */
    unit: &'a Unit,
    kind: JobKind,

    /* status: self-generated */
    stage: JobStage,
}

impl<'a> Job<'a> {
    pub(super) fn new(id: u32, unit: &Unit, kind: JobKind) -> Box<Job> {
        Box::new(Job {
            id,
            unit,
            kind,
            stage: JobStage::JobInit,
        })
    }

    pub(super) fn merge(job: &mut Job, kind: JobKind) -> Result<JobKind, JobError> {
        // job.kind = func(kind, job.kind, job.unit);
        todo!();
    }

    pub(super) fn apply(job: &mut Job) -> Result<(), JobError> {
        // job.stage = JobStage::JobInstall; enqueue job; job.stage = JobStage::JobRunning;
        todo!();
    }

    pub(super) fn action(job: &mut Job) -> Result<(), JobError> {
        // perform job.unit; job.stage = JobStage::JobEnd{JobResult::JobEnd};
        todo!();
    }
}

#[cfg(test)]
mod tests {
    // use super::*;

    #[test]
    fn job_test_new() {
        // let unit = Unit::new();
        // let id = 1;
        // let kind = JobKind::JobNop;
        // let job = Job::new(id, unit, kind);
        // assert_eq!(job.unit, &unit);
    }
}
