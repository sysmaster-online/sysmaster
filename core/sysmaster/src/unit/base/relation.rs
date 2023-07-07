// Copyright (c) 2022 Huawei Technologies Co.,Ltd. All rights reserved.
//
// sysMaster is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan
// PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY
// KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

use core::unit::UnitRelations;

pub(in super::super) fn unit_relation_to_inverse(relation: UnitRelations) -> UnitRelations {
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
        UnitRelations::UnitOnFailure => UnitRelations::UnitOnFailureOf,
        UnitRelations::UnitOnFailureOf => UnitRelations::UnitOnFailure,
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
