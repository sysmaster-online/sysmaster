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
use crate::framework::job_queue::JobQueue;
use device::device_monitor::{DeviceMonitor, MonitorNetlinkGroup};
use event::{EventType, Events, Source};
use std::os::unix::io::RawFd;
use std::rc::Rc;

/// uevent monitor
pub struct UeventMonitor {
    /// receive uevent from netlink socket
    device_monitor: DeviceMonitor,

    /// insert uevents to job queue
    job_queue: Rc<JobQueue>,
}

/// public methods
impl UeventMonitor {
    /// create a monitor instance for monitoring uevent from kernel
    pub fn new(job_queue: Rc<JobQueue>) -> UeventMonitor {
        UeventMonitor {
            device_monitor: DeviceMonitor::new(MonitorNetlinkGroup::Kernel, None),
            job_queue,
        }
    }

    /// forcely set the size of socket receive buffer
    pub fn set_receive_buffer(&self, v: usize) {
        basic::socket::set_receive_buffer(self.device_monitor.fd(), v).unwrap();
    }
}

impl Source for UeventMonitor {
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
            Ok(ret) => match ret {
                Some(device) => device,
                None => return 0,
            },
            Err(e) => {
                log::error!("Monitor Error: {}", e);
                return 0;
            }
        };

        /* The devpath is guaranteed to be valid. */
        log::debug!("Monitor: received device {}", device.get_devpath().unwrap());

        self.job_queue.job_queue_insert(device);
        0
    }

    /// token of event source
    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }
}
