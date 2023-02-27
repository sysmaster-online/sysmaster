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

/**Unit stats：
 A[UnitActive]
 B[UnitReloading]
 C[UnitInActive]
 D[UnitFailed]
 E[UnitActivating]
 F[UnitDeActivating]
 G[UnitMaintenance]
 ```graph LR
C[UnitInActive] -> E[UnitActivating]
E->A[UnitActive]
B[UnitReloading] -> E
E->F[UnitDeActivating]
E->D[UnitFailed]
```
*/
#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum UnitActiveState {
    /// unit is activated
    UnitActive,
    /// unit is in reloading
    UnitReloading,
    /// unit is not active
    UnitInActive,
    /// unit action is failed
    UnitFailed,
    /// unit is in starting
    UnitActivating,
    /// unit is in stopping
    UnitDeActivating,
    /// unit is in maintenance
    UnitMaintenance,
}

impl UnitActiveState {
    ///
    pub fn is_active_or_reloading(&self) -> bool {
        matches!(
            self,
            UnitActiveState::UnitActive | UnitActiveState::UnitReloading
        )
    }

    ///
    pub fn is_inactive_or_failed(&self) -> bool {
        matches!(
            self,
            UnitActiveState::UnitInActive | UnitActiveState::UnitFailed
        )
    }

    ///
    pub fn is_active_or_activating(&self) -> bool {
        matches!(
            self,
            UnitActiveState::UnitActive
                | UnitActiveState::UnitActivating
                | UnitActiveState::UnitReloading
        )
    }

    ///
    pub fn is_inactive_or_deactivating(&self) -> bool {
        matches!(
            self,
            UnitActiveState::UnitInActive
                | UnitActiveState::UnitFailed
                | UnitActiveState::UnitDeActivating
        )
    }
}

impl std::fmt::Display for UnitActiveState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnitActiveState::UnitActive => write!(f, "active"),
            UnitActiveState::UnitReloading => write!(f, "reloading"),
            UnitActiveState::UnitInActive => write!(f, "inactive"),
            UnitActiveState::UnitFailed => write!(f, "failed"),
            UnitActiveState::UnitActivating => write!(f, "activating"),
            UnitActiveState::UnitDeActivating => write!(f, "deactivating"),
            UnitActiveState::UnitMaintenance => write!(f, "maintenance"),
        }
    }
}

bitflags! {
    /// notify unit state to manager
    pub struct UnitNotifyFlags: u8 {
        /// notify reload success to manager
        const UNIT_NOTIFY_SUCCESS = 0;
        /// notify reload failure to manager
        const UNIT_NOTIFY_RELOAD_FAILURE = 1 << 0;
        /// notify auto restart to manager
        const UNIT_NOTIFY_WILL_AUTO_RESTART = 1 << 1;
    }
}
