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

//! device monitor
//!
use nix::{
    errno::Errno,
    sys::socket::{
        recv, sendmsg, AddressFamily, MsgFlags, NetlinkAddr, SockFlag, SockProtocol, SockType,
    },
};
use std::{io::IoSlice, os::unix::prelude::RawFd};

use crate::{device::Device, error::Error};

/// netlink group of device monitor
pub enum MonitorNetlinkGroup {
    /// none group
    None,
    /// monitoring kernel message
    Kernel,
    /// monitoring userspace message
    Userspace,
}

/// device monitor
#[derive(Debug)]
pub struct DeviceMonitor {
    /// socket fd
    socket: RawFd,
    /// socket address, currently only support netlink
    _sockaddr: NetlinkAddr,
}

impl DeviceMonitor {
    /// if fd is none, create a new socket
    pub fn new(group: MonitorNetlinkGroup, fd: Option<i32>) -> DeviceMonitor {
        let sock = match fd {
            Some(i) => i,
            None => nix::sys::socket::socket(
                AddressFamily::Netlink,
                SockType::Raw,
                SockFlag::SOCK_CLOEXEC | SockFlag::SOCK_NONBLOCK,
                SockProtocol::NetlinkKObjectUEvent,
            )
            .unwrap(),
        };

        let sa = NetlinkAddr::new(0, group as u32);
        nix::sys::socket::bind(sock, &sa).unwrap();

        DeviceMonitor {
            socket: sock,
            _sockaddr: sa,
        }
    }

    /// return socket fd
    pub fn fd(&self) -> i32 {
        self.socket
    }

    /// receive device
    pub fn receive_device(&self) -> Result<Device, Error> {
        let mut buf = vec![0; 1024 * 8];
        let n = match recv(self.socket, &mut buf, MsgFlags::empty()) {
            Ok(ret) => ret,
            Err(errno) => {
                return Err(Error::Nix {
                    msg: "syscall recv failed".to_string(),
                    source: errno,
                })
            }
        };
        let mut prefix_split_idx: usize = 0;

        for (idx, val) in buf.iter().enumerate() {
            if *val == 0 {
                prefix_split_idx = idx;
                break;
            }
        }

        let prefix = std::str::from_utf8(&buf[..prefix_split_idx]).unwrap();

        if prefix.contains("@/") {
            return Device::from_nulstr(&buf[prefix_split_idx + 1..n]);
        } else if prefix == "libdevm" {
            return Device::from_nulstr(&buf[40..n]);
        } else if prefix == "libudev" {
            return Err(Error::Nix {
                msg: "origin from udev".to_string(),
                source: Errno::EINVAL,
            });
        }

        Err(Error::Nix {
            msg: format!("invalid nulstr data ({:?})", buf),
            source: Errno::EINVAL,
        })
    }

    /// send device
    pub fn send_device(
        &self,
        device: &mut Device,
        destination: Option<NetlinkAddr>,
    ) -> Result<(), Error> {
        let dest = match destination {
            Some(addr) => addr,
            None => NetlinkAddr::new(0, 2),
        };

        let (nulstr, len) = device.get_properties_nulstr()?;

        let len_bytes = len.to_be_bytes();
        let iov = [
            IoSlice::new(b"libdevm\0"),
            IoSlice::new(&[254, 237, 190, 239]),
            IoSlice::new(&[40, 0, 0, 0]),
            IoSlice::new(&[40, 0, 0, 0]),
            IoSlice::new(&len_bytes[0..4]),
            // todo: supply subsystem hash
            IoSlice::new(&[0, 0, 0, 0]),
            // todo: supply devtype hash
            IoSlice::new(&[0, 0, 0, 0]),
            // todo: supply tag bloom high and low bytes
            IoSlice::new(&[0, 0, 0, 0]),
            IoSlice::new(&[0, 0, 0, 0]),
            IoSlice::new(nulstr),
        ];

        sendmsg(self.fd(), &iov, &[], MsgFlags::empty(), Some(&dest)).unwrap();

        Ok(())
    }
}

impl Drop for DeviceMonitor {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            let _ = libc::close(self.socket);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::*;
    use event::*;
    use std::{os::unix::prelude::RawFd, rc::Rc, thread::spawn};

    /// wrapper of DeviceMonitor
    struct Monitor {
        /// device monitor
        device_monitor: DeviceMonitor,
    }

    impl Source for Monitor {
        ///
        fn fd(&self) -> RawFd {
            self.device_monitor.fd()
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
        fn dispatch(&self, e: &Events) -> i32 {
            let device = self.device_monitor.receive_device().unwrap();
            println!("{device:?}");
            e.set_exit();
            0
        }

        ///
        fn token(&self) -> u64 {
            let data: u64 = unsafe { std::mem::transmute(self) };
            data
        }
    }

    /// test whether device monitor can receive uevent from kernel normally
    #[ignore]
    #[test]
    fn test_monitor_kernel() {
        let e = Events::new().unwrap();
        let s: Rc<dyn Source> = Rc::new(Monitor {
            device_monitor: DeviceMonitor::new(MonitorNetlinkGroup::Kernel, None),
        });
        e.add_source(s.clone()).unwrap();
        e.set_enabled(s.clone(), EventState::On).unwrap();

        spawn(|| {
            let mut device = Device::from_devname("/dev/sda".to_string()).unwrap();
            device
                .set_sysattr_value("uevent".to_string(), Some("change".to_string()))
                .unwrap();
        })
        .join()
        .unwrap();

        e.rloop().unwrap();

        e.del_source(s.clone()).unwrap();
    }

    /// test whether device monitor can receive device message from userspace normally
    #[ignore]
    #[test]
    fn test_monitor_userspace() {
        let e = Events::new().unwrap();
        let s: Rc<dyn Source> = Rc::new(Monitor {
            device_monitor: DeviceMonitor::new(MonitorNetlinkGroup::Userspace, None),
        });
        e.add_source(s.clone()).unwrap();
        e.set_enabled(s.clone(), EventState::On).unwrap();

        spawn(|| {
            let mut device = Device::from_devname("/dev/sda".to_string()).unwrap();
            let broadcaster = DeviceMonitor::new(MonitorNetlinkGroup::None, None);
            broadcaster.send_device(&mut device, None).unwrap();
        })
        .join()
        .unwrap();

        e.rloop().unwrap();

        e.del_source(s.clone()).unwrap();
    }
}
