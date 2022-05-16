use super::manager::Manager;
use event::{EventType, Events, Source};
use nix::{sys::signal::Signal, unistd::Pid};
use std::{convert::TryFrom, rc::Rc};
use utils::{Error, Result};

#[derive(Debug, PartialEq, Eq, Clone)]
pub(super) enum ProcessExit {
    Status(Pid, i32, nix::sys::signal::Signal),
}
pub(super) struct Signals {
    manager: Rc<Manager>,
}

impl Signals {
    pub(super) fn new(mr: &Rc<Manager>) -> Signals {
        Signals {
            manager: Rc::clone(mr),
        }
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

    fn dispatch(&self, e: &Events) -> Result<i32, Error> {
        log::debug!("Dispatching signals!");

        #[allow(clippy::never_loop)]
        loop {
            match e.read_signals() {
                Ok(Some(info)) => {
                    let signal = Signal::try_from(info.si_signo).unwrap();
                    log::debug!("read signal from event: {}", signal);
                    match signal {
                        Signal::SIGCHLD => match self.manager.dispatch_sigchld() {
                            Err(e) => {
                                log::error!("dispatch sigchld error: {}", e)
                            }
                            Ok(_) => break,
                        },
                        Signal::SIGHUP => todo!(),
                        Signal::SIGINT => todo!(),

                        Signal::SIGKILL => todo!(),
                        Signal::SIGUSR1 => todo!(),
                        Signal::SIGUSR2 => todo!(),
                        _ => todo!(),
                    }
                    break;
                }
                Ok(None) => {
                    log::debug!("read signals none");
                    break;
                }
                Err(e) => {
                    log::debug!("read signals error");
                    println!("{:?}", e);
                    break;
                }
            }
        }
        Ok(0)
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
