//! uevent_monitor
//!
use crate::JobQueue;
use kobject_uevent::UEvent;
use libdevice::*;
use libevent::*;
use netlink_sys::{protocols::NETLINK_KOBJECT_UEVENT, Socket, SocketAddr};
use std::rc::Rc;
use std::{os::unix::io::RawFd, os::unix::prelude::AsRawFd, process};

/// uevent monitor
#[derive(Debug)]
pub struct Monitor {
    /// receive uevent from netlink socket
    socket: Socket,

    /// insert uevents to job queue
    job_queue: Rc<JobQueue>,
}

impl Monitor {
    ///
    pub fn new(job_queue: Rc<JobQueue>) -> Monitor {
        let mut socket = Socket::new(NETLINK_KOBJECT_UEVENT).unwrap();
        let sa = SocketAddr::new(process::id(), 1);
        socket.bind(&sa).unwrap();
        Monitor { socket, job_queue }
    }

    /// forcely set the size of socket receive buffer
    pub fn set_receive_buffer_force(&self, v: usize) {
        libutils::socket_util::set_receive_buffer_force(self.socket.as_raw_fd(), v).unwrap();
    }
}

impl Source for Monitor {
    ///
    fn fd(&self) -> RawFd {
        self.socket.as_raw_fd()
    }

    ///
    fn event_type(&self) -> EventType {
        EventType::Io
    }

    ///
    fn epoll_event(&self) -> u32 {
        (libc::EPOLLIN) as u32
    }

    ///
    fn priority(&self) -> i8 {
        0i8
    }

    ///
    fn dispatch(&self, _: &Events) -> Result<i32, libevent::Error> {
        // println!("Start dispatch");
        let mut buf = vec![0; 1024 * 8];
        let n = self.socket.recv(&mut &mut buf[..], 0).unwrap();
        // let s = std::str::from_utf8(&buf[..n]).unwrap();
        let u = UEvent::from_netlink_packet(&buf[..n]).unwrap();
        let device = Device::from_uevent(u);
        // println!(">> {}", s);
        // println!("===\n{:#?}\n===", device);
        self.job_queue.job_queue_insert(device);
        self.job_queue.job_queue_start();
        Ok(0)
    }

    ///
    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }
}
