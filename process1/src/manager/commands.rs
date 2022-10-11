use super::manager::Manager;
use crate::proto::ProstServerStream;
use event::{EventType, Events, Source};
use std::os::unix::io::RawFd;
use std::{os::unix::prelude::AsRawFd, rc::Rc};
use utils::{Error, Result};

pub(super) struct Commands {
    manager: Rc<Manager>,
    fd: std::net::TcpListener,
}

impl Commands {
    pub(super) fn new(mr: &Rc<Manager>) -> Commands {
        let fd = std::net::TcpListener::bind("127.0.0.1:9527").unwrap();
        Commands {
            manager: Rc::clone(mr),
            fd,
        }
    }
}

impl Source for Commands {
    fn event_type(&self) -> EventType {
        EventType::Io
    }

    fn epoll_event(&self) -> u32 {
        (libc::EPOLLIN) as u32
    }

    fn dispatch(&self, _e: &Events) -> Result<i32, Error> {
        println!("Dispatching Command!");
        match self.fd.incoming().next() {
            None => println!("None CommandRequest!"),
            Some(stream) => {
                println!("{:?}", stream);
                let dispatch = ProstServerStream::new(stream.unwrap(), self.manager.clone());
                dispatch.process().unwrap();
            }
        }

        Ok(0)
    }

    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }

    fn fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }
}
