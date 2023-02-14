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

//! subcommand for devctl monitor
//!
use libdevice::{device_monitor::DeviceMonitor, device_monitor::MonitorNetlinkGroup};
use libevent::{EventState, EventType, Events, Source};
use libutils::socket_util::set_receive_buffer_force;
use nix::errno::Errno;
use std::{os::unix::prelude::RawFd, rc::Rc};

/// wrapper of DeviceMonitor
struct DevctlMonitorX {
    /// device monitor
    device_monitor: DeviceMonitor,

    /// prefix in log
    prefix: String,
}

impl Source for DevctlMonitorX {
    /// monitor socket fd
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

    /// event source priority
    fn priority(&self) -> i8 {
        0i8
    }

    /// print device messages from kernel and userspace
    fn dispatch(&self, _e: &Events) -> i32 {
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

        println!(
            "{} >> {} {} ({})",
            self.prefix,
            device.action.unwrap(),
            device.devpath,
            device.subsystem
        );
        0
    }

    /// source token
    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }
}

/// subcommand for monitoring device messages from kernel and userspace
pub fn subcommand_monitor() {
    println!(
        "start monitoring device events:
KERNEL - the kernel uevent
USERSPACE - broadcasted by devmaster after successful process on device
",
    );

    let kernel_monitor = Rc::new(DevctlMonitorX {
        device_monitor: DeviceMonitor::new(MonitorNetlinkGroup::Kernel, None),
        prefix: "KERNEL []".to_string(),
    });

    let userspace_monitor = Rc::new(DevctlMonitorX {
        device_monitor: DeviceMonitor::new(MonitorNetlinkGroup::Userspace, None),
        prefix: "USERSPACE []".to_string(),
    });

    if let Err(errno) = set_receive_buffer_force(kernel_monitor.fd(), 1024 * 1024 * 128) {
        log::error!("Failed to set receive buffer forcely ({errno:?})");
    }

    if let Err(errno) = set_receive_buffer_force(userspace_monitor.fd(), 1024 * 1024 * 128) {
        log::error!("Failed to set receive buffer forcely ({errno:?})");
    }

    let events = Events::new().unwrap();

    events.add_source(kernel_monitor.clone()).unwrap();
    events.add_source(userspace_monitor.clone()).unwrap();
    events.set_enabled(kernel_monitor, EventState::On).unwrap();
    events
        .set_enabled(userspace_monitor, EventState::On)
        .unwrap();

    events.rloop().unwrap();
}
