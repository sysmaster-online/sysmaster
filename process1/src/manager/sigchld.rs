
use std::cell::RefCell;
use std::rc::Rc;
use nix::libc;
use nix::sys::signal::Signal;
use nix::sys::wait::WaitPidFlag;
use nix::sys::wait::WaitStatus;
use nix::unistd::Pid;
use event::EventType;
use event::Events;
use event::Source;

use super::unit::UnitManager;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ProcessExit {
    Status(Pid, i32, nix::sys::signal::Signal),
}

#[derive(Debug)]
struct SignalChld {
    manager: Rc<RefCell<UnitManager>>,
}

impl SignalChld {
    pub fn new(manager: Rc<RefCell<UnitManager>>) -> SignalChld {
        SignalChld {
            manager,
        }
    }
}

impl Source for SignalChld {
    fn event_type(&self) -> EventType {
        EventType::Signal
    }

    fn signals(&self) -> Vec<libc::c_int> {
        vec![libc::SIGCHLD]
    }

    fn epoll_event(&self) -> u32 {
        libc::EPOLLIN as u32
    }

    fn priority(&self) -> i8 {
        0i8
    }

    fn dispatch(&self, _: &mut Events) {
        log::debug!("Dispatching sighandler waiting for pid");
        let wait_pid = Pid::from_raw(-1);
        let flags = WaitPidFlag::WNOHANG;
        let process_exit = {
            match nix::sys::wait::waitpid(wait_pid, Some(flags)) {
                Ok(wait_status) => match wait_status {
                    WaitStatus::Exited(pid, code) => {
                        Some(ProcessExit::Status(pid, code, Signal::SIGCHLD))
                    }
                    WaitStatus::Signaled(pid, signal, _dumped_core) => {
                        Some(ProcessExit::Status(pid, -1, signal))
                    }
                    _ => {
                        log::debug!("Ignored child signal: {:?}", wait_status);
                        None
                    }
                },
                Err(e) => {
                    log::error!("Error while waiting pid: {}", e);
                    None
                }
            }
        };

        match process_exit {
            Some(exit) => {
                let mut manager = self.manager.borrow_mut();
                manager.dispatch_sigchld(exit);
            },
            None => log::debug!("not waited a exit child process")
        }
    }

    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }
}

#[cfg(test)]
mod tests {
    use std::{rc::Rc, cell::RefCell};
    use utils::logger::init_log_with_console;

    use super::*;

    #[test]
    fn  test_sigchld_handler(){

        init_log_with_console("sigchld_handlers",3);
        let mut unit_manager = UnitManager::new();
        unit_manager.init_lookup_path();

        let unit_name = String::from("config.service");
        unit_manager.load_unit(&unit_name);
        match unit_manager.get_unit_on_name(&unit_name) {
            Some(unit_obj) => {
                let mut unit = unit_obj.borrow_mut();
                unit.start(&mut unit_manager);
            },
            None => println!("not fount unit: {}", unit_name),
        };

        
        let mut e = Events::new().unwrap();
        let manager  = Rc::new(RefCell::new(unit_manager));
        let s: Rc<RefCell<dyn Source>> = Rc::new(RefCell::new(SignalChld::new(manager.clone())));
        e.add_source(s.clone());

        e.rloop();
    }
}