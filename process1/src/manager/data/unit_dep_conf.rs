use core::fmt::{Display, Formatter, Result};

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
            UnitRelations::UnitRequiresBy => write!(f, "RequiresBy"),
            UnitRelations::UnitRequisiteOf => todo!(),
            UnitRelations::UnitWantsBy => write!(f, "WantsBy"),
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

pub struct UnitDepConf {
    pub deps: Vec<(UnitRelations, String)>,
}

impl UnitDepConf {
    pub fn new() -> UnitDepConf {
        UnitDepConf { deps: Vec::new() }
    }
}
