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

//!

/// from <linux/capability.h>
#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
#[repr(u8)]
pub enum Capability {
    /// `CAP_CHOWN` (from POSIX)
    CAP_CHOWN = 0,
    /// `CAP_DAC_OVERRIDE` (from POSIX)
    CAP_DAC_OVERRIDE = 1,
    /// `CAP_DAC_READ_SEARCH` (from POSIX)
    CAP_DAC_READ_SEARCH = 2,
    /// `CAP_FOWNER` (from POSIX)
    CAP_FOWNER = 3,
    /// `CAP_FSETID` (from POSIX)
    CAP_FSETID = 4,
    /// `CAP_KILL` (from POSIX)
    CAP_KILL = 5,
    /// `CAP_SETGID` (from POSIX)
    CAP_SETGID = 6,
    /// `CAP_SETUID` (from POSIX)
    CAP_SETUID = 7,
    /// `CAP_SETPCAP` (from Linux)
    CAP_SETPCAP = 8,
    ///
    CAP_LINUX_IMMUTABLE = 9,
    ///
    CAP_NET_BIND_SERVICE = 10,
    ///
    CAP_NET_BROADCAST = 11,
    ///
    CAP_NET_ADMIN = 12,
    ///
    CAP_NET_RAW = 13,
    ///
    CAP_IPC_LOCK = 14,
    ///
    CAP_IPC_OWNER = 15,
    /// `CAP_SYS_MODULE` (from Linux)
    CAP_SYS_MODULE = 16,
    /// `CAP_SYS_RAWIO` (from Linux)
    CAP_SYS_RAWIO = 17,
    /// `CAP_SYS_CHROOT` (from Linux)
    CAP_SYS_CHROOT = 18,
    /// `CAP_SYS_PTRACE` (from Linux)
    CAP_SYS_PTRACE = 19,
    /// `CAP_SYS_PACCT` (from Linux)
    CAP_SYS_PACCT = 20,
    /// `CAP_SYS_ADMIN` (from Linux)
    CAP_SYS_ADMIN = 21,
    /// `CAP_SYS_BOOT` (from Linux)
    CAP_SYS_BOOT = 22,
    /// `CAP_SYS_NICE` (from Linux)
    CAP_SYS_NICE = 23,
    /// `CAP_SYS_RESOURCE` (from Linux)
    CAP_SYS_RESOURCE = 24,
    /// `CAP_SYS_TIME` (from Linux)
    CAP_SYS_TIME = 25,
    /// `CAP_SYS_TTY_CONFIG` (from Linux)
    CAP_SYS_TTY_CONFIG = 26,
    /// `CAP_SYS_MKNOD` (from Linux, >= 2.4)
    CAP_MKNOD = 27,
    /// `CAP_LEASE` (from Linux, >= 2.4)
    CAP_LEASE = 28,
    ///
    CAP_AUDIT_WRITE = 29,
    /// `CAP_AUDIT_CONTROL` (from Linux, >= 2.6.11)
    CAP_AUDIT_CONTROL = 30,
    ///
    CAP_SETFCAP = 31,
    ///
    CAP_MAC_OVERRIDE = 32,
    ///
    CAP_MAC_ADMIN = 33,
    /// `CAP_SYSLOG` (from Linux, >= 2.6.37)
    CAP_SYSLOG = 34,
    /// `CAP_WAKE_ALARM` (from Linux, >= 3.0)
    CAP_WAKE_ALARM = 35,
    ///
    CAP_BLOCK_SUSPEND = 36,
    /// `CAP_AUDIT_READ` (from Linux, >= 3.16).
    CAP_AUDIT_READ = 37,
    /// `CAP_PERFMON` (from Linux, >= 5.8).
    CAP_PERFMON = 38,
    /// `CAP_BPF` (from Linux, >= 5.8).
    CAP_BPF = 39,
    /// `CAP_CHECKPOINT_RESTORE` (from Linux, >= 5.9).
    CAP_CHECKPOINT_RESTORE = 40,
}

