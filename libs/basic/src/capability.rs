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

//! Capability functions

/// from <linux/capability.h>
#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
#[allow(non_camel_case_types)]
#[repr(u8)]
pub enum Capability {
    /// `CHOWN` (from POSIX)
    CHOWN = 0,
    /// `DAC_OVERRIDE` (from POSIX)
    DAC_OVERRIDE = 1,
    /// `DAC_READ_SEARCH` (from POSIX)
    DAC_READ_SEARCH = 2,
    /// `FOWNER` (from POSIX)
    FOWNER = 3,
    /// `FSETID` (from POSIX)
    FSETID = 4,
    /// `KILL` (from POSIX)
    KILL = 5,
    /// `SETGID` (from POSIX)
    SETGID = 6,
    /// `SETUID` (from POSIX)
    SETUID = 7,
    /// `SETPCAP` (from Linux)
    SETPCAP = 8,
    ///
    LINUX_IMMUTABLE = 9,
    ///
    NET_BIND_SERVICE = 10,
    ///
    NET_BROADCAST = 11,
    ///
    NET_ADMIN = 12,
    ///
    NET_RAW = 13,
    ///
    IPC_LOCK = 14,
    ///
    IPC_OWNER = 15,
    /// `SYS_MODULE` (from Linux)
    SYS_MODULE = 16,
    /// `SYS_RAWIO` (from Linux)
    SYS_RAWIO = 17,
    /// `SYS_CHROOT` (from Linux)
    SYS_CHROOT = 18,
    /// `SYS_PTRACE` (from Linux)
    SYS_PTRACE = 19,
    /// `SYS_PACCT` (from Linux)
    SYS_PACCT = 20,
    /// `SYS_ADMIN` (from Linux)
    SYS_ADMIN = 21,
    /// `SYS_BOOT` (from Linux)
    SYS_BOOT = 22,
    /// `SYS_NICE` (from Linux)
    SYS_NICE = 23,
    /// `SYS_RESOURCE` (from Linux)
    SYS_RESOURCE = 24,
    /// `SYS_TIME` (from Linux)
    SYS_TIME = 25,
    /// `SYS_TTY_CONFIG` (from Linux)
    SYS_TTY_CONFIG = 26,
    /// `SYS_MKNOD` (from Linux, >= 2.4)
    MKNOD = 27,
    /// `LEASE` (from Linux, >= 2.4)
    LEASE = 28,
    ///
    AUDIT_WRITE = 29,
    /// `AUDIT_CONTROL` (from Linux, >= 2.6.11)
    AUDIT_CONTROL = 30,
    ///
    SETFCAP = 31,
    ///
    MAC_OVERRIDE = 32,
    ///
    MAC_ADMIN = 33,
    /// `SYSLOG` (from Linux, >= 2.6.37)
    SYSLOG = 34,
    /// `WAKE_ALARM` (from Linux, >= 3.0)
    WAKE_ALARM = 35,
    ///
    BLOCK_SUSPEND = 36,
    /// `AUDIT_READ` (from Linux, >= 3.16).
    AUDIT_READ = 37,
    /// `PERFMON` (from Linux, >= 5.8).
    PERFMON = 38,
    /// `BPF` (from Linux, >= 5.8).
    BPF = 39,
    /// `CHECKPOINT_RESTORE` (from Linux, >= 5.9).
    CHECKPOINT_RESTORE = 40,
}

impl std::fmt::Display for Capability {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let name = match *self {
            Capability::CHOWN => "CHOWN",
            Capability::DAC_OVERRIDE => "DAC_OVERRIDE",
            Capability::DAC_READ_SEARCH => "DAC_READ_SEARCH",
            Capability::FOWNER => "FOWNER",
            Capability::FSETID => "FSETID",
            Capability::KILL => "KILL",
            Capability::SETGID => "SETGID",
            Capability::SETUID => "SETUID",
            Capability::SETPCAP => "SETPCAP",
            Capability::LINUX_IMMUTABLE => "LINUX_IMMUTABLE",
            Capability::NET_BIND_SERVICE => "NET_BIND_SERVICE",
            Capability::NET_BROADCAST => "NET_BROADCAST",
            Capability::NET_ADMIN => "NET_ADMIN",
            Capability::NET_RAW => "NET_RAW",
            Capability::IPC_LOCK => "IPC_LOCK",
            Capability::IPC_OWNER => "IPC_OWNER",
            Capability::SYS_MODULE => "SYS_MODULE",
            Capability::SYS_RAWIO => "SYS_RAWIO",
            Capability::SYS_CHROOT => "SYS_CHROOT",
            Capability::SYS_PTRACE => "SYS_PTRACE",
            Capability::SYS_PACCT => "SYS_PACCT",
            Capability::SYS_ADMIN => "SYS_ADMIN",
            Capability::SYS_BOOT => "SYS_BOOT",
            Capability::SYS_NICE => "SYS_NICE",
            Capability::SYS_RESOURCE => "SYS_RESOURCE",
            Capability::SYS_TIME => "SYS_TIME",
            Capability::SYS_TTY_CONFIG => "SYS_TTY_CONFIG",
            Capability::MKNOD => "MKNOD",
            Capability::LEASE => "LEASE",
            Capability::AUDIT_WRITE => "AUDIT_WRITE",
            Capability::AUDIT_CONTROL => "AUDIT_CONTROL",
            Capability::SETFCAP => "SETFCAP",
            Capability::MAC_OVERRIDE => "MAC_OVERRIDE",
            Capability::MAC_ADMIN => "MAC_ADMIN",
            Capability::SYSLOG => "SYSLOG",
            Capability::WAKE_ALARM => "WAKE_ALARM",
            Capability::BLOCK_SUSPEND => "BLOCK_SUSPEND",
            Capability::AUDIT_READ => "AUDIT_READ",
            Capability::PERFMON => "PERFMON",
            Capability::BPF => "BPF",
            Capability::CHECKPOINT_RESTORE => "CHECKPOINT_RESTORE",
        };
        write!(f, "{}", name)
    }
}

