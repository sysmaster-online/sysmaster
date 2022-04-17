use std::{os::unix::prelude::AsRawFd, rc::Rc};

use event::{EventType, Events, Source};
use utils::{Error, Result};

use std::os::unix::io::RawFd;

use crate::proto::ProstServerStream;

use super::manager::Manager;

pub struct Commands {
    manager: Rc<Manager>,
    fd: std::net::TcpListener,
}

impl Commands {
    pub fn new(m: Rc<Manager>) -> Commands {
        let fd = std::net::TcpListener::bind("127.0.0.1:9527").unwrap();
        Commands { manager: m, fd }
    }

    pub fn handle(&self) {}
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
        match self.fd.incoming().nth(0) {
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
