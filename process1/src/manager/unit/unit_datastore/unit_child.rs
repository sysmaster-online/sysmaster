use super::unit_sets::UnitSets;
use crate::manager::signals::ProcessExit;
use crate::manager::unit::unit_entry::UnitX;
use nix::sys::signal::Signal;
use nix::sys::wait::{WaitPidFlag, WaitStatus};
use nix::unistd::Pid;
use std::cell::RefCell;
use std::collections::HashMap;
use std::error::Error;
use std::rc::Rc;

#[derive(Debug)]
pub(super) struct UnitChild {
    data: RefCell<UnitChildData>,
}

impl UnitChild {
    pub(super) fn new(units: Rc<UnitSets>) -> UnitChild {
        UnitChild {
            data: RefCell::new(UnitChildData::new(units)),
        }
    }

    pub(super) fn dispatch_sigchld(&self) -> Result<(), Box<dyn Error>> {
        self.data.borrow_mut().dispatch_sigchld()
    }

    pub(super) fn add_watch_pid(&self, pid: Pid, id: &str) {
        self.data.borrow_mut().add_watch_pid(pid, id)
    }

    pub(super) fn unwatch_pid(&self, pid: Pid) {
        self.data.borrow_mut().unwatch_pid(pid)
    }
}

#[derive(Debug)]
struct UnitChildData {
    units: Rc<UnitSets>,
    watch_pids: HashMap<Pid, Rc<UnitX>>, // key: pid, value: unit
}

// the declaration "pub(self)" is for identification only.
impl UnitChildData {
    pub(self) fn new(units: Rc<UnitSets>) -> UnitChildData {
        UnitChildData {
            units,
            watch_pids: HashMap::new(),
        }
    }

    pub(self) fn dispatch_sigchld(&mut self) -> Result<(), Box<dyn Error>> {
        log::debug!("Dispatching sighandler waiting for pid");
        let wait_pid = Pid::from_raw(-1);
        let flags = WaitPidFlag::WNOHANG;
        let process_exit = {
            match nix::sys::wait::waitpid(wait_pid, Some(flags)) {
                Ok(wait_status) => match wait_status {
                    WaitStatus::Exited(pid, code) => {
                        ProcessExit::Status(pid, code, Signal::SIGCHLD)
                    }
                    WaitStatus::Signaled(pid, signal, _dumped_core) => {
                        ProcessExit::Status(pid, -1, signal)
                    }
                    _ => {
                        log::debug!("Ignored child signal: {:?}", wait_status);
                        return Err(format!("Ignored child signal: {:?}", wait_status).into());
                    }
                },
                Err(e) => {
                    log::error!("Error while waiting pid: {}", e);
                    return Err(format!("Error while waiting pid: {}", e).into());
                }
            }
        };

        match process_exit {
            ProcessExit::Status(pid, code, signal) => {
                match self.watch_pids.get(&pid) {
                    Some(unit) => {
                        unit.sigchld_events(pid, code, signal);
                    }
                    None => {
                        log::debug!("not found unit obj of pid: {:?}", pid);
                        return Err(format!("not found unit obj of pid: {:?}", pid).into());
                    }
                }

                self.watch_pids.remove(&pid);
                Ok(())
            }
        }
    }

    pub(self) fn add_watch_pid(&mut self, pid: Pid, id: &str) {
        let unit = self.units.get(id).unwrap();
        self.watch_pids.insert(pid, unit);
    }

    pub(self) fn unwatch_pid(&mut self, pid: Pid) {
        self.watch_pids.remove(&pid);
    }
}