impl std::fmt::Display for Capability {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let name = match *self {
            Capability::CAP_CHOWN => "CAP_CHOWN",
            Capability::CAP_DAC_OVERRIDE => "CAP_DAC_OVERRIDE",
            Capability::CAP_DAC_READ_SEARCH => "CAP_DAC_READ_SEARCH",
            Capability::CAP_FOWNER => "CAP_FOWNER",
            Capability::CAP_FSETID => "CAP_FSETID",
            Capability::CAP_KILL => "CAP_KILL",
            Capability::CAP_SETGID => "CAP_SETGID",
            Capability::CAP_SETUID => "CAP_SETUID",
            Capability::CAP_SETPCAP => "CAP_SETPCAP",
            Capability::CAP_LINUX_IMMUTABLE => "CAP_LINUX_IMMUTABLE",
            Capability::CAP_NET_BIND_SERVICE => "CAP_NET_BIND_SERVICE",
            Capability::CAP_NET_BROADCAST => "CAP_NET_BROADCAST",
            Capability::CAP_NET_ADMIN => "CAP_NET_ADMIN",
            Capability::CAP_NET_RAW => "CAP_NET_RAW",
            Capability::CAP_IPC_LOCK => "CAP_IPC_LOCK",
            Capability::CAP_IPC_OWNER => "CAP_IPC_OWNER",
            Capability::CAP_SYS_MODULE => "CAP_SYS_MODULE",
            Capability::CAP_SYS_RAWIO => "CAP_SYS_RAWIO",
            Capability::CAP_SYS_CHROOT => "CAP_SYS_CHROOT",
            Capability::CAP_SYS_PTRACE => "CAP_SYS_PTRACE",
            Capability::CAP_SYS_PACCT => "CAP_SYS_PACCT",
            Capability::CAP_SYS_ADMIN => "CAP_SYS_ADMIN",
            Capability::CAP_SYS_BOOT => "CAP_SYS_BOOT",
            Capability::CAP_SYS_NICE => "CAP_SYS_NICE",
            Capability::CAP_SYS_RESOURCE => "CAP_SYS_RESOURCE",
            Capability::CAP_SYS_TIME => "CAP_SYS_TIME",
            Capability::CAP_SYS_TTY_CONFIG => "CAP_SYS_TTY_CONFIG",
            Capability::CAP_MKNOD => "CAP_MKNOD",
            Capability::CAP_LEASE => "CAP_LEASE",
            Capability::CAP_AUDIT_WRITE => "CAP_AUDIT_WRITE",
            Capability::CAP_AUDIT_CONTROL => "CAP_AUDIT_CONTROL",
            Capability::CAP_SETFCAP => "CAP_SETFCAP",
            Capability::CAP_MAC_OVERRIDE => "CAP_MAC_OVERRIDE",
            Capability::CAP_MAC_ADMIN => "CAP_MAC_ADMIN",
            Capability::CAP_SYSLOG => "CAP_SYSLOG",
            Capability::CAP_WAKE_ALARM => "CAP_WAKE_ALARM",
            Capability::CAP_BLOCK_SUSPEND => "CAP_BLOCK_SUSPEND",
            Capability::CAP_AUDIT_READ => "CAP_AUDIT_READ",
            Capability::CAP_PERFMON => "CAP_PERFMON",
            Capability::CAP_BPF => "CAP_BPF",
            Capability::CAP_CHECKPOINT_RESTORE => "CAP_CHECKPOINT_RESTORE",
        };
        write!(f, "{}", name)
    }
}

