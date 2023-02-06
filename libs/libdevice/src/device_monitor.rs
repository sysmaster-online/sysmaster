//! device monitor
//!
use nix::sys::socket::{
    recv, sendmsg, AddressFamily, MsgFlags, NetlinkAddr, SockFlag, SockProtocol, SockType,
};
use std::io::IoSlice;

use crate::Device;

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
    socket: i32,
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
    pub fn receive_device(&self) -> Option<Device> {
        let mut buf = vec![0; 1024 * 8];
        let n = recv(self.socket, &mut buf, MsgFlags::empty()).unwrap();
        let mut prefix_split_idx: usize = 0;

        for (idx, val) in buf.iter().enumerate() {
            if *val == 0 {
                prefix_split_idx = idx;
                break;
            }
        }

        let prefix = std::str::from_utf8(&buf[..prefix_split_idx]).unwrap();

        if prefix.contains("@/") {
            return Some(Device::from_buffer(&buf[prefix_split_idx + 1..n]));
        } else if prefix == "libdevm" {
            return Some(Device::from_buffer(&buf[40..n]));
        }

        None
    }

    /// send device
    pub fn send_device(&self, device: &Device, destination: Option<NetlinkAddr>) {
        let dest = match destination {
            Some(addr) => addr,
            None => NetlinkAddr::new(0, 2),
        };

        let properties_nulstr_len = device.properties_nulstr_len.to_be_bytes();
        let iov = [
            IoSlice::new(b"libdevm\0"),
            IoSlice::new(&[254, 237, 190, 239]),
            IoSlice::new(&[40, 0, 0, 0]),
            IoSlice::new(&[40, 0, 0, 0]),
            IoSlice::new(&properties_nulstr_len[0..4]),
            // todo: supply subsystem hash
            IoSlice::new(&[0, 0, 0, 0]),
            // todo: supply devtype hash
            IoSlice::new(&[0, 0, 0, 0]),
            // todo: supply tag bloom high and low bytes
            IoSlice::new(&[0, 0, 0, 0]),
            IoSlice::new(&[0, 0, 0, 0]),
            IoSlice::new(&device.properties_nulstr),
        ];

        sendmsg(self.fd(), &iov, &[], MsgFlags::empty(), Some(&dest)).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use std::{os::unix::prelude::RawFd, rc::Rc};

    use super::*;
    use libevent::*;

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
        fn dispatch(&self, e: &Events) -> Result<i32, libevent::Error> {
            let device = self.device_monitor.receive_device().unwrap();
            println!("{device:?}");
            e.set_exit();
            Ok(0)
        }

        ///
        fn token(&self) -> u64 {
            let data: u64 = unsafe { std::mem::transmute(self) };
            data
        }
    }

    /// require implementation of `devctl trigger`, ignore this test case temporarily
    #[ignore]
    #[test]
    fn test_monitor_kernel() {
        let e = Events::new().unwrap();
        let s: Rc<dyn Source> = Rc::new(Monitor {
            device_monitor: DeviceMonitor::new(MonitorNetlinkGroup::Kernel, None),
        });
        e.add_source(s.clone()).unwrap();

        e.set_enabled(s.clone(), EventState::On).unwrap();

        e.rloop().unwrap();

        e.del_source(s.clone()).unwrap();
    }

    /// require implementation of `devctl trigger`, ignore this test case temporarily
    #[ignore]
    #[test]
    fn test_monitor_userspace() {
        let e = Events::new().unwrap();
        let s: Rc<dyn Source> = Rc::new(Monitor {
            device_monitor: DeviceMonitor::new(MonitorNetlinkGroup::Userspace, None),
        });
        e.add_source(s.clone()).unwrap();

        e.set_enabled(s.clone(), EventState::On).unwrap();

        e.rloop().unwrap();

        e.del_source(s.clone()).unwrap();
    }
}
