use super::service_base::{LOG_LEVEL, PLUGIN_NAME};
use super::service_comm::ServiceUmComm;
use process1::manager::{UnitManager, UnitManagerObj, UnitMngUtil};
use process1::{ReStation, Reliability};
use std::rc::Rc;
use std::sync::Arc;
use utils::logger;

struct ServiceManager {
    comm: Arc<ServiceUmComm>,
}

// the declaration "pub(self)" is for identification only.
impl ServiceManager {
    pub(self) fn new() -> ServiceManager {
        let _comm = ServiceUmComm::get_instance();
        ServiceManager {
            comm: Arc::clone(&_comm),
        }
    }
}

impl UnitManagerObj for ServiceManager {
    // nothing to customize
}

impl ReStation for ServiceManager {
    // no input, no compensate

    // no data

    // reload: no external connections, no entry
}

impl UnitMngUtil for ServiceManager {
    fn attach_um(&self, um: Rc<UnitManager>) {
        self.comm.attach_um(um);
    }

    fn attach_reli(&self, reli: Rc<Reliability>) {
        self.comm.attach_reli(reli);
    }
}

impl Default for ServiceManager {
    fn default() -> Self {
        ServiceManager::new()
    }
}

use process1::declure_umobj_plugin;
declure_umobj_plugin!(
    ServiceManager,
    ServiceManager::default,
    PLUGIN_NAME,
    LOG_LEVEL
);
