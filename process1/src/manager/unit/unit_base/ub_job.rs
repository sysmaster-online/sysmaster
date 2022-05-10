#![warn(unused_imports)]

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
