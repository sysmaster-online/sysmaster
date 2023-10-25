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
use nix::sys::statfs;
use nix::unistd::AccessFlags;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::os::unix::prelude::MetadataExt;
use std::path::{Path, PathBuf};
use std::{env, fs};

macro_rules! unsafe_set_and_return {
    ($var:expr, $value:expr) => {
        #[allow(unused_unsafe)]
        unsafe {
            $var = $value;
            if $var != Machine::None {
                return $var;
            }
        }
    };
}

/// Virtualization system
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Machine {
    /// None
    None = 0,
    /// Chroot
    Chroot,
    /// Containerd
    Containerd,
    /// Docker
    Docker,
    /// Initrd
    Initrd,
    /// kvm
    Kvm,
    /// Lxc
    Lxc,
    /// Oracle
    Oracle,
    /// Podman
    Podman,
    /// PowerRVM
    PowerRVM,
    /// Qemu
    Qemu,
    /// Uml
    Uml,
    /// Vmvare
    Vmware,
    /// VMother
    VMother,
    /// Xen
    Xen,
}

impl From<&str> for Machine {
    fn from(s: &str) -> Self {
        match s {
            "chroot" => Machine::Chroot,
            "containerd" => Machine::Containerd,
            "docker" => Machine::Docker,
            "initrd" => Machine::Initrd,
            "kvm" | "KVM" | "linux,kvm" => Machine::Kvm,
            "lxc" => Machine::Lxc,
            "innotek GmbH" | "oracle" | "ORACLE" | "Oracle Corporation" => Machine::Oracle,
            "podman" => Machine::Podman,
            "qemu" | "QEMU" => Machine::Qemu,
            "vmvare" | "VMW" | "VMware" => Machine::Vmware,
            "xen" | "XEN" | "Xen" => Machine::Xen,
            _ => Machine::None,
        }
    }
}

impl Default for Machine {
    fn default() -> Self {
        Machine::None
    }
}

impl Machine {
    /// return type of OS
    pub fn new() -> Self {
        Self::detect_virtualization()
    }

    /// detect Machine type
    pub fn detect_virtualization() -> Self {
        let ret = Self::detect_container();
        if ret == Machine::None {
            return Self::detect_vm();
        }
        Machine::None
    }

    /// detect virtual machine
    pub fn detect_vm() -> Self {
        static mut CACHED_FOUND: Machine = Machine::None;
        unsafe {
            if CACHED_FOUND != Machine::None {
                return CACHED_FOUND;
            }
        }

        let vm_dmi = Machine::detect_vm_dmi();
        if vm_dmi == Machine::Oracle || vm_dmi == Machine::Xen {
            unsafe_set_and_return!(CACHED_FOUND, vm_dmi);
        }

        let vm_uml = Machine::detect_vm_uml();
        unsafe_set_and_return!(CACHED_FOUND, vm_uml);

        // let vm_hyper = Machine::detect_vm_cpuid(); unsupported!!
        // unsafe_set_and_return!(CACHED_FOUND, vm_hyper);

        let vm_xen = Machine::detect_vm_xen();
        unsafe_set_and_return!(CACHED_FOUND, vm_xen);

        let vm_hyper = Machine::detect_vm_hypervisor();
        unsafe_set_and_return!(CACHED_FOUND, vm_hyper);

        #[cfg(target_arch = "aarch64")]
        let vm = Machine::detect_vm_device_tree();
        #[cfg(target_arch = "aarch64")]
        unsafe_set_and_return!(CACHED_FOUND, vm);

        unsafe { CACHED_FOUND }
    }

    /// unsupported
    pub fn detect_vm_cpuid() -> Self {
        todo!()
    }

    /// detect vm by dmi
    pub fn detect_vm_dmi() -> Self {
        let vendors = vec![
            "/sys/class/dmi/id/product_name",
            "/sys/class/dmi/id/sys_vendor",
            "/sys/class/dmi/id/board_vendor",
            "/sys/class/dmi/id/bios_vendor",
        ];

        for vendor in vendors {
            if let Ok(v) = std::fs::read_to_string(vendor) {
                let machine = Machine::from(v.as_str());
                if machine != Machine::None {
                    return machine;
                }
            }
        }

        Machine::None
    }

    /// detect xen
    pub fn detect_vm_xen() -> Self {
        if Path::new("/proc/xen").exists() {
            return Machine::Xen;
        }

        Machine::None
    }

