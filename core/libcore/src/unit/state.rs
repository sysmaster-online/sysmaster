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

use basic::show_table::{CellAlign, CellColor, ShowTable};
use bitflags::bitflags;

/**Unit stats：
 ```graph LR
 A[UnitActive]
 B[UnitReloading]
 C[UnitInActive]
 D[UnitFailed]
 E[UnitActivating]
 F[UnitDeActivating]
 G[UnitMaintenance]
 ```
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
    Active,
    /// unit is in reloading
    Reloading,
    /// unit is not active
    InActive,
    /// unit action is failed
    Failed,
    /// unit is in starting
    Activating,
    /// unit is in stopping
    DeActivating,
    /// unit is in maintenance
    Maintenance,
}

impl UnitActiveState {
    ///
    pub fn is_active_or_reloading(&self) -> bool {
        matches!(self, UnitActiveState::Active | UnitActiveState::Reloading)
    }

    ///
    pub fn is_inactive_or_failed(&self) -> bool {
        matches!(self, UnitActiveState::InActive | UnitActiveState::Failed)
    }

    ///
    pub fn is_active_or_activating(&self) -> bool {
        matches!(
            self,
            UnitActiveState::Active | UnitActiveState::Activating | UnitActiveState::Reloading
        )
    }

    ///
    pub fn is_inactive_or_deactivating(&self) -> bool {
        matches!(
            self,
            UnitActiveState::InActive | UnitActiveState::Failed | UnitActiveState::DeActivating
        )
    }
}

impl std::fmt::Display for UnitActiveState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnitActiveState::Active => write!(f, "active"),
            UnitActiveState::Reloading => write!(f, "reloading"),
            UnitActiveState::InActive => write!(f, "inactive"),
            UnitActiveState::Failed => write!(f, "failed"),
            UnitActiveState::Activating => write!(f, "activating"),
            UnitActiveState::DeActivating => write!(f, "deactivating"),
            UnitActiveState::Maintenance => write!(f, "maintenance"),
        }
    }
}

bitflags! {
    /// notify unit state to manager
    pub struct UnitNotifyFlags: u8 {
        /// the default flags propagate to jobs, it means nothing.
        const EMPTY = 0;
        /// notify that the unit running reload failure
        const RELOAD_FAILURE = 1 << 0;
        /// notify that the unit is in auto restart state
        const WILL_AUTO_RESTART = 1 << 1;
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
        let full_active_state = self.active_state.to_string() + " (" + &self.sub_state + ")";

        let mut color = CellColor::Empty;
        if self.active_state == "active" {
            color = CellColor::Green;
        } else if self.active_state == "failed" {
            color = CellColor::Red;
        }

        status_table.add_line(vec!["Loaded:", &self.load_state]);
        status_table.add_line(vec!["Active:", &full_active_state]);
        status_table.add_line(vec!["CGroup:", &self.cgroup_path]);
        if let Some(doc) = &self.documentation {
            status_table.add_line(vec!["Docs:", doc]);
        }
        status_table.add_line(vec!["PID:", &self.pid]);
        status_table.set_one_col_align(0, CellAlign::Right);
        /* The first column: keep the left space, delete the right space. */
        status_table.set_one_col_space(0, true, false);
        /* Cell (1, 1) is used to show the unit state, make it colored. */
        status_table.set_one_cell_color(1, 1, color);

        let mut first_line =
            "\x1b".to_string() + &String::from(color) + "● " + "\x1b[0m" + &self.name;
        first_line = match &self.description {
            None => first_line + "\n",
            Some(str) => first_line + " - " + str + "\n",
        };
        write!(f, "{}", first_line + &status_table.to_string())
    }
}

impl From<UnitStatus> for nix::Error {
    fn from(status: UnitStatus) -> Self {
        nix::Error::from_i32(status.error_code)
    }
}
