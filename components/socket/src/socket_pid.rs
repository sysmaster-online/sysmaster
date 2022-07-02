//! socket_pid模块实现子进程pid的管理。
//!

use super::socket_comm::SocketComm;
use nix::unistd::Pid;
use std::cell::RefCell;
use std::rc::Rc;

pub(super) struct SocketPid {
    comm: Rc<SocketComm>,
    data: RefCell<SocketPidData>,
}

impl SocketPid {
    pub(super) fn new(comm: &Rc<SocketComm>) -> SocketPid {
        SocketPid {
            comm: comm.clone(),
            data: RefCell::new(SocketPidData::new()),
        }
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

    pub(super) fn control(&self) -> Option<Pid> {
        self.data.borrow().control()
    }

    pub(super) fn set_control(&self, pid: Pid) {
        self.data.borrow_mut().set_control(pid)
    }
}

struct SocketPidData {
    control: Option<Pid>,
}

impl SocketPidData {
    pub(self) fn new() -> Self {
        SocketPidData { control: None }
    }

    pub(self) fn control(&self) -> Option<Pid> {
        self.control.as_ref().cloned()
    }

    pub(self) fn reset_control(&mut self) {
        self.control = None;
    }

    pub(self) fn set_control(&mut self, pid: Pid) {
        self.control = Some(pid);
    }
}

#[cfg(test)]
mod tests {
    use super::SocketPid;
    use crate::socket_comm::SocketComm;
    use nix::unistd::Pid;
    use std::rc::Rc;

    #[test]
    fn test_socket_pid() {
        let _comm = Rc::new(SocketComm::new());
        let pid = Rc::new(SocketPid::new(&_comm));

        pid.set_control(Pid::from_raw(5));
        assert_eq!(pid.control().unwrap(), Pid::from_raw(5));
    }
}
