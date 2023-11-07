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
use basic::murmurhash2::murmurhash2;
use nix::{
    errno::Errno,
    sys::socket::{
        recv, sendmsg, AddressFamily, MsgFlags, NetlinkAddr, SockFlag, SockProtocol, SockType,
    },
};
use std::{io::IoSlice, mem::size_of, os::unix::prelude::RawFd};

use crate::{device::Device, error::Error};

const UDEV_MONITOR_MAGIC: u32 = 0xfeedcafe;

/// Compatible with 'string_hash32' in libsystemd
fn string_hash32(s: &str) -> u32 {
    murmurhash2(s.as_bytes(), s.len(), 0)
}

/// Compatible with 'string_bloom64' in libsystemd
fn string_bloom64(s: &str) -> u64 {
    let mut bits: u64 = 0;
    let hash: u32 = string_hash32(s);

    bits |= 1_u64 << (hash & 63);
    bits |= 1_u64 << ((hash >> 6) & 63);
    bits |= 1_u64 << ((hash >> 12) & 63);
    bits |= 1_u64 << ((hash >> 18) & 63);

    bits
}

/// netlink group of device monitor
pub enum MonitorNetlinkGroup {
    /// none group
    None,
    /// monitoring kernel message
    Kernel,
    /// monitoring userspace message
    Userspace,
}

#[repr(C)]
struct MonitorNetlinkHeader {
    prefix: [u8; 8],
    magic: u32,
    header_size: u32,
    properties_off: u32,
    properties_len: u32,
    filter_subsystem_hash: u32,
    filter_devtype_hash: u32,
    filter_tag_bloom_hi: u32,
    filter_tag_bloom_lo: u32,
}

impl Default for MonitorNetlinkHeader {
    fn default() -> Self {
        Self {
            prefix: *b"libudev\0",
            magic: UDEV_MONITOR_MAGIC.to_be(),
            header_size: size_of::<MonitorNetlinkHeader>() as u32,
            properties_off: size_of::<MonitorNetlinkHeader>() as u32,
            properties_len: 0,
            filter_subsystem_hash: 0,
            filter_devtype_hash: 0,
            filter_tag_bloom_hi: 0,
            filter_tag_bloom_lo: 0,
        }
    }
}

impl MonitorNetlinkHeader {
    fn to_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self as *const _ as *const u8,
                size_of::<MonitorNetlinkHeader>(),
            )
        }
    }
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

        let prefix = String::from_utf8(buf[..prefix_split_idx].to_vec()).unwrap();

        if prefix.contains("@/") {
            return Device::from_nulstr(&buf[prefix_split_idx + 1..n]);
        } else if prefix == "libudev" {
            return Device::from_nulstr(&buf[40..n]);
        }

        Err(Error::Nix {
            msg: format!("invalid nulstr data ({:?})", buf),
            source: Errno::EINVAL,
        })
    }

    /// send device
    pub fn send_device(
        &self,
        device: &Device,
        destination: Option<NetlinkAddr>,
    ) -> Result<(), Error> {
        let mut header = MonitorNetlinkHeader::default();

        let dest = match destination {
            Some(addr) => addr,
            None => NetlinkAddr::new(0, 2),
        };

        let (properties, len) = device.get_properties_nulstr()?;

        header.properties_len = len as u32;
        header.filter_subsystem_hash = string_hash32(device.get_subsystem()?.as_str()).to_be();
        if let Ok(devtype) = device.get_devtype() {
            header.filter_devtype_hash = string_hash32(&devtype).to_be();
        }

        let mut tag_bloom_bits: u64 = 0;
        for tag in &device.tag_iter() {
            tag_bloom_bits |= string_bloom64(tag.as_str());
        }

        if tag_bloom_bits > 0 {
            header.filter_tag_bloom_hi = ((tag_bloom_bits >> 32) as u32).to_be();
            header.filter_tag_bloom_lo = ((tag_bloom_bits & 0xffffffff) as u32).to_be();
        }

        let iov = [IoSlice::new(header.to_bytes()), IoSlice::new(&properties)];

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
            println!("{:?}", device);
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
            let device = Device::from_devname("/dev/sda").unwrap();
            device.set_sysattr_value("uevent", Some("change")).unwrap();
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
            let device = Device::from_devname("/dev/sda").unwrap();
            device.set_action_from_string("change").unwrap();
            device.set_subsystem("block");
            device.set_seqnum(1000);

            let broadcaster = DeviceMonitor::new(MonitorNetlinkGroup::None, None);
            broadcaster.send_device(&device, None).unwrap();
        })
        .join()
        .unwrap();

        e.rloop().unwrap();

        e.del_source(s.clone()).unwrap();
    }
}
