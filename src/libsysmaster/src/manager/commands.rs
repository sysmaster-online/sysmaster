use super::manager::Manager;
use super::rentry::ReliLastFrame;
use crate::proto::ProstServerStream;
use crate::reliability::Reliability;
use libevent::{EventType, Events, Source};
use libutils::{Error, Result};
use std::net::{SocketAddr, TcpListener};
use std::os::unix::io::RawFd;
use std::{os::unix::prelude::AsRawFd, rc::Rc};

pub(super) struct Commands {
    // associated objects
    reli: Rc<Reliability>,
    manager: Rc<Manager>,

    // owned objects
    fd: TcpListener,
}

impl Commands {
    pub(super) fn new(relir: &Rc<Reliability>, mr: &Rc<Manager>) -> Commands {
        let addrs = [
            SocketAddr::from(([127, 0, 0, 1], 9526)),
            SocketAddr::from(([127, 0, 0, 1], 9527)),
        ];
        let fd = TcpListener::bind(&addrs[..]).unwrap();
        Commands {
            reli: Rc::clone(relir),
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

        self.reli.set_last_frame1(ReliLastFrame::CmdOp as u32);
        match self.fd.incoming().next() {
            None => println!("None CommandRequest!"),
            Some(stream) => {
                println!("{:?}", stream);
                let dispatch = ProstServerStream::new(stream.unwrap(), self.manager.clone());
                dispatch.process().unwrap();
            }
        }
        self.reli.clear_last_frame();

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
