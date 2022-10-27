#![warn(unused_imports)]
use crate::manager::unit::unit_rentry::UnitRelations;

pub(in crate::manager::unit) fn unit_relation_to_inverse(relation: UnitRelations) -> UnitRelations {
    match relation {
        UnitRelations::UnitRequires => UnitRelations::UnitRequiresBy,
        UnitRelations::UnitRequisite => UnitRelations::UnitRequisiteOf,
        UnitRelations::UnitWants => UnitRelations::UnitWantsBy,
        UnitRelations::UnitBindsTo => UnitRelations::UnitBoundBy,
        UnitRelations::UnitPartOf => UnitRelations::UnitConsistsOf,
        UnitRelations::UnitUpHolds => UnitRelations::UnitUpHeldBy,
        UnitRelations::UnitRequiresBy => UnitRelations::UnitRequires,
        UnitRelations::UnitRequisiteOf => UnitRelations::UnitRequisite,
        UnitRelations::UnitWantsBy => UnitRelations::UnitWants,
        UnitRelations::UnitBoundBy => UnitRelations::UnitBindsTo,
        UnitRelations::UnitConsistsOf => UnitRelations::UnitPartOf,
        UnitRelations::UnitUpHeldBy => UnitRelations::UnitUpHolds,
        UnitRelations::UnitConflicts => UnitRelations::UnitConflictedBy,
        UnitRelations::UnitConflictedBy => UnitRelations::UnitConflicts,
        UnitRelations::UnitBefore => UnitRelations::UnitAfter,
        UnitRelations::UnitAfter => UnitRelations::UnitBefore,
        UnitRelations::UnitOnSuccess => UnitRelations::UnitOnSuccessOf,
        UnitRelations::UnitOnSuccessOf => UnitRelations::UnitOnSuccess,
        UnitRelations::UnitOnFailure => UnitRelations::UnitonFailureOf,
        UnitRelations::UnitonFailureOf => UnitRelations::UnitOnFailure,
        UnitRelations::UnitTriggers => UnitRelations::UnitTriggeredBy,
        UnitRelations::UnitTriggeredBy => UnitRelations::UnitTriggers,
        UnitRelations::UnitPropagatesReloadTo => UnitRelations::UnitReloadPropagatedFrom,
        UnitRelations::UnitReloadPropagatedFrom => UnitRelations::UnitPropagatesReloadTo,
        UnitRelations::UnitPropagatesStopTo => UnitRelations::UnitStopPropagatedFrom,
        UnitRelations::UnitStopPropagatedFrom => UnitRelations::UnitPropagatesStopTo,
        UnitRelations::UnitJoinsNameSpaceOf => UnitRelations::UnitJoinsNameSpaceOf,
        UnitRelations::UnitReferences => UnitRelations::UnitReferencedBy,
        UnitRelations::UnitReferencedBy => UnitRelations::UnitReferences,
        UnitRelations::UnitInSlice => UnitRelations::UnitSliceOf,
        UnitRelations::UnitSliceOf => UnitRelations::UnitInSlice,
    }
}
