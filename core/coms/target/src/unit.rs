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

//! TargetUnit is used grouping units and as a synchronization points during startup
//! TargetUnit is the entrance of the sub unit，implement the trait UnitObj,UnitMngUtil and UnitSubClass.
//! Trait UnitObj defines the behavior of the sub unit.
//! Trait UnitMngUtil is used to attach the Unitmanager to the sub unit.
//! Trait UnitSubClass implement the convert from sub unit to UnitObj.
use super::comm::TargetUnitComm;
use super::mng::TargetMng;
use core::error::*;
use core::rel::{ReStation, Reliability};
use core::unit::UnitBase;
use core::unit::{
    SubUnit, UmIf, UnitActiveState, UnitDependencyMask, UnitMngUtil, UnitRelationAtom,
    UnitRelations,
};
use nix::sys::wait::WaitStatus;
use std::{path::PathBuf, rc::Rc};

struct Target {
    um: Rc<dyn UmIf>,
    comm: Rc<TargetUnitComm>,
    mng: Rc<TargetMng>,
}

impl ReStation for Target {
    // no input, no compensate

    // data
    fn db_map(&self, reload: bool) {
        self.mng.db_map(reload);
    }

    fn db_insert(&self) {
        self.mng.db_insert();
    }

    // reload: entry-only
    fn entry_coldplug(&self) {
        // rebuild external connections, like: timer, ...
        // do nothing now
    }

    fn entry_clear(&self) {
        // release external connection, like: timer, ...
        // do nothing now
    }
}

impl Target {
    fn new(um_if: Rc<dyn UmIf>) -> Target {
        let _comm = Rc::new(TargetUnitComm::new());
        Target {
            um: Rc::clone(&um_if),
            comm: Rc::clone(&_comm),
            mng: Rc::new(TargetMng::new(&_comm)),
        }
    }

    pub(self) fn owner(&self) -> Option<Rc<dyn UnitBase>> {
        if let Some(ref unit) = self.comm.owner() {
            Some(Rc::clone(unit))
        } else {
            None
        }
    }

    pub(self) fn add_default_dependencies(&self) {
        let u = match self.owner() {
            None => return,
            Some(u) => u,
        };

        if !u.default_dependencies() {
            return;
        }

        log::debug!("Adding default dependencies for target: {}", u.id());
        let um = self.um.clone();
        let deps = um.get_dependency_list(
            &u.id(),
            UnitRelationAtom::UnitAtomAddDefaultTargetDependencyQueue,
        );
        for other in deps {
            /* If the other unit has configured DefaultDependencies=false,
             * don't add default dependencies automatically. */
            if !um.unit_has_default_dependecy(&other) {
                continue;
            }

            /* Don't create loop, as we will add UnitAfter later. */
            if um.unit_has_dependency(&u.id(), UnitRelationAtom::UnitAtomBefore, &other) {
                continue;
            }

            if let Err(e) = um.unit_add_dependency(
                &u.id(),
                UnitRelations::UnitAfter,
                &other,
                true,
                UnitDependencyMask::Default,
            ) {
                log::error!("Failed to add default dependencies for {}: {:?}", u.id(), e);
                return;
            }
        }
    }
}

impl SubUnit for Target {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn load(&self, _conf_str: Vec<PathBuf>) -> Result<()> {
        //todo add default dependency funnction need add
        log::debug!("load for target");
        self.add_default_dependencies();
        Ok(())
    }

    fn current_active_state(&self) -> UnitActiveState {
        self.mng.to_unit_state()
    }

    fn get_subunit_state(&self) -> String {
        self.mng.get_state()
    }

    fn attach_unit(&self, unit: Rc<dyn UnitBase>) {
        self.comm.attach_unit(Rc::clone(&unit));
        self.db_insert();
    }

    fn init(&self) {}

    fn done(&self) {}

    fn dump(&self) {}

    fn start(&self) -> Result<()> {
        log::info!("Starting {}", self.comm.owner().unwrap().id());
        //if current state is not valid, just return.
        self.mng.start_check()?;

        self.mng.start_action(true);
        Ok(())
    }

    fn stop(&self, force: bool) -> Result<()> {
        if !force {
            self.mng.stop_check()?;
        }

        self.mng.stop_action(true);
        Ok(())
    }

    fn kill(&self) {}

    fn release_resources(&self) {}

    fn sigchld_events(&self, _wait_status: WaitStatus) {}

    fn reset_failed(&self) {}
}

impl UnitMngUtil for Target {
    fn attach_um(&self, _um: Rc<dyn UmIf>) {
        self.comm.attach_um(_um);
    }

    fn attach_reli(&self, reli: Rc<Reliability>) {
        self.comm.attach_reli(reli);
    }
}

/*impl Default for Target {
    fn default() -> Self {
        Target::new()
    }
}*/

use core::declare_unitobj_plugin_with_param;
declare_unitobj_plugin_with_param!(Target, Target::new);
