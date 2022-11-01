use super::manager::Manager;
use super::rentry::ReliLastFrame;
use crate::reliability::Reliability;
use libevent::{EventType, Events, Source};
use libutils::Result;
use nix::sys::signal::Signal;
use std::{convert::TryFrom, rc::Rc};

pub(super) struct Signals {
    // associated objects
    reli: Rc<Reliability>,
    manager: Rc<Manager>,
}

impl Signals {
    pub(super) fn new(relir: &Rc<Reliability>, mr: &Rc<Manager>) -> Signals {
        Signals {
            reli: Rc::clone(relir),
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

    fn dispatch(&self, e: &Events) -> Result<i32> {
        log::debug!("Dispatching signals!");

        self.reli.set_last_frame1(ReliLastFrame::ManagerOp as u32);
        match e.read_signals() {
            Ok(Some(info)) => {
                let signal = Signal::try_from(info.si_signo).unwrap();
                log::debug!("read signal from event: {}", signal);
                if let Err(e) = self.manager.dispatch_signal(&signal) {
                    log::error!("dispatch signal{:?} error: {}", signal, e);
                }
            }
            Ok(None) => log::debug!("read signals none"),
            Err(e) => log::debug!("read signals error, {:?}", e),
        }
        self.reli.clear_last_frame();

        Ok(0)
    }

    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }
}
