//! TargetUnit is the entrance of the sub unit，implement the trait UnitObj,UnitMngUtil and UnitSubClass.
//! Trait UnitObj defines the behavior of the sub unit.
//! Trait UnitMngUtil is used to attach the Unitmanager to the sub unit.
//! Trait UnitSubClass implement the convert from sub unit to UnitObj.

use std::{path::PathBuf, rc::Rc};

use super::target_comm::TargetComm;
use super::target_mng::TargetMng;
use process1::manager::{
    UnitActiveState, UnitDependencyMask, UnitMngUtil, UnitObj, UnitRelationAtom, UnitRelations,
    UnitSubClass,
};
use utils::logger;

struct Target {
    comm: Rc<TargetComm>,
    mng: Rc<TargetMng>,
}

impl Target {
    fn new() -> Target {
        let _comm = Rc::new(TargetComm::new());
        Target {
            comm: Rc::clone(&_comm),
            mng: Rc::new(TargetMng::new(&_comm)),
        }
    }

    pub(self) fn add_default_dependencies(&self) {
        if self.comm.um().is_none() {
            return;
        }

        if let Some(unit) = self.comm.unit() {
            log::debug!("add default dependencies for target[{}]", unit.get_id());
            if !unit.default_dependencies() {
                return;
            }
            let um = self.comm.um().unwrap();
            let deps = um.get_dependency_list(
                unit.get_id(),
                UnitRelationAtom::UnitAtomAddDefaultTargetDependencyQueue,
            );
            for _u in deps {
                if !_u.default_dependencies() {
                    continue;
                }

                if um.unit_has_dependecy(
                    unit.get_id(),
                    UnitRelationAtom::UnitAtomBefore,
                    _u.get_id(),
                ) {
                    continue;
                }

                let e = um.unit_add_dependency(
                    unit.get_id(),
                    UnitRelations::UnitAfter,
                    _u.get_id(),
                    true,
                    UnitDependencyMask::UnitDependencyDefault,
                );
                if e.is_err() {
                    log::error!("add default dependencies error {:?}", e);
                    return;
                }
            }
        }
    }
}

impl UnitObj for Target {
    fn load(&self, _conf_str: &Vec<PathBuf>) -> utils::Result<(), Box<dyn std::error::Error>> {
        //todo add default dependency funnction need add
        log::debug!("load for target");
        self.add_default_dependencies();
        Ok(())
    }
    fn current_active_state(&self) -> UnitActiveState {
        self.mng.to_unit_state()
    }

    fn attach_unit(&self, unit: Rc<process1::manager::Unit>) {
        self.comm.attach_unit(unit);
    }

    fn init(&self) {}

    fn done(&self) {}

    fn coldplug(&self) {}

    fn dump(&self) {}

    fn start(&self) -> utils::Result<(), process1::manager::UnitActionError> {
        //if current state is not valid, just return.
        self.mng.start_check()?;

        self.mng.start_action();
        Ok(())
    }

    fn stop(&self) -> utils::Result<(), process1::manager::UnitActionError> {
        self.mng.stop_check()?;

        self.mng.stop_action();
        Ok(())
    }

    fn reload(&self) {}

    fn kill(&self) {}

    fn release_resources(&self) {}

    fn sigchld_events(
        &self,
        _pid: nix::unistd::Pid,
        _code: i32,
        _status: nix::sys::signal::Signal,
    ) {
    }

    fn reset_failed(&self) {}
}

impl UnitSubClass for Target {
    fn into_unitobj(self: Box<Self>) -> Box<dyn UnitObj> {
        Box::new(*self)
    }
}

impl UnitMngUtil for Target {
    fn attach(&self, _um: Rc<process1::manager::UnitManager>) {
        self.comm.attach_um(_um);
    }
}

impl Default for Target {
    fn default() -> Self {
        Target::new()
    }
}

const LOG_LEVEL: u32 = 4;
const PLUGIN_NAME: &str = "TargetUnit";
use process1::declure_unitobj_plugin;
declure_unitobj_plugin!(Target, Target::default, PLUGIN_NAME, LOG_LEVEL);

#[cfg(test)]
mod tests {
    #[test]
    fn test() {}
}
