use std::{cell::RefCell, convert::TryFrom, rc::Rc};

use event::{EventType, Events, Source};
use nix::{sys::signal::Signal, unistd::Pid};

use super::manager::Manager;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ProcessExit {
    Status(Pid, i32, nix::sys::signal::Signal),
}
pub struct Signals {
    manager: Rc<RefCell<Manager>>,
}

impl Signals {
    pub fn new(m: Rc<RefCell<Manager>>) -> Signals {
        Signals { manager: m.clone() }
    }
}

impl Source for Signals {
    fn event_type(&self) -> EventType {
        EventType::Signal
    }

    fn signals(&self) -> Vec<libc::c_int> {
        vec![libc::SIGCHLD, libc::SIGTERM, libc::SIGINT]
    }

    fn epoll_event(&self) -> u32 {
        (libc::EPOLLIN) as u32
    }

    fn priority(&self) -> i8 {
        0i8
    }

    fn dispatch(&self, e: &mut Events) {
        println!("Dispatching signal!");
        loop {
            match e.read_signals() {
                Ok(Some(info)) => {
                    let signal = Signal::try_from(info.si_signo).unwrap();
                    match signal {
                        Signal::SIGCHLD => {
                            let mut m = self.manager.borrow_mut();
                            match m.dispatch_sigchld() {
                                Err(e) => {
                                    log::error!("dispatch sigchld error: {}", e)
                                }
                                Ok(_) => break,
                            }
                        }
                        Signal::SIGHUP => todo!(),
                        Signal::SIGINT => todo!(),

                        Signal::SIGKILL => todo!(),
                        Signal::SIGUSR1 => todo!(),
                        Signal::SIGUSR2 => todo!(),
                        _ => todo!(),
                    }
                    println!("read signo: {:?}", info.si_signo);
                    break;
                }
                Ok(None) => break,
                Err(e) => {
                    println!("{:?}", e);
                    break;
                }
            }
        }
    }

    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }

    fn fd(&self) -> std::os::unix::prelude::RawFd {
        todo!()
    }

    fn pid(&self) -> libc::pid_t {
        0
    }
}
