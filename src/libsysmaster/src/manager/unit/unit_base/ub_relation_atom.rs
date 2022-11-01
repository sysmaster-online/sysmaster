#![warn(unused_imports)]
use crate::manager::unit::unit_rentry::UnitRelations;

#[allow(missing_docs)]
#[derive(Eq, PartialEq, Clone, Copy, Debug)]
#[repr(u64)]
pub enum UnitRelationAtom {
    UnitAtomPullInStart = 1u64 << 0,
    UnitAtomPullInStartIgnored = 1u64 << 1,
    UnitAtomPullInVerify = 1u64 << 2,
    UnitAtomPullInStop = 1u64 << 3,
    UnitAtomPullInStopIgnored = 1u64 << 4,
    UnitAtomAddStopWhenUnneededQueue = 1u64 << 5,
    UnitAtomPinsStopWhenUnneeded = 1u64 << 6,
    UnitAtomCannotBeActiveWithout = 1u64 << 7,
    UnitAtomAddCannotBeActiveWithoutQueue = 1u64 << 8,
    UnitAtomStartSteadily = 1u64 << 9,
    UnitAtomAddStartWhenUpheldQueue = 1u64 << 10,
    UnitAtomRetroActiveStartReplace = 1u64 << 11,
    UnitAtomRetroActiveStartFail = 1u64 << 12,
    UnitAtomRetroActiveStopOnStart = 1u64 << 13,
    UnitAtomRetroActiveStopOnStop = 1u64 << 14,
    UnitAtomPropagateStartFailure = 1u64 << 15,
    UnitAtomPropagateStopFailure = 1u64 << 16,
    UnitAtomPropagateInactiveStartAsFailure = 1u64 << 17,
    UnitAtomPropagateStop = 1u64 << 18,
    UnitAtomPropagateRestart = 1u64 << 19,
    UnitAtomAddDefaultTargetDependencyQueue = 1u64 << 20,
    UnitAtomDefaultTargetDependencies = 1u64 << 21,
    UnitAtomBefore = 1u64 << 22,
    UnitAtomAfter = 1u64 << 23,
    UnitAtomOnSuccess = 1u64 << 24,
    UnitAtomOnFailure = 1u64 << 25,
    UnitAtomTriggers = 1u64 << 26,
    UnitAtomTriggeredBy = 1u64 << 27,
    UnitAtomPropagatesReloadTo = 1u64 << 28,
    UnitAtomJoinsNameSpaceOf = 1u64 << 29,
    UnitAtomReferences = 1u64 << 30,
    UnitAtomReferencedBy = 1u64 << 31,
    UnitAtomInSlice = 1u64 << 32,
    UnitAtomSliceOf = 1u64 << 33,
}

