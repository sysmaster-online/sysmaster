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

use basic::show_table::ShowTable;
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

/// UnitStatus is used to display unit's status
pub struct UnitStatus {
    name: String,
    description: Option<String>,
    documentation: Option<String>,
    load_state: String,
    sub_state: String,
    active_state: String,
    cgroup_path: String,
    pid: String,
    error_code: i32,
}

impl UnitStatus {
    #[allow(clippy::too_many_arguments)]
    ///
    pub fn new(
        name: String,
        description: Option<String>,
        documentation: Option<String>,
        load_state: String,
        sub_state: String,
        active_state: String,
        cgroup_path: String,
        pid: String,
        error_code: i32,
    ) -> Self {
        Self {
            name,
            description,
            documentation,
            load_state,
            sub_state,
            active_state,
            cgroup_path,
            pid,
            error_code,
        }
    }
}

impl std::fmt::Display for UnitStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut status_table = ShowTable::new();
        let full_active_state = self.active_state.to_string() + "(" + &self.sub_state + ")";
        status_table.add_line(vec!["Loaded:", &self.load_state]);
        status_table.add_line(vec!["Active:", &full_active_state]);
        status_table.add_line(vec!["CGroup:", &self.cgroup_path]);
        if let Some(doc) = &self.documentation {
            status_table.add_line(vec!["Docs:", doc]);
        }
        status_table.add_line(vec!["PID:", &self.pid]);
        status_table.set_one_cell_align_right(0);
        status_table.align_define();
        let first_line = match &self.description {
            None => "● ".to_string() + &self.name + "\n",
            Some(str) => "● ".to_string() + &self.name + " - " + str + "\n",
        };
        write!(f, "{}", first_line + &status_table.to_string())
    }
}

impl From<UnitStatus> for nix::Error {
    fn from(status: UnitStatus) -> Self {
        nix::Error::from_i32(status.error_code)
    }
}
