#![warn(unused_imports)]

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(in crate::manager::unit) enum JobMode {
    #[serde(alias = "fail")]
    JobFail,
    #[serde(alias = "replace")]
    JobReplace,
    #[serde(alias = "replace_irreversible")]
    JobReplaceIrreversible,
    #[serde(alias = "isolate")]
    JobIsolate,
    #[serde(alias = "flush")]
    JobFlush,
    #[serde(alias = "ignore_dependencies")]
    JobIgnoreDependencies,
    #[serde(alias = "ignore_requirements")]
    JobIgnoreRequirements,
    #[serde(alias = "trigger")]
    JobTrigger,
}

impl Default for JobMode {
    fn default() -> Self {
        JobMode::JobReplace
    }
}
