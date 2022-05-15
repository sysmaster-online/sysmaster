#![warn(unused_imports)]

use core::fmt::{Display, Formatter, Result};

const JOBFAIL: &str = "fail";
const JOBREPLACE: &str = "replace";
const JOBREPLACEIRREVERSIBLE: &str = "replace_irreversible";
const JOBISOLATE: &str = "isolate";
const JOBFLUSH: &str = "flush";
const JOBIGNOREDEPENDENCIES: &str = "ignore_dependencies";
const JOBIGNOREREQUIREMENTS: &str = "ignore_requirements";
const JOBTRIGGER: &str = "trigger";

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub(in crate::manager::unit) enum JobMode {
    JobFail,
    JobReplace,
    JobReplaceIrreversible,
    JobIsolate,
    JobFlush,
    JobIgnoreDependencies,
    JobIgnoreRequirements,
    JobTrigger,
}

impl Display for JobMode {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            JobMode::JobFail => write!(f, "fail"),
            JobMode::JobReplace => write!(f, "replace"),
            JobMode::JobReplaceIrreversible => write!(f, "replace_irreversible"),
            JobMode::JobIsolate => write!(f, "isolate"),
            JobMode::JobFlush => write!(f, "flush"),
            JobMode::JobIgnoreDependencies => write!(f, "ignore_dependencies"),
            JobMode::JobIgnoreRequirements => write!(f, "ignore_requirements"),
            JobMode::JobTrigger => write!(f, "trigger"),
        }
    }
}

impl From<String> for JobMode {
    fn from(str: String) -> Self {
        match str.as_str() {
            JOBFAIL => JobMode::JobFail,
            JOBREPLACE => JobMode::JobReplace,
            JOBREPLACEIRREVERSIBLE => JobMode::JobReplaceIrreversible,
            JOBISOLATE => JobMode::JobIsolate,
            JOBFLUSH => JobMode::JobFlush,
            JOBIGNOREDEPENDENCIES => JobMode::JobIgnoreDependencies,
            JOBIGNOREREQUIREMENTS => JobMode::JobIgnoreRequirements,
            JOBTRIGGER => JobMode::JobTrigger,
            _ => JobMode::JobFail,
        }
    }
}

impl From<JobMode> for String {
    fn from(_job_mode: JobMode) -> Self {
        match _job_mode {
            JobMode::JobFail => "fail".into(),
            JobMode::JobReplace => "replace".into(),
            JobMode::JobReplaceIrreversible => "replace_irreversible".into(),
            JobMode::JobIsolate => "isolate".into(),
            JobMode::JobFlush => "flush".into(),
            JobMode::JobIgnoreDependencies => "ignore_dependencies".into(),
            JobMode::JobIgnoreRequirements => "ignore_requirements".into(),
            JobMode::JobTrigger => "trigger".into(),
        }
    }
}
