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

use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use std::{convert::TryFrom, num::ParseIntError, str::FromStr};

#[allow(missing_docs)]
#[derive(Hash, PartialEq, Eq, Copy, Clone, Debug, Serialize, Deserialize)]
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
    UnitOnFailureOf,

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

#[allow(missing_docs)]
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum UnitDependencyMask {
    File = 1 << 0,
    Implicit = 1 << 1,
    Default = 1 << 2,
}

#[allow(missing_docs)]
#[derive(PartialEq, Eq, Hash, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum UnitType {
    UnitService = 0,
    UnitTarget,
    UnitSocket,
    UnitMount,
    UnitTimer,
    UnitPath,
    UnitTypeMax,
    UnitTypeInvalid,
    UnitTypeErrnoMax,
}

impl UnitType {
    ///
    pub fn iterator() -> impl Iterator<Item = UnitType> {
        [
            UnitType::UnitService,
            UnitType::UnitTarget,
            UnitType::UnitSocket,
            UnitType::UnitMount,
            UnitType::UnitTimer,
            UnitType::UnitPath,
        ]
        .iter()
        .copied()
    }
}

impl FromStr for UnitType {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ret = match s.to_lowercase().as_str() {
            "service" => UnitType::UnitService,
            "target" => UnitType::UnitTarget,
            "socket" => UnitType::UnitSocket,
            "mount" => UnitType::UnitMount,
            "timer" => UnitType::UnitTimer,
            "path" => UnitType::UnitPath,
            _ => UnitType::UnitTypeInvalid,
        };
        Ok(ret)
    }
}

impl From<UnitType> for String {
    fn from(u_t: UnitType) -> Self {
        match u_t {
            UnitType::UnitService => "service".into(),
            UnitType::UnitTarget => "target".into(),
            UnitType::UnitSocket => "socket".into(),
            UnitType::UnitMount => "mount".into(),
            UnitType::UnitTimer => "timer".into(),
            UnitType::UnitPath => "path".into(),
            UnitType::UnitTypeMax => null_str!(""),
            UnitType::UnitTypeInvalid => null_str!(""),
            UnitType::UnitTypeErrnoMax => null_str!(""),
        }
    }
}

impl TryFrom<u32> for UnitType {
    type Error = String;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(UnitType::UnitService),
            1 => Ok(UnitType::UnitTarget),
            2 => Ok(UnitType::UnitSocket),
            3 => Ok(UnitType::UnitMount),
            4 => Ok(UnitType::UnitTimer),
            5 => Ok(UnitType::UnitPath),
            v => Err(format!("input {} is invalid", v)),
        }
    }
}

/// parse UnitType by unit_name
pub fn unit_name_to_type(unit_name: &str) -> UnitType {
    let words: Vec<&str> = unit_name.split('.').collect();
    if words.is_empty() {
        return UnitType::UnitTypeInvalid;
    }
    UnitType::from_str(words[words.len() - 1]).unwrap_or(UnitType::UnitTypeInvalid)
}

bitflags! {
    /// flags used when writing files
    pub struct UnitWriteFlags: u8 {
        /// runtime file
        const RUNTIME = 1 << 0;
        /// persistent file
        const PERSISTENT = 1 << 1;
        /// sub-unit type section in file
        const PRIVATE = 1 << 2;
    }
}

///
pub fn unit_write_flags_is_noop(flags: UnitWriteFlags) -> bool {
    !flags.intersects(UnitWriteFlags::RUNTIME | UnitWriteFlags::PERSISTENT)
}
