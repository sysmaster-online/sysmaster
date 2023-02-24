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

//! uevent_monitor
//!
use libdevice::device_monitor::{DeviceMonitor, MonitorNetlinkGroup};
use libevent::{EventType, Events, Source};
use nix::errno::Errno;
use std::os::unix::io::RawFd;
use std::rc::Rc;

use crate::job_queue::JobQueue;

/// uevent monitor
#[derive(Debug)]
pub struct Monitor {
    /// receive uevent from netlink socket
    device_monitor: DeviceMonitor,

    /// insert uevents to job queue
    job_queue: Rc<JobQueue>,
}

/// public methods
impl Monitor {
    /// create a monitor instance for monitoring uevent from kernel
    pub fn new(job_queue: Rc<JobQueue>) -> Monitor {
        Monitor {
            device_monitor: DeviceMonitor::new(MonitorNetlinkGroup::Kernel, None),
            job_queue,
        }
    }

    /// forcely set the size of socket receive buffer
    pub fn set_receive_buffer_force(&self, v: usize) {
        libutils::socket_util::set_receive_buffer_force(self.device_monitor.fd(), v).unwrap();
    }
}

impl Source for Monitor {
    /// socket fd
    fn fd(&self) -> RawFd {
        self.device_monitor.fd()
    }

    /// event type
    fn event_type(&self) -> EventType {
        EventType::Io
    }

    /// epoll type
    fn epoll_event(&self) -> u32 {
        (libc::EPOLLIN) as u32
    }

    /// priority of event source
    fn priority(&self) -> i8 {
        0i8
    }

    /// receive device from socket and insert into job queue
    fn dispatch(&self, _: &Events) -> i32 {
        let device = match self.device_monitor.receive_device() {
            Ok(ret) => ret,
            Err(e) => match e {
                libdevice::error::Error::Syscall {
                    syscall: _,
                    errno: Errno::EAGAIN,
                } => {
                    return 0;
                }
                libdevice::error::Error::Syscall {
                    syscall: _,
                    errno: _,
                } => {
                    log::error!("{}", e);
                    return 0;
                }
                _ => {
                    return 0;
                }
            },
        };

        log::debug!("Monitor: received device {}", device.devpath);

        self.job_queue.job_queue_insert(device);
        self.job_queue.job_queue_start();
        0
    }

    /// token of event source
    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }
}
