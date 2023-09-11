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

//! some common used security functions

#[allow(unused_imports)]
use std::{
    fs::File,
    io::{BufReader, Read},
    os::{raw::c_int, unix::prelude::AsRawFd},
    path::Path,
};

use nix::{
    errno::Errno,
    sys::{
        socket::{socket, AddressFamily, SockFlag, SockProtocol, SockType},
        stat::fstat,
    },
};

/// check if selinux is enabled
pub fn selinux_enabled() -> bool {
    #[cfg(feature = "selinux")]
    {
        let res = unsafe { selinux::is_selinux_enabled() };
        res > 0
    }

    #[cfg(not(feature = "selinux"))]
    {
        false
    }
}

/// sets the context used for the next execve call
#[allow(unused_variables)]
pub fn set_exec_context(context: &str) -> i32 {
    #[cfg(feature = "selinux")]
    {
        let context = String::from(context);
        let context = std::ffi::CString::new(context).unwrap();
        unsafe { selinux::setexeccon(context.as_ptr()) }
    }

    #[cfg(not(feature = "selinux"))]
    {
        0
    }
}

/// check if smack is enabled
pub fn smack_enabled() -> bool {
    Path::new("/sys/fs/smackfs").exists()
}

/// check if apparmor is enabled
pub fn apparmor_enabled() -> bool {
    let mut file = match File::open("/sys/module/apparmor/parameters/enabled") {
        Err(_) => {
            return false;
        }
        Ok(v) => v,
    };
    let mut buf = [0u8; 2];
    let _ = file.read_exact(&mut buf);
    buf == [b'Y', b'\n']
}

/// check if audit is enabled
pub fn audit_enabled() -> bool {
    match socket(
        AddressFamily::Netlink,
        SockType::Raw,
        SockFlag::SOCK_CLOEXEC | SockFlag::SOCK_NONBLOCK,
        SockProtocol::NetlinkAudit,
    ) {
        Ok(fd) => fd > 0,
        Err(Errno::EAFNOSUPPORT) | Err(Errno::ENOTSUP) | Err(Errno::EPERM) => false,
        Err(_) => true,
    }
}

/// check if ima is enabled
pub fn ima_enabled() -> bool {
    Path::new("/sys/kernel/security/ima/").exists()
}

/// check if the tomoyo is enabled
pub fn tomoyo_enabled() -> bool {
    Path::new("/sys/kernel/security/tomoyo/version").exists()
}

/// check if the uefi-secureboot is enabled
pub fn uefi_secureboot_enabled() -> bool {
    /* mokutil check 3 files to tell if the system is secure boot enabled,
     * while systemd only checks one file, we decide to copy the logic of
     * systemd. The 128-bit id is a magic number, See:
     * https://uefi.org/sites/default/files/resources/UEFI_Spec_Errata_Only.pdf */
    let uefi_file_path =
        "/sys/firmware/efi/efivars/SecureBoot-8be4df61-93ca-11d2-aa0d-00e098032b8c";
    let file = match File::open(uefi_file_path) {
        Err(_) => {
            return false;
        }
        Ok(v) => v,
    };
    let stat = match fstat(file.as_raw_fd()) {
        Err(_) => return false,
        Ok(v) => v,
    };

    /* file too small or too large. */
    if stat.st_size < 4 || stat.st_size > 4 * 1024 * 1024 + 4 {
        return false;
    }

    let mut reader = BufReader::new(file);
    /* The file should be 5 bytes, the first 4 bytes is attribute,
     * the following 1 byte is value, we only care the last 1 byte
     * here. */
    let mut buf = [0_u8; 5];
    if reader.read_exact(&mut buf).is_err() {
        return false;
    }
    let value = *buf.get(4).unwrap();

    value > 0
}

/// check if the tpm2 is enabled
pub fn tpm2_enabled() -> bool {
    Path::new("/sys/class/tpmrm").exists()
}