impl std::str::FromStr for Capability {
    type Err = crate::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "CAP_CHOWN" => Ok(Capability::CAP_CHOWN),
            "CAP_DAC_OVERRIDE" => Ok(Capability::CAP_DAC_OVERRIDE),
            "CAP_DAC_READ_SEARCH" => Ok(Capability::CAP_DAC_READ_SEARCH),
            "CAP_FOWNER" => Ok(Capability::CAP_FOWNER),
            "CAP_FSETID" => Ok(Capability::CAP_FSETID),
            "CAP_KILL" => Ok(Capability::CAP_KILL),
            "CAP_SETGID" => Ok(Capability::CAP_SETGID),
            "CAP_SETUID" => Ok(Capability::CAP_SETUID),
            "CAP_SETPCAP" => Ok(Capability::CAP_SETPCAP),
            "CAP_LINUX_IMMUTABLE" => Ok(Capability::CAP_LINUX_IMMUTABLE),
            "CAP_NET_BIND_SERVICE" => Ok(Capability::CAP_NET_BIND_SERVICE),
            "CAP_NET_BROADCAST" => Ok(Capability::CAP_NET_BROADCAST),
            "CAP_NET_ADMIN" => Ok(Capability::CAP_NET_ADMIN),
            "CAP_NET_RAW" => Ok(Capability::CAP_NET_RAW),
            "CAP_IPC_LOCK" => Ok(Capability::CAP_IPC_LOCK),
            "CAP_IPC_OWNER" => Ok(Capability::CAP_IPC_OWNER),
            "CAP_SYS_MODULE" => Ok(Capability::CAP_SYS_MODULE),
            "CAP_SYS_RAWIO" => Ok(Capability::CAP_SYS_RAWIO),
            "CAP_SYS_CHROOT" => Ok(Capability::CAP_SYS_CHROOT),
            "CAP_SYS_PTRACE" => Ok(Capability::CAP_SYS_PTRACE),
            "CAP_SYS_PACCT" => Ok(Capability::CAP_SYS_PACCT),
            "CAP_SYS_ADMIN" => Ok(Capability::CAP_SYS_ADMIN),
            "CAP_SYS_BOOT" => Ok(Capability::CAP_SYS_BOOT),
            "CAP_SYS_NICE" => Ok(Capability::CAP_SYS_NICE),
            "CAP_SYS_RESOURCE" => Ok(Capability::CAP_SYS_RESOURCE),
            "CAP_SYS_TIME" => Ok(Capability::CAP_SYS_TIME),
            "CAP_SYS_TTY_CONFIG" => Ok(Capability::CAP_SYS_TTY_CONFIG),
            "CAP_MKNOD" => Ok(Capability::CAP_MKNOD),
            "CAP_LEASE" => Ok(Capability::CAP_LEASE),
            "CAP_AUDIT_WRITE" => Ok(Capability::CAP_AUDIT_WRITE),
            "CAP_AUDIT_CONTROL" => Ok(Capability::CAP_AUDIT_CONTROL),
            "CAP_SETFCAP" => Ok(Capability::CAP_SETFCAP),
            "CAP_MAC_OVERRIDE" => Ok(Capability::CAP_MAC_OVERRIDE),
            "CAP_MAC_ADMIN" => Ok(Capability::CAP_MAC_ADMIN),
            "CAP_SYSLOG" => Ok(Capability::CAP_SYSLOG),
            "CAP_WAKE_ALARM" => Ok(Capability::CAP_WAKE_ALARM),
            "CAP_BLOCK_SUSPEND" => Ok(Capability::CAP_BLOCK_SUSPEND),
            "CAP_AUDIT_READ" => Ok(Capability::CAP_AUDIT_READ),
            "CAP_PERFMON" => Ok(Capability::CAP_PERFMON),
            "CAP_BPF" => Ok(Capability::CAP_BPF),
            "CAP_CHECKPOINT_RESTORE" => Ok(Capability::CAP_CHECKPOINT_RESTORE),
            _ => Err(crate::Error::Caps {
                what: format!("invalid capability: {}", s),
            }),
        }
    }
}

impl Capability {
    ///
    pub fn bitmask(&self) -> u64 {
        1u64 << (*self as u8)
    }

    ///
    pub fn index(&self) -> u8 {
        *self as u8
    }
}
