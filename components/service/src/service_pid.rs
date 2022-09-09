use super::service_comm::ServiceComm;
use nix::unistd::Pid;
use process1::manager::UnitActionError;
use std::cell::RefCell;
use std::rc::Rc;
use utils::process_util;

pub(super) struct ServicePid {
    comm: Rc<ServiceComm>,
    data: RefCell<ServicePidData>,
}

impl ServicePid {
    pub(super) fn new(commr: &Rc<ServiceComm>) -> ServicePid {
        ServicePid {
            comm: Rc::clone(commr),
            data: RefCell::new(ServicePidData::new()),
        }
    }

    pub(super) fn set_main(&self, pid: Pid) {
        self.data.borrow_mut().set_main(pid)
    }

    pub(super) fn reset_main(&self) {
        self.data.borrow_mut().reset_main()
    }

    pub(super) fn unwatch_main(&self) {
        match self.main() {
            Some(pid) => {
                self.comm.um().child_unwatch_pid(pid);
                self.data.borrow_mut().reset_main();
            }
            None => {}
        }
    }

    pub(super) fn set_control(&self, pid: Pid) {
        self.data.borrow_mut().set_control(pid)
    }

    pub(super) fn reset_control(&self) {
        self.data.borrow_mut().reset_control()
    }

    pub(super) fn unwatch_control(&self) {
        match self.control() {
            Some(pid) => {
                self.comm.um().child_unwatch_pid(pid);
                self.data.borrow_mut().reset_control();
            }
            None => {}
        }
    }

    pub(super) fn main(&self) -> Option<Pid> {
        self.data.borrow().main()
    }

    pub(super) fn control(&self) -> Option<Pid> {
        self.data.borrow().control()
    }

    pub(super) fn main_alive(&self) -> Result<bool, UnitActionError> {
        self.data.borrow().main_alive()
    }
}

struct ServicePidData {
    main: Option<Pid>,
    control: Option<Pid>,
}

// the declaration "pub(self)" is for identification only.
impl ServicePidData {
    pub(self) fn new() -> ServicePidData {
        ServicePidData {
            main: None,
            control: None,
        }
    }

    pub(self) fn set_main(&mut self, pid: Pid) {
        self.main = Some(pid);
    }

    pub(self) fn reset_main(&mut self) {
        self.main = None;
    }

    pub(self) fn set_control(&mut self, pid: Pid) {
        self.control = Some(pid);
    }

    pub(self) fn reset_control(&mut self) {
        self.control = None;
    }

    pub(self) fn main(&self) -> Option<Pid> {
        self.main.as_ref().cloned()
    }

    pub(self) fn control(&self) -> Option<Pid> {
        self.control.as_ref().cloned()
    }

    pub(self) fn main_alive(&self) -> Result<bool, UnitActionError> {
        if self.main.is_none() {
            return Err(UnitActionError::UnitActionEAgain);
        }

        Ok(process_util::alive(self.main.unwrap()))
    }
}
