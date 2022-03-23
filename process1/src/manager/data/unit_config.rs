use core::fmt::{Display, Formatter, Result};

use crate::null_str;

#[derive(PartialEq, Eq, Hash, Copy, Clone)]
pub enum UnitType {
    UnitService = 0,
    UnitTarget,
    UnitTypeMax,
    UnitTypeInvalid,
    UnitTypeErrnoMax,
}

impl From<UnitType> for String {
    fn from(u_t: UnitType) -> Self {
        match u_t {
            UnitType::UnitService => "Service".into(),
            UnitType::UnitTarget => "Target".into(),
            UnitType::UnitTypeMax => "Max".into(),
            UnitType::UnitTypeInvalid => null_str!("").into(),
            UnitType::UnitTypeErrnoMax => null_str!("").into(),
        }
    }
}
impl Display for UnitType {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            UnitType::UnitService => write!(f, "Service"),
            UnitType::UnitTarget => write!(f, "Target"),
            UnitType::UnitTypeMax => write!(f, "Max"),
            UnitType::UnitTypeInvalid => write!(f, ""),
            UnitType::UnitTypeErrnoMax => write!(f, ""),
        }
    }
}
#[derive(Hash, PartialEq, Eq, Copy, Clone)]
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

impl Display for UnitRelations {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            UnitRelations::UnitRequires => write!(f, "Requires"),
            UnitRelations::UnitRequisite => write!(f, "Requisite"),
            UnitRelations::UnitWants => write!(f, "Wants"),
            UnitRelations::UnitBindsTo => write!(f, "BindsTo"),
            UnitRelations::UnitPartOf => write!(f, "PartOf"),
            UnitRelations::UnitUpHolds => write!(f, "UpHolds"),
            UnitRelations::UnitRequiresBy => todo!(),
            UnitRelations::UnitRequisiteOf => todo!(),
            UnitRelations::UnitWantsBy => todo!(),
            UnitRelations::UnitBoundBy => todo!(),
            UnitRelations::UnitConsistsOf => todo!(),
            UnitRelations::UnitUpHeldBy => todo!(),
            UnitRelations::UnitConflicts => todo!(),
            UnitRelations::UnitConflictedBy => todo!(),
            UnitRelations::UnitBefore => write!(f, "Before"),
            UnitRelations::UnitAfter => write!(f, "After"),
            UnitRelations::UnitOnSuccess => todo!(),
            UnitRelations::UnitOnSuccessOf => todo!(),
            UnitRelations::UnitOnFailure => todo!(),
            UnitRelations::UnitonFailureOf => todo!(),
            UnitRelations::UnitTriggers => todo!(),
            UnitRelations::UnitTriggeredBy => todo!(),
            UnitRelations::UnitPropagatesReloadTo => todo!(),
            UnitRelations::UnitReloadPropagatedFrom => todo!(),
            UnitRelations::UnitPropagatesStopTo => todo!(),
            UnitRelations::UnitStopPropagatedFrom => todo!(),
            UnitRelations::UnitJoinsNameSpaceOf => todo!(),
            UnitRelations::UnitReferences => todo!(),
            UnitRelations::UnitReferencedBy => todo!(),
            UnitRelations::UnitInSlice => todo!(),
            UnitRelations::UnitSliceOf => todo!(),
        }
    }
}

impl From<UnitRelations> for String {
    fn from(unit_relations: UnitRelations) -> Self {
        match unit_relations {
            UnitRelations::UnitAfter => "After".into(),
            UnitRelations::UnitRequires => "Requires".into(),
            UnitRelations::UnitRequisite => "Requisite".into(),
            UnitRelations::UnitWants => "Wants".into(),
            UnitRelations::UnitBindsTo => "BindsTo".into(),
            UnitRelations::UnitPartOf => "PartOf".into(),
            UnitRelations::UnitUpHolds => "UpHolds".into(),
            UnitRelations::UnitRequiresBy => todo!(),
            UnitRelations::UnitRequisiteOf => todo!(),
            UnitRelations::UnitWantsBy => todo!(),
            UnitRelations::UnitBoundBy => todo!(),
            UnitRelations::UnitConsistsOf => todo!(),
            UnitRelations::UnitUpHeldBy => todo!(),
            UnitRelations::UnitConflicts => todo!(),
            UnitRelations::UnitConflictedBy => todo!(),
            UnitRelations::UnitBefore => todo!(),
            UnitRelations::UnitOnSuccess => todo!(),
            UnitRelations::UnitOnSuccessOf => todo!(),
            UnitRelations::UnitOnFailure => todo!(),
            UnitRelations::UnitonFailureOf => todo!(),
            UnitRelations::UnitTriggers => todo!(),
            UnitRelations::UnitTriggeredBy => todo!(),
            UnitRelations::UnitPropagatesReloadTo => todo!(),
            UnitRelations::UnitReloadPropagatedFrom => todo!(),
            UnitRelations::UnitPropagatesStopTo => todo!(),
            UnitRelations::UnitStopPropagatedFrom => todo!(),
            UnitRelations::UnitJoinsNameSpaceOf => todo!(),
            UnitRelations::UnitReferences => todo!(),
            UnitRelations::UnitReferencedBy => todo!(),
            UnitRelations::UnitInSlice => todo!(),
            UnitRelations::UnitSliceOf => todo!(),
        }
    }
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
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

pub struct UnitConfig {
    pub name: String,
    pub deps: Vec<(UnitRelations, String)>,
    pub desc: String,
    pub documentation: String,
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
            documentation: null_str!(""),
            allow_isolate: false,
            ignore_on_isolate: false,
            on_success_job_mode: JobMode::JobFail,
            on_failure_job_mode: JobMode::JobFail,
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }
}
