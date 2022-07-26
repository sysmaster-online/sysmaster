#![warn(unused_imports)]

use serde::{Deserialize, Deserializer, Serialize};

use crate::manager::unit::DeserializeWith;

#[derive(Clone, Debug, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum JobMode {
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

impl DeserializeWith for JobMode {
    fn deserialize_with<'de, D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(de)?;

        match s.as_ref() {
            "fail" => Ok(JobMode::JobFail),
            "replace" => Ok(JobMode::JobReplace),
            "replace_irreversible" => Ok(JobMode::JobReplaceIrreversible),
            "isolate" => Ok(JobMode::JobIsolate),
            "flush" => Ok(JobMode::JobFlush),
            "ignore_dependencies" => Ok(JobMode::JobIgnoreDependencies),
            "ignore_requirements" => Ok(JobMode::JobIgnoreRequirements),
            "trigger" => Ok(JobMode::JobTrigger),
            &_ => Ok(JobMode::JobReplace),
        }
    }
}
