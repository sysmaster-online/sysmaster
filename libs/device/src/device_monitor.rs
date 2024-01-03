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
use crate::{device::Device, error::Error};
use basic::errno_is_transient;
use basic::murmurhash2::murmurhash2;
use basic::socket::next_datagram_size_fd;
use libc::*;
use nix::{
    errno::Errno,
    sys::socket::{
        recv, sendmsg, AddressFamily, MsgFlags, NetlinkAddr, SockFlag, SockProtocol, SockType,
    },
};
use std::collections::{HashMap, HashSet};
use std::{io::IoSlice, mem::size_of, os::unix::prelude::RawFd};

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

#[inline]
/// Generate a BPF instruction
fn bpf_inst(ins: &mut Vec<sock_filter>, code: u32, jt: u8, jf: u8, k: u32) {
    let inst = sock_filter {
        code: code as u16,
        jt: jt as u8,
        jf: jf as u8,
        k: k as u32,
    };
    ins.push(inst);
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
    sockaddr: NetlinkAddr,

    /// key:subsystem value:devtype
    subsystem_filter: HashMap<String, String>,
    tag_filter: HashSet<String>,
    match_sysattr_filter: HashMap<String, String>,
    nomatch_sysattr_filter: HashMap<String, String>,
    match_parent_filter: HashSet<String>,
    nomatch_parent_filter: HashSet<String>,
    filter_uptodate: bool,
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
            sockaddr: sa,
            subsystem_filter: HashMap::new(),
            tag_filter: HashSet::new(),
            match_sysattr_filter: HashMap::new(),
            nomatch_sysattr_filter: HashMap::new(),
            match_parent_filter: HashSet::new(),
            nomatch_parent_filter: HashSet::new(),
            filter_uptodate: false,
        }
    }

    /// return socket fd
    pub fn fd(&self) -> i32 {
        self.socket
    }

    /// receive device
    pub fn receive_device(&self) -> Result<Option<Device>, Error> {
        let n = match next_datagram_size_fd(self.socket) {
            Ok(n) => n,
            Err(err) => {
                let e = Errno::from_i32(err.get_errno());
                if !errno_is_transient(e) {
                    log::error!("Failed to get the received message size err:{:?}", err);
                }
                return Err(Error::Nix {
                    msg: "".to_string(),
                    source: e,
                });
            }
        };

        let mut buf = vec![0; n];
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

        let device: Device;
        if prefix.contains("@/") {
            device = match Device::from_nulstr(&buf[prefix_split_idx + 1..n]) {
                Ok(device) => device,
                Err(err) => return Err(err),
            };
        } else if prefix == "libudev" {
            device = match Device::from_nulstr(&buf[40..n]) {
                Ok(device) => device,
                Err(err) => return Err(err),
            };
        } else {
            return Err(Error::Nix {
                msg: format!("invalid nulstr data ({:?})", buf),
                source: Errno::EINVAL,
            });
        }

        /* Skip device, if it does not pass the current filter */
        match self.passes_filter(&device) {
            Ok(flag) => {
                if !flag {
                    log::trace!("Received device does not pass filter, ignoring.");
                    Ok(None)
                } else {
                    Ok(Some(device))
                }
            }
            Err(err) => {
                log::error!(
                    "Failed to check received device passing filter err:{:?}",
                    err
                );
                Err(err)
            }
        }
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
            header.filter_tag_bloom_lo = ((tag_bloom_bits & 0xffffffff_u64) as u32).to_be();
        }

        let iov = [IoSlice::new(header.to_bytes()), IoSlice::new(&properties)];

        sendmsg(self.fd(), &iov, &[], MsgFlags::empty(), Some(&dest)).unwrap();

        Ok(())
    }

    /// add subsystem and devtype match
    pub fn filter_add_match_subsystem_devtype(
        &mut self,
        subsystem: &str,
        devtype: &str,
    ) -> Result<(), Error> {
        if subsystem.is_empty() {
            return Err(Error::Nix {
                msg: "subsystem is empty".to_string(),
                source: Errno::EINVAL,
            });
        }

        self.subsystem_filter
            .insert(subsystem.to_string(), devtype.to_string());
        self.filter_uptodate = false;

        Ok(())
    }

    /// add tag match
    pub fn filter_add_match_tag(&mut self, tag: &str) -> Result<(), Error> {
        if tag.is_empty() {
            return Err(Error::Nix {
                msg: "tag is empty".to_string(),
                source: Errno::EINVAL,
            });
        }

        self.tag_filter.insert(tag.to_string());
        self.filter_uptodate = false;

        Ok(())
    }

    fn passes_filter(&self, device: &Device) -> Result<bool, Error> {
        match self.check_subsystem_filter(device) {
            Ok(flag) => {
                if !flag {
                    return Ok(false);
                }
            }
            Err(err) => return Err(err),
        }

        if !self.check_tag_filter(device) {
            return Ok(false);
        }

        if !device.match_sysattr(&self.match_sysattr_filter, &self.nomatch_sysattr_filter) {
            return Ok(false);
        }

        Ok(device.match_parent(&self.match_parent_filter, &self.nomatch_parent_filter))
    }

    fn check_subsystem_filter(&self, device: &Device) -> Result<bool, Error> {
        if self.subsystem_filter.is_empty() {
            return Ok(true);
        }

        let subsystem = match device.get_subsystem() {
            Ok(subsystem) => subsystem,
            Err(err) => return Err(err),
        };

        let devtype = match device.get_devtype() {
            Ok(devtype) => devtype,
            Err(err) => {
                if err.get_errno() != nix::Error::ENOENT {
                    return Err(err);
                } else {
                    String::from("")
                }
            }
        };

        for (key, value) in &self.subsystem_filter {
            if key != &subsystem {
                continue;
            }
            if value.is_empty() || value == &devtype {
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn check_tag_filter(&self, device: &Device) -> bool {
        if self.tag_filter.is_empty() {
            return true;
        }
        for tag in &self.tag_filter {
            if let Ok(true) = device.has_tag(tag) {
                return true;
            }
        }
        false
    }

    /// Call this method to let the socket filter make sense
    /// whenever filter conditions are updated.
    pub fn bpf_filter_update(&mut self) -> Result<(), Error> {
        if self.filter_uptodate {
            return Ok(());
        }

        /* No need to filter uevents from kernel. */
        if self.sockaddr.groups() == MonitorNetlinkGroup::Kernel as u32
            || (self.subsystem_filter.is_empty() && self.tag_filter.is_empty())
        {
            self.filter_uptodate = true;
            return Ok(());
        }

        let mut ins: Vec<sock_filter> = Vec::new();

        /* Load magic sense code, the offset of magic is 8 bytes */
        bpf_inst(&mut ins, BPF_LD | BPF_W | BPF_ABS, 0, 0, 8);
        /* Jump 1 step if magic matches */
        bpf_inst(
            &mut ins,
            BPF_JMP | BPF_JEQ | BPF_K,
            1,
            0,
            UDEV_MONITOR_MAGIC,
        );
        /* Illegal magic, pass the packet */
        bpf_inst(&mut ins, BPF_RET | BPF_K, 0, 0, 0xffffffff);

        if !self.tag_filter.is_empty() {
            let mut tag_n = self.tag_filter.len();

            for tag in self.tag_filter.iter() {
                let tag_bloom_bits = string_bloom64(tag);
                let hi = (tag_bloom_bits >> 32) as u32;
                let lo = (tag_bloom_bits & 0xffffffff_u64) as u32;
                /* Load tag high bloom bits */
                bpf_inst(&mut ins, BPF_LD | BPF_W | BPF_ABS, 0, 0, 32);
                /* Bits and */
                bpf_inst(&mut ins, BPF_ALU | BPF_AND | BPF_K, 0, 0, hi);
                /* Skip 3 steps to continue matching the next tag */
                bpf_inst(&mut ins, BPF_JMP | BPF_JEQ | BPF_K, 0, 3, hi);

                /* Load tag low bloom bits */
                bpf_inst(&mut ins, BPF_LD | BPF_W | BPF_ABS, 0, 0, 36);
                /* Bits and */
                bpf_inst(&mut ins, BPF_ALU | BPF_AND | BPF_K, 0, 0, lo);

                tag_n -= 1;
                /* Skip 3 steps to continue matching the next tag */
                bpf_inst(
                    &mut ins,
                    BPF_JMP | BPF_JEQ | BPF_K,
                    (1 + (tag_n * 6)) as u8,
                    0,
                    lo,
                );
            }

            /* No tag matched, drop the packet */
            bpf_inst(&mut ins, BPF_RET | BPF_K, 0, 0, 0);
        }

        if !self.subsystem_filter.is_empty() {
            for (subsystem, devtype) in self.subsystem_filter.iter() {
                let subsystem_hash = string_hash32(subsystem);

                /* Load subsystem hash */
                bpf_inst(&mut ins, BPF_LD | BPF_W | BPF_ABS, 0, 0, 24);
                if devtype.is_empty() {
                    /* Jump 1 step when subsystem is not matched */
                    bpf_inst(&mut ins, BPF_JMP | BPF_JEQ | BPF_K, 0, 1, subsystem_hash);
                } else {
                    /* Jump 3 steps when subsystem is not matched */
                    bpf_inst(&mut ins, BPF_JMP | BPF_JEQ | BPF_K, 0, 3, subsystem_hash);
                    /* Load devtype hash */
                    bpf_inst(&mut ins, BPF_LD | BPF_W | BPF_ABS, 0, 0, 28);
                    let devtype_hash = string_hash32(devtype);
                    /* Jump 1 step when devtype is not matched */
                    bpf_inst(&mut ins, BPF_JMP | BPF_JEQ | BPF_K, 0, 1, devtype_hash);
                }

                /* Subsystem matched, pass the packet */
                bpf_inst(&mut ins, BPF_RET | BPF_K, 0, 0, 0xffffffff);
            }

            /* Nothing matched, drop the packet */
            bpf_inst(&mut ins, BPF_RET | BPF_K, 0, 0, 0);
        }

        /* Pass the packet */
        bpf_inst(&mut ins, BPF_RET | BPF_K, 0, 0, 0xffffffff);

        let filter = sock_fprog {
            len: ins.len() as u16,
            filter: ins.as_ptr() as *mut libc::sock_filter,
        };

        let r = unsafe {
            setsockopt(
                self.socket,
                SOL_SOCKET,
                SO_ATTACH_FILTER,
                &filter as *const sock_fprog as *const _,
                size_of::<sock_fprog>() as u32,
            )
        };

        if r < 0 {
            return Err(Error::Nix {
                msg: "failed to set socket filter".to_string(),
                source: nix::Error::from_i32(
                    std::io::Error::last_os_error().raw_os_error().unwrap(),
                ),
            });
        }

        self.filter_uptodate = true;

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
    use crate::{device::*, DeviceAction};
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
            if let Ok(Some(device)) = self.device_monitor.receive_device() {
                println!("{}", device.get_device_id().unwrap());
            }
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
    #[test]
    fn test_monitor_kernel() {
        let device = Device::from_subsystem_sysname("net", "lo").unwrap();
        if device.trigger(DeviceAction::Change).is_err() {
            return;
        }

        let e = Events::new().unwrap();
        let s: Rc<dyn Source> = Rc::new(Monitor {
            device_monitor: DeviceMonitor::new(MonitorNetlinkGroup::Kernel, None),
        });
        e.add_source(s.clone()).unwrap();
        e.set_enabled(s.clone(), EventState::On).unwrap();

        spawn(|| {
            let device = Device::from_subsystem_sysname("net", "lo").unwrap();
            device.set_sysattr_value("uevent", Some("change")).unwrap();
        })
        .join()
        .unwrap();

        e.rloop().unwrap();

        e.del_source(s.clone()).unwrap();
    }

    /// test whether device monitor can receive device message from userspace normally
    #[test]
    fn test_monitor_userspace() {
        let device = Device::from_subsystem_sysname("net", "lo").unwrap();
        if device.trigger(DeviceAction::Change).is_err() {
            return;
        }

        let e = Events::new().unwrap();
        let s: Rc<dyn Source> = Rc::new(Monitor {
            device_monitor: DeviceMonitor::new(MonitorNetlinkGroup::Userspace, None),
        });
        e.add_source(s.clone()).unwrap();
        e.set_enabled(s.clone(), EventState::On).unwrap();

        spawn(|| {
            let device = Device::from_subsystem_sysname("net", "lo").unwrap();
            device.set_action_from_string("change").unwrap();
            device.set_seqnum(1000);

            let broadcaster = DeviceMonitor::new(MonitorNetlinkGroup::None, None);
            broadcaster.send_device(&device, None).unwrap();
        })
        .join()
        .unwrap();

        e.rloop().unwrap();

        e.del_source(s.clone()).unwrap();
    }

    #[test]
    fn test_filter_add_match_subsystem_devtype() {
        let mut device_monitor = DeviceMonitor::new(MonitorNetlinkGroup::Userspace, None);
        assert!(device_monitor
            .filter_add_match_subsystem_devtype("", "")
            .is_err());
        device_monitor
            .filter_add_match_subsystem_devtype("net", "")
            .unwrap();
        device_monitor
            .filter_add_match_subsystem_devtype("block", "disk")
            .unwrap();
    }

    #[test]
    fn test_filter_add_match_tag() {
        let mut device_monitor = DeviceMonitor::new(MonitorNetlinkGroup::Userspace, None);
        assert!(device_monitor.filter_add_match_tag("").is_err());
        device_monitor.filter_add_match_tag("sysmaster").unwrap();
    }
}
