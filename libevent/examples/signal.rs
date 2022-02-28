use libc::getpid;
use libc::kill;

use std::{cell::RefCell, rc::Rc};

use event::EventType;
use event::Events;
use event::Source;

#[derive(Debug)]
struct Signals {}

impl Signals {
    fn new() -> Signals {
        Signals {}
    }
}

impl Source for Signals {
    fn event_type(&self) -> EventType {
        EventType::Signal
    }

    fn signals(&self) -> Vec<libc::c_int> {
        vec![libc::SIGCHLD, libc::SIGTERM]
    }

    fn epoll_event(&self) -> u32 {
        (libc::EPOLLIN | libc::EPOLLONESHOT) as u32
    }

    fn priority(&self) -> i8 {
        0i8
    }

    fn dispatch(&self, _: &mut Events) {
        println!("Dispatching signal!");
    }

    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }
}

fn main() {
    let mut e = Events::new().unwrap();
    let s: Rc<RefCell<dyn Source>> = Rc::new(RefCell::new(Signals::new()));
    e.add_source(s.clone());
    unsafe {
        kill(getpid(), libc::SIGTERM);
    }
    e.rloop();
}
