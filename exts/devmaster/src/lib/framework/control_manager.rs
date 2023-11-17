// Copyright (c) 2022 Huawei Technologies Co.,Ltd. All rights reserved.
//
// sysMaster is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan
// PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY
// KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

//! control manager
//!
use crate::framework::job_queue::JobQueue;
use crate::framework::worker_manager::WorkerManager;
use event::{Events, Source};
use nix::unistd::unlink;
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::rc::Weak;
use std::{
    cell::RefCell,
    io::Read,
    os::unix::prelude::{AsRawFd, RawFd},
    rc::Rc,
};

/// listening address for control manager
pub const CONTROL_MANAGER_LISTEN_ADDR: &str = "/run/devmaster/control";

/// control manager
pub struct ControlManager {
    /// listener for devctl messages
    listener: RefCell<UnixListener>,

    /// reference to worker manager
    worker_manager: Weak<WorkerManager>,
    /// reference to job queue
    _job_queue: Weak<JobQueue>,
    events: Rc<Events>,
}

/// public methods
impl ControlManager {
    /// create a control manager instance
    pub fn new(
        listen_addr: String,
        worker_manager: Rc<WorkerManager>,
        job_queue: Rc<JobQueue>,
        events: Rc<Events>,
    ) -> ControlManager {
        /*
         * Cleanup remaining socket if it exists.
         */
        if Path::new(listen_addr.as_str()).exists() {
            let _ = unlink(listen_addr.as_str());
        }

        let listener = RefCell::new(UnixListener::bind(listen_addr.as_str()).unwrap_or_else(
            |error| {
                log::error!("Control Manager: failed to bind listener \"{}\"", error);
                panic!();
            },
        ));

        ControlManager {
            listener,
            worker_manager: Rc::downgrade(&worker_manager),
            _job_queue: Rc::downgrade(&job_queue),
            events,
        }
    }
}

/// internal methods
impl ControlManager {
    /// process command from devctl
    pub(crate) fn cmd_process(&self, cmd: String) {
        let tokens: Vec<&str> = cmd.split(' ').collect();

        let (cmd_kind, _devname) = (tokens[0], tokens[1]);

        match cmd_kind {
            "kill" => {
                self.worker_manager.upgrade().unwrap().kill_workers();
            }
            "exit" => {
                self.events.set_exit();
            }
            "ping" => {
                log::debug!("Received devmaster control message (PING)");
            }
            _ => {
                todo!();
            }
        }
    }
}

impl Source for ControlManager {
    /// tcp listener fd
    fn fd(&self) -> RawFd {
        self.listener.borrow().as_raw_fd()
    }

    /// event type
    fn event_type(&self) -> event::EventType {
        event::EventType::Io
    }

    /// epoll type
    fn epoll_event(&self) -> u32 {
        (libc::EPOLLIN) as u32
    }

    /// Set the priority, -127i8 ~ 128i8, the smaller the value, the higher the priority
    fn priority(&self) -> i8 {
        100
    }

    /// start dispatching after the event arrives
    fn dispatch(&self, _: &event::Events) -> i32 {
        let (mut stream, _) = self.listener.borrow_mut().accept().unwrap();
        let mut cmd = String::new();
        stream.read_to_string(&mut cmd).unwrap();

        log::debug!("Control Manager: received message \"{}\"", cmd);

        self.cmd_process(cmd);

        0
    }

    /// Unless you can guarantee all types of token allocation, it is recommended to use the default implementation here
    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }
}
