use super::target_base::{LOG_LEVEL, PLUGIN_NAME};
use super::target_comm::TargetUmComm;
use libsysmaster::manager::{UnitManager, UnitManagerObj, UnitMngUtil};
use libsysmaster::{ReStation, Reliability};
use libutils::logger;
use std::rc::Rc;
use std::sync::Arc;

struct TargetManager {
    comm: Arc<TargetUmComm>,
}

// the declaration "pub(self)" is for identification only.
impl TargetManager {
    pub(self) fn new() -> TargetManager {
        let _comm = TargetUmComm::get_instance();
        TargetManager {
            comm: Arc::clone(&_comm),
        }
    }
}

impl UnitManagerObj for TargetManager {
    // nothing to customize
}

impl ReStation for TargetManager {
    // no input, no compensate

    // no data

    // reload: no external connections, no entry
}

impl UnitMngUtil for TargetManager {
    fn attach_um(&self, um: Rc<UnitManager>) {
        self.comm.attach_um(um);
    }

    fn attach_reli(&self, reli: Rc<Reliability>) {
        self.comm.attach_reli(reli);
    }
}

impl Default for TargetManager {
    fn default() -> Self {
        TargetManager::new()
    }
}

use libsysmaster::declure_umobj_plugin;
declure_umobj_plugin!(
    TargetManager,
    TargetManager::default,
    PLUGIN_NAME,
    LOG_LEVEL
);
