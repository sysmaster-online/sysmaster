use crate::null_str;

#[derive(Debug, PartialEq, Eq, Hash, Copy, Clone)]
pub enum UnitType {
    UnitService = 0,
    UnitTarget,
    UnitTypeMax,
    UnitTypeInvalid,
    UnitTypeErrnoMax,
}

#[derive(Hash, PartialEq, Eq, Debug, Copy, Clone)]
pub enum UnitRelations {
    UnitRequires,
    UnitRequisite,
    UnitWants,
    UnitBindsTo,
    UnitPartOf,
    UnitUpHolds,

    UnitRequiresBy,
    UnitRequisiteOf,
    UnitWantsBy,
    UnitBoundBy,
    UnitConsistsOf,
    UnitUpHeldBy,

    UnitConflicts,
    UnitConflictedBy,

    UnitBefore,
    UnitAfter,

    UnitOnSuccess,
    UnitOnSuccessOf,
    UnitOnFailure,
    UnitonFailureOf,

    UnitTriggers,
    UnitTriggeredBy,

    UnitPropagatesReloadTo,
    UnitReloadPropagatedFrom,

    UnitPropagatesStopTo,
    UnitStopPropagatedFrom,

    UnitJoinsNameSpaceOf,

    UnitReferences,
    UnitReferencedBy,

    UnitInSlice,
    UnitSliceOf,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum JobMode {
    JobFail,
    JobReplace,
    JobReplaceIrreversible,
    JobIsolate,
    JobFlush,
    JobIgnoreDependencies,
    JobIgnoreRequirements,
    JobTrigger,
}

pub enum UnitConfigItem {
    UcItemName(String),
    UcItemDesc(String),
    UcItemDoc(String),
    UcItemAllowIsolate(bool),
    UcItemIgnoreOnIsolate(bool),
    UcItemOnSucJobMode(JobMode),
    UcItemOnFailJobMode(JobMode),
}

#[derive(Debug)]
pub struct UnitConfig {
    pub name: String,
    pub deps: Vec<(UnitRelations, String)>,
    pub desc: String,
    pub documnetation: String,
    pub allow_isolate: bool,
    pub ignore_on_isolate: bool,
    pub on_success_job_mode: JobMode,
    pub on_failure_job_mode: JobMode,
}

impl UnitConfig {
    pub fn new() -> UnitConfig {
        UnitConfig {
            name: String::from(""),
            deps: Vec::new(),
            desc: String::from(""),
            documnetation: null_str!(""),
            allow_isolate: false,
            ignore_on_isolate: false,
            on_success_job_mode: JobMode::JobFail,
            on_failure_job_mode: JobMode::JobFail,
        }
    }
}
