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
use crate::error::*;
use crate::framework::job_queue::JobQueue;
use crate::framework::worker_manager::WorkerManager;
use device::device::Device;
use event::Source;
use snafu::ResultExt;
use std::time::SystemTime;
use std::{
    cell::RefCell,
    io::Read,
    net::TcpListener,
    os::unix::prelude::{AsRawFd, RawFd},
    rc::Rc,
};

/// listening address for control manager
pub const CONTROL_MANAGER_LISTEN_ADDR: &str = "0.0.0.0:1224";

/// control manager
#[derive(Debug)]
pub struct ControlManager {
    /// listener for devctl messages
    listener: RefCell<TcpListener>,

    /// reference to worker manager
    worker_manager: Rc<WorkerManager>,
    /// reference to job queue
    job_queue: Rc<JobQueue>,
    // events: Rc<Events>,
}

/// public methods
impl ControlManager {
    /// create a control manager instance
    pub fn new(
        listen_addr: String,
        worker_manager: Rc<WorkerManager>,
        job_queue: Rc<JobQueue>,
    ) -> ControlManager {
        ControlManager {
            listener: RefCell::new(TcpListener::bind(listen_addr).unwrap()),
            worker_manager,
            job_queue,
        }
    }
}

/// internal methods
impl ControlManager {
    /// process command from devctl
    pub(crate) fn cmd_process(&self, cmd: String) {
        let tokens: Vec<&str> = cmd.split(' ').collect();

        let (cmd_kind, devname) = (tokens[0], tokens[1]);

        match cmd_kind {
            "test" => {
                let seqnum = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    % 1000;

                let device = Device::new();
                let _ = device
                    .set_devname(devname)
                    .context(DeviceSnafu)
                    .log_error("failed to set devname");
                let _ = device
                    .set_seqnum(seqnum)
                    .context(DeviceSnafu)
                    .log_error("failed to set devname");

                self.job_queue.job_queue_insert(device);
                self.job_queue.job_queue_start();
            }
            "kill" => {
                self.worker_manager.clone().start_kill_workers_timer();
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
        -50
    }

    /// start dispatching after the event arrives
    fn dispatch(&self, _: &event::Events) -> i32 {
        let (mut stream, _) = self.listener.borrow_mut().accept().unwrap();
        let mut cmd = String::new();
        stream.read_to_string(&mut cmd).unwrap();

        log::debug!("Control Manager: received message \"{cmd}\"");

        self.cmd_process(cmd);

        0
    }

    /// Unless you can guarantee all types of token allocation, it is recommended to use the default implementation here
    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }
}