pub(in crate::manager::unit) fn unit_relation_to_atom(
    relation: UnitRelations,
) -> Vec<UnitRelationAtom> {
    let mut atoms = Vec::new();
    match relation {
        UnitRelations::UnitRequires => {
            atoms.push(UnitRelationAtom::UnitAtomPullInStart);
            atoms.push(UnitRelationAtom::UnitAtomRetroActiveStartReplace);
            atoms.push(UnitRelationAtom::UnitAtomAddStopWhenUnneededQueue);
            atoms.push(UnitRelationAtom::UnitAtomAddDefaultTargetDependencyQueue);
        }
        UnitRelations::UnitRequisite => {
            atoms.push(UnitRelationAtom::UnitAtomPullInVerify);
            atoms.push(UnitRelationAtom::UnitAtomAddStopWhenUnneededQueue);
            atoms.push(UnitRelationAtom::UnitAtomAddDefaultTargetDependencyQueue);
        }
        UnitRelations::UnitWants => {
            atoms.push(UnitRelationAtom::UnitAtomPullInStartIgnored);
            atoms.push(UnitRelationAtom::UnitAtomRetroActiveStartFail);
            atoms.push(UnitRelationAtom::UnitAtomAddStopWhenUnneededQueue);
            atoms.push(UnitRelationAtom::UnitAtomDefaultTargetDependencies);
        }
        UnitRelations::UnitBindsTo => {
            atoms.push(UnitRelationAtom::UnitAtomPullInStart);
            atoms.push(UnitRelationAtom::UnitAtomRetroActiveStartReplace);
            atoms.push(UnitRelationAtom::UnitAtomCannotBeActiveWithout);
            atoms.push(UnitRelationAtom::UnitAtomAddStopWhenUnneededQueue);
            atoms.push(UnitRelationAtom::UnitAtomAddDefaultTargetDependencyQueue);
        }
        UnitRelations::UnitPartOf => {
            atoms.push(UnitRelationAtom::UnitAtomAddDefaultTargetDependencyQueue);
        }
        UnitRelations::UnitUpHolds => {
            atoms.push(UnitRelationAtom::UnitAtomPullInStartIgnored);
            atoms.push(UnitRelationAtom::UnitAtomRetroActiveStartReplace);
            atoms.push(UnitRelationAtom::UnitAtomAddStartWhenUpheldQueue);
            atoms.push(UnitRelationAtom::UnitAtomAddStopWhenUnneededQueue);
            atoms.push(UnitRelationAtom::UnitAtomAddDefaultTargetDependencyQueue);
        }
        UnitRelations::UnitRequiresBy => {
            atoms.push(UnitRelationAtom::UnitAtomPropagateStop);
            atoms.push(UnitRelationAtom::UnitAtomPropagateRestart);
            atoms.push(UnitRelationAtom::UnitAtomPropagateStartFailure);
            atoms.push(UnitRelationAtom::UnitAtomPinsStopWhenUnneeded);
            atoms.push(UnitRelationAtom::UnitAtomDefaultTargetDependencies);
        }
        UnitRelations::UnitRequisiteOf => {
            atoms.push(UnitRelationAtom::UnitAtomPropagateStop);
            atoms.push(UnitRelationAtom::UnitAtomPropagateRestart);
            atoms.push(UnitRelationAtom::UnitAtomPropagateStartFailure);
            atoms.push(UnitRelationAtom::UnitAtomPropagateInactiveStartAsFailure);
            atoms.push(UnitRelationAtom::UnitAtomPinsStopWhenUnneeded);
            atoms.push(UnitRelationAtom::UnitAtomDefaultTargetDependencies);
        }
        UnitRelations::UnitWantsBy => {
            atoms.push(UnitRelationAtom::UnitAtomDefaultTargetDependencies);
            atoms.push(UnitRelationAtom::UnitAtomPinsStopWhenUnneeded);
        }
        UnitRelations::UnitBoundBy => {
            atoms.push(UnitRelationAtom::UnitAtomRetroActiveStopOnStop);
            atoms.push(UnitRelationAtom::UnitAtomPropagateStop);
            atoms.push(UnitRelationAtom::UnitAtomPropagateRestart);
            atoms.push(UnitRelationAtom::UnitAtomPropagateStartFailure);
            atoms.push(UnitRelationAtom::UnitAtomPinsStopWhenUnneeded);
            atoms.push(UnitRelationAtom::UnitAtomAddCannotBeActiveWithoutQueue);
            atoms.push(UnitRelationAtom::UnitAtomDefaultTargetDependencies);
        }
        UnitRelations::UnitUpHeldBy => {
            atoms.push(UnitRelationAtom::UnitAtomStartSteadily);
            atoms.push(UnitRelationAtom::UnitAtomDefaultTargetDependencies);
            atoms.push(UnitRelationAtom::UnitAtomPinsStopWhenUnneeded);
        }
        UnitRelations::UnitConsistsOf => {
            atoms.push(UnitRelationAtom::UnitAtomPropagateStop);
            atoms.push(UnitRelationAtom::UnitAtomPropagateRestart);
        }
        UnitRelations::UnitConflicts => {
            atoms.push(UnitRelationAtom::UnitAtomPullInStop);
            atoms.push(UnitRelationAtom::UnitAtomRetroActiveStopOnStart);
        }
        UnitRelations::UnitConflictedBy => {
            atoms.push(UnitRelationAtom::UnitAtomPullInStopIgnored);
            atoms.push(UnitRelationAtom::UnitAtomRetroActiveStopOnStart);
            atoms.push(UnitRelationAtom::UnitAtomPropagateStopFailure);
        }
        UnitRelations::UnitPropagatesStopTo => {
            atoms.push(UnitRelationAtom::UnitAtomRetroActiveStopOnStop);
            atoms.push(UnitRelationAtom::UnitAtomPropagateStop);
        }
        UnitRelations::UnitBefore => {
            atoms.push(UnitRelationAtom::UnitAtomBefore);
        }
        UnitRelations::UnitAfter => {
            atoms.push(UnitRelationAtom::UnitAtomAfter);
        }
        UnitRelations::UnitOnSuccess => {
            atoms.push(UnitRelationAtom::UnitAtomOnSuccess);
        }
        UnitRelations::UnitOnFailure => {
            atoms.push(UnitRelationAtom::UnitAtomOnFailure);
        }
        UnitRelations::UnitTriggers => {
            atoms.push(UnitRelationAtom::UnitAtomTriggers);
        }
        UnitRelations::UnitTriggeredBy => {
            atoms.push(UnitRelationAtom::UnitAtomTriggeredBy);
        }
        UnitRelations::UnitPropagatesReloadTo => {
            atoms.push(UnitRelationAtom::UnitAtomPropagatesReloadTo);
        }
        UnitRelations::UnitJoinsNameSpaceOf => {
            atoms.push(UnitRelationAtom::UnitAtomJoinsNameSpaceOf);
        }
        UnitRelations::UnitReferences => {
            atoms.push(UnitRelationAtom::UnitAtomReferences);
        }
        UnitRelations::UnitReferencedBy => {
            atoms.push(UnitRelationAtom::UnitAtomReferences);
        }
        UnitRelations::UnitInSlice => {
            atoms.push(UnitRelationAtom::UnitAtomInSlice);
        }
        UnitRelations::UnitSliceOf => {
            atoms.push(UnitRelationAtom::UnitAtomSliceOf);
        }
        UnitRelations::UnitReloadPropagatedFrom
        | UnitRelations::UnitOnSuccessOf
        | UnitRelations::UnitonFailureOf
        | UnitRelations::UnitStopPropagatedFrom => {} // empty
    };
    atoms
}