    /// detect vm hypervisor
    pub fn detect_vm_hypervisor() -> Self {
        if let Ok(v) = std::fs::read_to_string("/sys/hypervisor/type") {
            if v.starts_with("xen") {
                return Machine::Xen;
            } else {
                return Machine::VMother;
            }
        }
        Machine::None
    }

    #[cfg(target_arch = "aarch64")]
    /// detect vm by device tree
    pub fn detect_vm_device_tree() -> Self {
        if let Ok(v) = fs::read_to_string("/proc/device-tree/hypervisor/compatible") {
            return Machine::from(v.as_str());
        };

        if Path::new("/proc/device-tree/ibm,partition-name").exists()
            && Path::new("/proc/device-tree/hmc-managed?").exists()
            && Path::new("/proc/device-tree/chosen/qemu,graphic-width").exists()
        {
            return Machine::PowerRVM;
        }
        if let Ok(dir) = std::fs::read_dir("/proc/device-tree") {
            for entry in dir.flatten() {
                if entry.file_name().to_str().unwrap_or("").contains("fw-cfg") {
                    return Machine::Qemu;
                }
            }
        }
        Machine::None
    }

    /// detect vm for uml
    pub fn detect_vm_uml() -> Self {
        let cpuinfo_path = PathBuf::from("/proc/cpuinfo");
        if cpuinfo_path.exists() {
            if let Ok(inner) = File::open(cpuinfo_path) {
                for line in BufReader::new(inner).lines().flatten() {
                    if line.starts_with("vendor_id") && line.contains("User Mode Linux") {
                        return Machine::Uml;
                    }
                }
            }
        }
        Machine::None
    }

    /// if running in container return true, others return false
    pub fn detect_container() -> Self {
        static mut CACHED_FOUND: Machine = Machine::None;
        unsafe {
            if CACHED_FOUND != Machine::None {
                return CACHED_FOUND;
            }
        }
        // unsupported openvz, proot, wsl and microsoft
        if let Ok(v) = fs::read_to_string("/run/host/container-manager") {
            unsafe_set_and_return!(CACHED_FOUND, Machine::from(v.as_str()));
        };

        if let Ok(v) = env::var("container") {
            unsafe_set_and_return!(CACHED_FOUND, Machine::from(v.as_str()));
        }

        if let Ok(v) = fs::read_to_string("/run/systemd/container") {
            unsafe_set_and_return!(CACHED_FOUND, Machine::from(v.as_str()));
        };
        unsafe_set_and_return!(CACHED_FOUND, Self::detect_container_files());

        unsafe { CACHED_FOUND }
    }

    fn detect_container_files() -> Self {
        match nix::unistd::access("/run/.containerenv", AccessFlags::F_OK) {
            Ok(_) => return Machine::Podman,
            Err(e) => {
                log::debug!("access /run/.cantainerenv error: {}", e);
            }
        }

        match nix::unistd::access("/.dockerenv", AccessFlags::F_OK) {
            Ok(_) => return Machine::Docker,
            Err(e) => {
                log::debug!("access /.dockerenv error: {}", e);
            }
        }

        Machine::None
    }

    /// in chroot?
    pub fn in_chroot() -> bool {
        if let Ok(v) = env::var("SYSTEMD_IGNORE_CHROOT") {
            match v.as_str() {
                "true" | "yes" | "1" => return false,
                _ => (),
            }
        }

        if let Ok(a) = fs::metadata("/proc/1/root") {
            if let Ok(b) = fs::metadata("/") {
                if a.dev() == b.dev() && a.ino() == b.ino() {
                    return true;
                };
            }
        };

        false
    }

    /// in initrd?
    pub fn in_initrd(path: Option<&str>) -> bool {
        #[cfg(target_env = "musl")]
        type FsTypeT = libc::c_ulong;
        #[cfg(not(target_env = "musl"))]
        type FsTypeT = libc::c_long;

        let path = path.map_or("/", |path| path);
        let is_tmpfs = statfs::statfs(path).map_or(false, |s| {
            s.filesystem_type() == statfs::FsType(libc::TMPFS_MAGIC as FsTypeT)
        });

        let has_initrd_release: bool = Path::new("/etc/initrd-release").exists();

        is_tmpfs && has_initrd_release
    }
}

#[cfg(test)]
mod tests {
    use crate::machine::Machine;

    #[test]
    fn test_machine() {
        let machine = Machine::new();
        println!("{:?}", machine);
        assert_ne!(Machine::Oracle, machine);
    }
}
