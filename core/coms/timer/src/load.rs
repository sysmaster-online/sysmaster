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

use crate::{comm::TimerUnitComm, config::TimerConfig};
use basic::{SHUTDOWN_TARGET, SYSINIT_TARGET, TIMERS_TARGET};
use core::{
    error::*,
    unit::{UnitDependencyMask, UnitRelations, UnitType},
};
use std::{path::Path, rc::Rc};

pub(super) struct TimerLoad {
    config: Rc<TimerConfig>,
    comm: Rc<TimerUnitComm>,
}

impl TimerLoad {
    pub(super) fn new(configr: &Rc<TimerConfig>, commr: &Rc<TimerUnitComm>) -> Self {
        TimerLoad {
            config: configr.clone(),
            comm: commr.clone(),
        }
    }

    pub fn timer_add_extras(&self) -> Result<()> {
        log::debug!("timer add extras");
        if self.config.unit_ref_target().is_empty() {
            self.load_related_unit(UnitType::UnitService)?;
        }
        if let Some(owner) = self.comm.owner() {
            let um = self.comm.um();
            um.unit_add_two_dependency(
                &owner.id(),
                UnitRelations::UnitBefore,
                UnitRelations::UnitTriggers,
                &self.config.unit_ref_target(),
                true,
                UnitDependencyMask::Implicit,
            )?;
        }

        self.add_default_dependencies()?;

        Ok(())
    }

    fn load_related_unit(&self, related_type: UnitType) -> Result<()> {
        let unit_name = self.comm.owner().map(|u| u.id());
        let suffix = String::from(related_type);
        if suffix.is_empty() {
            return Err(format!("failed to load related unit {}", suffix).into());
        }
        if unit_name.is_none() {
            return Err(format!("failed to load related unit {} unit name is none", suffix).into());
        }
        let u_name = unit_name.unwrap();
        let stem_name = Path::new(&u_name).file_stem().unwrap().to_str().unwrap();
        let relate_name = format!("{}.{}", stem_name, suffix);
        self.config.set_unit_ref(relate_name);
        Ok(())
    }

    pub(self) fn add_default_dependencies(&self) -> Result<()> {
        let u = match self.comm.owner() {
            None => {
                return Ok(());
            }
            Some(v) => v,
        };

        if !u.default_dependencies() {
            return Ok(());
        }

        log::debug!("Adding default dependencies for timer: {}", u.id());
        let um = self.comm.um();
        um.unit_add_dependency(
            &u.id(),
            UnitRelations::UnitAfter,
            TIMERS_TARGET,
            true,
            UnitDependencyMask::Default,
        )?;
        um.unit_add_two_dependency(
            &u.id(),
            UnitRelations::UnitAfter,
            UnitRelations::UnitRequires,
            SYSINIT_TARGET,
            true,
            UnitDependencyMask::Default,
        )?;
        um.unit_add_two_dependency(
            &u.id(),
            UnitRelations::UnitBefore,
            UnitRelations::UnitConflicts,
            SHUTDOWN_TARGET,
            true,
            UnitDependencyMask::Default,
        )?;
        Ok(())
    }
}