impl std::str::FromStr for Capability {
    type Err = crate::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "CHOWN" => Ok(Capability::CHOWN),
            "DAC_OVERRIDE" => Ok(Capability::DAC_OVERRIDE),
            "DAC_READ_SEARCH" => Ok(Capability::DAC_READ_SEARCH),
            "FOWNER" => Ok(Capability::FOWNER),
            "FSETID" => Ok(Capability::FSETID),
            "KILL" => Ok(Capability::KILL),
            "SETGID" => Ok(Capability::SETGID),
            "SETUID" => Ok(Capability::SETUID),
            "SETPCAP" => Ok(Capability::SETPCAP),
            "LINUX_IMMUTABLE" => Ok(Capability::LINUX_IMMUTABLE),
            "NET_BIND_SERVICE" => Ok(Capability::NET_BIND_SERVICE),
            "NET_BROADCAST" => Ok(Capability::NET_BROADCAST),
            "NET_ADMIN" => Ok(Capability::NET_ADMIN),
            "NET_RAW" => Ok(Capability::NET_RAW),
            "IPC_LOCK" => Ok(Capability::IPC_LOCK),
            "IPC_OWNER" => Ok(Capability::IPC_OWNER),
            "SYS_MODULE" => Ok(Capability::SYS_MODULE),
            "SYS_RAWIO" => Ok(Capability::SYS_RAWIO),
            "SYS_CHROOT" => Ok(Capability::SYS_CHROOT),
            "SYS_PTRACE" => Ok(Capability::SYS_PTRACE),
            "SYS_PACCT" => Ok(Capability::SYS_PACCT),
            "SYS_ADMIN" => Ok(Capability::SYS_ADMIN),
            "SYS_BOOT" => Ok(Capability::SYS_BOOT),
            "SYS_NICE" => Ok(Capability::SYS_NICE),
            "SYS_RESOURCE" => Ok(Capability::SYS_RESOURCE),
            "SYS_TIME" => Ok(Capability::SYS_TIME),
            "SYS_TTY_CONFIG" => Ok(Capability::SYS_TTY_CONFIG),
            "MKNOD" => Ok(Capability::MKNOD),
            "LEASE" => Ok(Capability::LEASE),
            "AUDIT_WRITE" => Ok(Capability::AUDIT_WRITE),
            "AUDIT_CONTROL" => Ok(Capability::AUDIT_CONTROL),
            "SETFCAP" => Ok(Capability::SETFCAP),
            "MAC_OVERRIDE" => Ok(Capability::MAC_OVERRIDE),
            "MAC_ADMIN" => Ok(Capability::MAC_ADMIN),
            "SYSLOG" => Ok(Capability::SYSLOG),
            "WAKE_ALARM" => Ok(Capability::WAKE_ALARM),
            "BLOCK_SUSPEND" => Ok(Capability::BLOCK_SUSPEND),
            "AUDIT_READ" => Ok(Capability::AUDIT_READ),
            "PERFMON" => Ok(Capability::PERFMON),
            "BPF" => Ok(Capability::BPF),
            "CHECKPOINT_RESTORE" => Ok(Capability::CHECKPOINT_RESTORE),
            _ => Err(crate::Error::Caps {
                what: format!("invalid capability: {}", s),
            }),
        }
    }
}

impl Capability {
    /// The `bitmask` function returns a bitmask where the bit at the position of `self` is set to 1 and
    /// all other bits are set to 0.
    ///
    /// Returns:
    ///
    /// The bitmask function returns a u64 value, which is the result of shifting 1u64 to the left by
    /// the value of *self as u8.
    pub fn bitmask(&self) -> u64 {
        1u64 << (*self as u8)
    }

    /// The function returns the index of a value as an unsigned 8-bit integer.
    ///
    /// Returns:
    ///
    /// The index of the object, converted to an unsigned 8-bit integer.
    pub fn index(&self) -> u8 {
        *self as u8
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitmask_test() {
        let vec = Capability::WAKE_ALARM;

        assert_eq!(vec.bitmask(), 1 << 35);
    }

    #[test]
    fn index_test() {
        let vec = Capability::WAKE_ALARM;

        assert_eq!(vec.index(), 35);
    }
}
