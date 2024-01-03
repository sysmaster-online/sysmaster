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

use crate::Result;
use basic::socket::set_receive_buffer;
use device::{device_monitor::DeviceMonitor, device_monitor::MonitorNetlinkGroup};
use event::{EventState, EventType, Events, Source};
use nix::errno::Errno;
use nix::sys::signal::Signal;
use std::collections::{HashMap, HashSet};
use std::{os::unix::prelude::RawFd, rc::Rc};

/// wrapper of DeviceMonitor
struct DevctlMonitorX {
    /// device monitor
    device_monitor: DeviceMonitor,

    /// prefix in log
    prefix: String,

    show_property: bool,
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
            Ok(ret) => match ret {
                Some(device) => device,
                None => return 0,
            },
            Err(e) => match e {
                device::error::Error::Nix {
                    msg: _,
                    source: Errno::EAGAIN,
                } => {
                    return 0;
                }
                device::error::Error::Nix { msg: _, source: _ } => {
                    log::error!("{}", e);
                    return 0;
                }
                _ => {
                    return 0;
                }
            },
        };

        let ts = nix::time::clock_gettime(nix::time::ClockId::CLOCK_MONOTONIC).unwrap();
        println!(
            "{} [{}] {} {} ({})",
            self.prefix,
            ts.to_string(),
            device.get_action().unwrap(),
            device.get_devpath().unwrap(),
            device.get_subsystem().unwrap()
        );
        if self.show_property {
            for properties in &device.property_iter() {
                println!("{}={}", properties.0, properties.1);
            }
            println!();
        }
        0
    }

    /// source token
    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }
}

struct DevctlMonitorSignal {}

impl DevctlMonitorSignal {
    fn new() -> DevctlMonitorSignal {
        DevctlMonitorSignal {}
    }
}

impl Source for DevctlMonitorSignal {
    /// monitor socket fd
    fn fd(&self) -> RawFd {
        0
    }

    /// The signal type needs to specify the signal to listen to
    fn signals(&self) -> Vec<Signal> {
        vec![Signal::SIGINT, Signal::SIGTERM]
    }

    /// event type
    fn event_type(&self) -> EventType {
        EventType::Signal
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
    fn dispatch(&self, event: &Events) -> i32 {
        event.set_exit();
        0
    }

    /// source token
    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }
}

pub struct MonitorArgs {
    property: bool,
    environment: bool,
    kernel: bool,
    userspace: bool,
    subsystem_match: Option<Vec<String>>,
    tag_match: Option<Vec<String>>,
}

impl MonitorArgs {
    pub fn new(
        property: bool,
        environment: bool,
        kernel: bool,
        userspace: bool,
        subsystem_match: Option<Vec<String>>,
        tag_match: Option<Vec<String>>,
    ) -> Self {
        MonitorArgs {
            property,
            environment,
            kernel,
            userspace,
            subsystem_match,
            tag_match,
        }
    }

    /// subcommand for monitoring device messages from kernel and userspace
    pub fn subcommand(&self) -> Result<()> {
        let events = Events::new().unwrap();

        let mut print_kernel = false;
        let mut print_userspace = false;

        if self.kernel {
            print_kernel = true;
        } else if self.userspace {
            print_userspace = true;
        } else {
            print_kernel = true;
            print_userspace = true;
        }

        let mut subsystem_filter: HashMap<String, String> = HashMap::new();
        if let Some(subsystem_devtypes) = &self.subsystem_match {
            for subsystem_devtype in subsystem_devtypes {
                if let Some(pos) = subsystem_devtype.find('/') {
                    let devtype = subsystem_devtype[pos + 1..].to_string();
                    let subsystem = subsystem_devtype[..pos].to_string();
                    subsystem_filter.insert(subsystem, devtype);
                } else {
                    subsystem_filter.insert(subsystem_devtype.to_string(), "".to_string());
                }
            }
        }

        let mut tag_filter: HashSet<String> = HashSet::new();
        if let Some(tags) = &self.tag_match {
            for tag in tags {
                tag_filter.insert(tag.to_string());
            }
        }

        let signal = Rc::new(DevctlMonitorSignal::new());
        events.add_source(signal.clone()).unwrap();
        events.set_enabled(signal, EventState::OneShot).unwrap();

        println!("monitor will print the received events for:");

        if print_kernel {
            self.setup_monitor(
                MonitorNetlinkGroup::Kernel,
                "KERNEL".to_string(),
                &events,
                subsystem_filter.clone(),
                tag_filter.clone(),
            )?;
            println!("KERNEL - the kernel uevent");
        }
        if print_userspace {
            self.setup_monitor(
                MonitorNetlinkGroup::Userspace,
                "USERSPACE".to_string(),
                &events,
                subsystem_filter,
                tag_filter,
            )?;
            println!("USERSPACE - broadcasted by devmaster after successful process on device");
        }
        println!();

        events.rloop().unwrap();
        Ok(())
    }

    fn setup_monitor(
        &self,
        sender: MonitorNetlinkGroup,
        prefix: String,
        events: &Events,
        subsystem_filter: HashMap<String, String>,
        tag_filter: HashSet<String>,
    ) -> Result<()> {
        let mut device_monitor = DeviceMonitor::new(sender, None);
        for (subsystem, devtype) in &subsystem_filter {
            if let Err(err) = device_monitor.filter_add_match_subsystem_devtype(subsystem, devtype)
            {
                log::error!(
                    "Failed to apply subsystem filter subsystem:{:?} devtype:{:?}",
                    subsystem,
                    devtype
                );
                return Err(err.get_errno());
            }
        }

        for tag in &tag_filter {
            if let Err(err) = device_monitor.filter_add_match_tag(tag) {
                log::error!("Failed to apply tag filter {:?}", tag);
                return Err(err.get_errno());
            }
        }

        let monitor = Rc::new(DevctlMonitorX {
            device_monitor,
            prefix,
            show_property: self.property || self.environment,
        });
        if let Err(err) = set_receive_buffer(monitor.fd(), 1024 * 1024 * 128) {
            log::error!("Failed to set receive buffer forcely ({:?})", err);
            return Err(nix::errno::from_i32(err.get_errno()));
        }

        events.add_source(monitor.clone()).unwrap();
        events.set_enabled(monitor, EventState::On).unwrap();

        Ok(())
    }
}
