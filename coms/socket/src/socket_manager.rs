use super::socket_base::{LOG_LEVEL, PLUGIN_NAME};
use super::socket_comm::SocketUmComm;
use super::socket_rentry::SocketReFrame;
use libsysmaster::manager::{UnitManager, UnitManagerObj, UnitMngUtil};
use libsysmaster::{ReStation, Reliability};
use libutils::logger;
use std::rc::Rc;
use std::sync::Arc;

struct SocketManager {
    comm: Arc<SocketUmComm>,
}

// the declaration "pub(self)" is for identification only.
impl SocketManager {
    pub(self) fn new() -> SocketManager {
        let _comm = SocketUmComm::get_instance();
        SocketManager {
            comm: Arc::clone(&_comm),
        }
    }
}

impl UnitManagerObj for SocketManager {
    // nothing to customize
}

impl ReStation for SocketManager {
    // input: do nothing

    // compensate
    fn db_compensate_last(&self, _lframe: (u32, Option<u32>, Option<u32>), lunit: Option<&String>) {
        assert!(lunit.is_some());

        let frame = self.comm.rentry().last_frame();
        if frame.is_none() {
            // debug
            return;
        }

        let unit_id = lunit.unwrap();
        match frame.unwrap() {
            SocketReFrame::FdListen(spread) => self.rc_last_fdlisten(unit_id, spread),
        }
    }

    fn do_compensate_last(&self, _lframe: (u32, Option<u32>, Option<u32>), lunit: Option<&String>) {
        assert!(lunit.is_some());

        let frame = self.comm.rentry().last_frame();
        if frame.is_none() {
            // debug
            return;
        }

        let unit_id = lunit.unwrap();
        match frame.unwrap() {
            SocketReFrame::FdListen(spread) => self.dc_last_fdlisten(unit_id, spread),
        }
    }

    // no data

    // reload: no external connections, no entry
}

impl SocketManager {
    fn rc_last_fdlisten(&self, lunit: &String, spread: bool) {
        match spread {
            true => self.comm.um().rentry_trigger_merge(lunit, true), // merge to trigger
            false => {}                                               // do nothing, try again
        }
    }

    fn dc_last_fdlisten(&self, lunit: &str, spread: bool) {
        match spread {
            true => self.comm.um().trigger_unit(lunit), // re-run
            false => {}                                 // do nothing, try again
        }
    }
}

impl UnitMngUtil for SocketManager {
    fn attach_um(&self, um: Rc<UnitManager>) {
        self.comm.attach_um(um);
    }

    fn attach_reli(&self, reli: Rc<Reliability>) {
        self.comm.attach_reli(reli);
    }
}

impl Default for SocketManager {
    fn default() -> Self {
        SocketManager::new()
    }
}

use libsysmaster::declure_umobj_plugin;
declure_umobj_plugin!(
    SocketManager,
    SocketManager::default,
    PLUGIN_NAME,
    LOG_LEVEL
);