pub(in crate::manager::unit) fn unit_relation_from_unique_atom(
    atom: UnitRelationAtom,
) -> Vec<UnitRelations> {
    let mut deps = Vec::new();
    match atom {
        UnitRelationAtom::UnitAtomPullInStart => {
            deps.push(UnitRelations::UnitRequires);
            deps.push(UnitRelations::UnitBindsTo);
        }
        UnitRelationAtom::UnitAtomPullInStartIgnored => {
            deps.push(UnitRelations::UnitWants);
            deps.push(UnitRelations::UnitUpHolds);
        }
        UnitRelationAtom::UnitAtomPullInVerify => {
            deps.push(UnitRelations::UnitRequisite);
        }
        UnitRelationAtom::UnitAtomPullInStop => {
            deps.push(UnitRelations::UnitConflicts);
        }
        UnitRelationAtom::UnitAtomPullInStopIgnored => {
            deps.push(UnitRelations::UnitConflictedBy);
        }
        UnitRelationAtom::UnitAtomAddStopWhenUnneededQueue => {
            deps.push(UnitRelations::UnitRequires);
            deps.push(UnitRelations::UnitRequisite);
            deps.push(UnitRelations::UnitWants);
            deps.push(UnitRelations::UnitBindsTo);
            deps.push(UnitRelations::UnitUpHolds);
        }
        UnitRelationAtom::UnitAtomPinsStopWhenUnneeded => {
            deps.push(UnitRelations::UnitRequiresBy);
            deps.push(UnitRelations::UnitRequisiteOf);
            deps.push(UnitRelations::UnitWantsBy);
            deps.push(UnitRelations::UnitBoundBy);
            deps.push(UnitRelations::UnitUpHeldBy);
        }
        UnitRelationAtom::UnitAtomCannotBeActiveWithout => {
            deps.push(UnitRelations::UnitBindsTo);
        }
        UnitRelationAtom::UnitAtomAddCannotBeActiveWithoutQueue => {
            deps.push(UnitRelations::UnitBoundBy);
        }
        UnitRelationAtom::UnitAtomStartSteadily => {
            deps.push(UnitRelations::UnitUpHeldBy);
        }
        UnitRelationAtom::UnitAtomAddStartWhenUpheldQueue => {
            deps.push(UnitRelations::UnitUpHolds);
        }
        UnitRelationAtom::UnitAtomRetroActiveStartReplace => {
            deps.push(UnitRelations::UnitRequires);
            deps.push(UnitRelations::UnitBindsTo);
            deps.push(UnitRelations::UnitUpHolds);
        }
        UnitRelationAtom::UnitAtomRetroActiveStartFail => {
            deps.push(UnitRelations::UnitWants);
        }
        UnitRelationAtom::UnitAtomRetroActiveStopOnStart => {
            deps.push(UnitRelations::UnitConflicts);
            deps.push(UnitRelations::UnitConflictedBy);
        }
        UnitRelationAtom::UnitAtomRetroActiveStopOnStop => {
            deps.push(UnitRelations::UnitBoundBy);
            deps.push(UnitRelations::UnitPropagatesStopTo);
        }
        UnitRelationAtom::UnitAtomPropagateStartFailure => {
            deps.push(UnitRelations::UnitRequiresBy);
            deps.push(UnitRelations::UnitRequisiteOf);
            deps.push(UnitRelations::UnitBoundBy);
        }
        UnitRelationAtom::UnitAtomPropagateStopFailure => {
            deps.push(UnitRelations::UnitConflictedBy);
        }
        UnitRelationAtom::UnitAtomPropagateInactiveStartAsFailure => {
            deps.push(UnitRelations::UnitRequisiteOf);
        }
        UnitRelationAtom::UnitAtomPropagateStop => {
            deps.push(UnitRelations::UnitRequiresBy);
            deps.push(UnitRelations::UnitRequisiteOf);
            deps.push(UnitRelations::UnitBoundBy);
            deps.push(UnitRelations::UnitConsistsOf);
            deps.push(UnitRelations::UnitPropagatesStopTo);
        }
        UnitRelationAtom::UnitAtomPropagateRestart => {
            deps.push(UnitRelations::UnitRequiresBy);
            deps.push(UnitRelations::UnitRequisiteOf);
            deps.push(UnitRelations::UnitBoundBy);
            deps.push(UnitRelations::UnitConsistsOf);
        }
        UnitRelationAtom::UnitAtomAddDefaultTargetDependencyQueue => {
            deps.push(UnitRelations::UnitRequires);
            deps.push(UnitRelations::UnitRequisite);
            deps.push(UnitRelations::UnitWants);
            deps.push(UnitRelations::UnitBindsTo);
            deps.push(UnitRelations::UnitPartOf);
            deps.push(UnitRelations::UnitUpHolds);
        }
        UnitRelationAtom::UnitAtomDefaultTargetDependencies => {
            deps.push(UnitRelations::UnitRequiresBy);
            deps.push(UnitRelations::UnitRequisiteOf);
            deps.push(UnitRelations::UnitWantsBy);
            deps.push(UnitRelations::UnitBoundBy);
            deps.push(UnitRelations::UnitUpHeldBy);
        }
        UnitRelationAtom::UnitAtomBefore => {
            deps.push(UnitRelations::UnitBefore);
        }
        UnitRelationAtom::UnitAtomAfter => {
            deps.push(UnitRelations::UnitAfter);
        }
        UnitRelationAtom::UnitAtomOnSuccess => {
            deps.push(UnitRelations::UnitOnSuccess);
        }
        UnitRelationAtom::UnitAtomOnFailure => {
            deps.push(UnitRelations::UnitOnFailure);
        }
        UnitRelationAtom::UnitAtomTriggers => {
            deps.push(UnitRelations::UnitTriggers);
        }
        UnitRelationAtom::UnitAtomTriggeredBy => {
            deps.push(UnitRelations::UnitTriggeredBy);
        }
        UnitRelationAtom::UnitAtomPropagatesReloadTo => {
            deps.push(UnitRelations::UnitPropagatesReloadTo);
        }
        UnitRelationAtom::UnitAtomJoinsNameSpaceOf => {
            deps.push(UnitRelations::UnitJoinsNameSpaceOf);
        }
        UnitRelationAtom::UnitAtomReferences => {
            deps.push(UnitRelations::UnitReferences);
        }
        UnitRelationAtom::UnitAtomReferencedBy => {
            deps.push(UnitRelations::UnitReferencedBy);
        }
        UnitRelationAtom::UnitAtomInSlice => {
            deps.push(UnitRelations::UnitInSlice);
        }
        UnitRelationAtom::UnitAtomSliceOf => {
            deps.push(UnitRelations::UnitSliceOf);
        }
    };
    deps
}
