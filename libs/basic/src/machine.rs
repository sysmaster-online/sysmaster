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
use crate::{
    id128::{self, Id128FormatFlag},
    namespace,
};
use libc::syncfs;
use log;
use nix::{
    dir::Dir,
    errno::Errno,
    fcntl,
    fcntl::OFlag,
    mount, sched,
    sys::{stat::*, statfs},
    unistd::{self, sync, AccessFlags, Pid},
    Result,
};
use std::{
    env, fs,
    fs::File,
    io::{BufRead, BufReader, ErrorKind},
    ops::BitAnd,
    os::unix::io::{AsRawFd, RawFd},
    os::unix::prelude::MetadataExt,
    path::{Path, PathBuf},
    process,
};
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

        #[cfg(target_arch = "riscv64")]
        let vm = Machine::detect_vm_device_tree();
        #[cfg(target_arch = "riscv64")]
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
                if entry.path().to_string_lossy().contains("fw-cfg") {
                    return Machine::Qemu;
                }
            }
        }
        Machine::None
    }

    #[cfg(target_arch = "riscv64")]
    /// detect vm by device tree
    pub fn detect_vm_device_tree() -> Self {
        if let Ok(dir) = std::fs::read_dir("/proc/device-tree") {
            for entry in dir.flatten() {
                if entry.path().to_string_lossy().contains("fw-cfg") {
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

fn syncfs_path(path: &str) -> Result<()> {
    if path.is_empty() {
        return Err(Errno::EINVAL);
    }

    let fd = Dir::open(
        path,
        OFlag::O_RDONLY | OFlag::O_CLOEXEC | OFlag::O_NONBLOCK,
        Mode::empty(),
    )?;
    match unsafe { syncfs(fd.as_raw_fd()) } {
        0 => Ok(()),
        e => Err(Errno::from_i32(e)),
    }
}

/// Write machine-id from memory to disk
pub fn machine_id_commit() -> Result<()> {
    #[cfg(target_env = "musl")]
    type FsTypeT = libc::c_ulong;
    #[cfg(not(target_env = "musl"))]
    type FsTypeT = libc::c_long;

    let etc_machine_id = Path::new("/etc/machine-id");
    let id128: String;
    let mnt_fd: RawFd;

    syncfs_path("/etc/")?;
    syncfs_path("/var/")?;

    sync();

    if !crate::mount::is_mount_point(etc_machine_id) {
        log::debug!("{:?} is not a mount point. Nothing to do.", etc_machine_id);
        return Ok(());
    }

    if !crate::fs::check_filesystem(etc_machine_id, statfs::FsType(libc::TMPFS_MAGIC as FsTypeT)) {
        log::error!("{:?} is not on a temporary file system.", etc_machine_id);
        return Err(nix::Error::EROFS);
    }

    match id128::id128_read_by_path(etc_machine_id, Id128FormatFlag::ID128_FORMAT_PLAIN) {
        Ok(id128_string) => id128 = id128_string,
        Err(e) => {
            log::error!(
                "We didn't find a valid machine ID in {:?}:{}",
                etc_machine_id,
                e
            );
            return Err(nix::Error::EINVAL);
        }
    }

    mnt_fd = namespace::namespace_open(&Pid::from_raw(0), Path::new(&"mnt".to_string()))?;

    namespace::detach_mount_namespace()?;

    if let Err(e) = mount::umount2(etc_machine_id, mount::MntFlags::from_bits(0).unwrap()) {
        log::error!("Failed to umount {:?}:{}", etc_machine_id, e);
        return Err(e);
    }

    id128::id128_write(
        etc_machine_id,
        &true,
        &id128,
        Id128FormatFlag::ID128_FORMAT_PLAIN,
    )?;

    namespace::namespace_enter(&mnt_fd, sched::CloneFlags::CLONE_NEWNS)?;

    mount::umount2(etc_machine_id, mount::MntFlags::MNT_DETACH)
}

fn generate_machine_id() -> Result<String> {
    let dbus_machine_id = Path::new("/var/lib/dbus/machine-id");

    if let Ok(id128) =
        id128::id128_read_by_path(dbus_machine_id, Id128FormatFlag::ID128_FORMAT_PLAIN)
    {
        log::info!("Initializing machine ID from D-Bus machine ID (/var/lib/dbus/machine-id).");
        return Ok(id128);
    }

    if process::id() == 1 {
        if let Ok(id128) = env::var("container_uuid") {
            if id128::id128_is_valid(&id128.clone().into_bytes()) {
                log::info!(
                    "Initializing machine ID from container UUID (process 1's container_uuid)."
                );
                return Ok(id128);
            }
        }
    } else {
        let penv = String::from_utf8(fs::read(Path::new("/proc/1/environ")).unwrap()).unwrap();
        let idv: Vec<&str> = penv.split("container_uuid=").collect();
        if idv.len() > 1 {
            let id128 = idv[1];
            let idplain: String = id128.chars().take(32).collect();
            let idrfc: String = id128.chars().take(36).collect();

            if id128::id128_is_valid(&idplain.clone().into_bytes()) {
                log::info!("Initializing machine ID from environ's container UUID (/proc/1/environ's container_uuid).");
                return Ok(idplain);
            } else if id128::id128_is_valid(&idrfc.clone().into_bytes()) {
                log::info!("Initializing machine ID from environ's container UUID (/proc/1/environ's container_uuid).");
                return Ok(idrfc);
            }
        }
    }

    match id128::id128_read_by_path(
        Path::new("/sys/class/dmi/id/product_uuid"),
        Id128FormatFlag::ID128_FORMAT_UUID,
    ) {
        Ok(id128) => {
            log::info!("Initializing machine ID from VM UUID (/sys/class/dmi/id/product_uuid).");
            return Ok(id128);
        }
        Err(e) => {
            if e.kind() == ErrorKind::NotFound {
                if let Ok(id128) = id128::id128_read_by_path(
                    Path::new("/proc/device-tree/vm,uuid"),
                    Id128FormatFlag::ID128_FORMAT_UUID,
                ) {
                    log::info!("Initializing machine ID from VM UUID (/proc/device-tree/vm,uuid).");
                    return Ok(id128);
                }
            }
        }
    }

    log::info!("Initializing machine ID from random generator.");
    id128::id128_randomize(Id128FormatFlag::ID128_FORMAT_PLAIN)
}

///
pub fn machine_id_setup(force_transient: bool, machine_id: &str) -> Result<String> {
    let etc_machine_id = Path::new("/etc/machine-id");
    let run_machine_id = Path::new("/run/machine-id");
    let mut writable = false;
    let mut fd: RawFd = -1;
    let ret_id128: String;

    // with umask(0000)
    let mut saved_umask = SFlag::from_bits_truncate(
        umask(Mode::from_bits_truncate(0o0000)).bits() | SFlag::S_IFMT.bits(),
    );
    while saved_umask.contains(SFlag::S_IFMT) {
        match fcntl::open(
            etc_machine_id,
            OFlag::O_RDWR | OFlag::O_CREAT | OFlag::O_NOCTTY | OFlag::O_CLOEXEC,
            Mode::from_bits_truncate(0o0444),
        ) {
            Ok(f) => {
                writable = true;
                fd = f;
            }
            Err(e1) => {
                match fcntl::open(
                    etc_machine_id,
                    OFlag::O_RDONLY | OFlag::O_NOCTTY | OFlag::O_CLOEXEC,
                    Mode::from_bits_truncate(0o0444),
                ) {
                    Ok(_) => writable = false,
                    Err(e2) => {
                        if e1 == Errno::EROFS && e2 == Errno::ENOENT {
                            log::error!("System cannot boot: Missing /etc/machine/id and /etc is mounted read-only.
Booting up is supported only when:
1) /etc/machine-id exists and is populated.
2) /etc/machine-id exists and is empty
3) /etc/machine-id is missing and /etc is writable.");
                        } else {
                            log::error!("Cannot open {:?}:{}", etc_machine_id, e2);
                        }
                        return Err(e2);
                    }
                }
            }
        };
        saved_umask = saved_umask.bitand(SFlag::from_bits_truncate(0o0777));
    }

    if machine_id.is_empty() {
        if let Ok(id128) =
            id128::id128_read_by_path(etc_machine_id, Id128FormatFlag::ID128_FORMAT_PLAIN)
        {
            return Ok(id128);
        }
        ret_id128 = generate_machine_id()?;
    } else {
        ret_id128 = machine_id.to_string();
    }

    if writable {
        if force_transient {
            if let Err(e) = unistd::write(fd, "uninitialized\n".as_bytes()) {
                log::error!("Failed to write uninitialized {:?}:{}", etc_machine_id, e);
                return Err(e);
            }
        } else {
            id128::id128_write(
                etc_machine_id,
                &true,
                &ret_id128,
                Id128FormatFlag::ID128_FORMAT_PLAIN,
            )?;
            unistd::close(fd)?;
            return Ok(ret_id128);
        }
    }

    unistd::close(fd)?;

    // with umask(0022)
    let mut saved_umask = SFlag::from_bits_truncate(
        umask(Mode::from_bits_truncate(0o0022)).bits() | SFlag::S_IFMT.bits(),
    );
    while saved_umask.contains(SFlag::S_IFMT) {
        id128::id128_write(
            run_machine_id,
            &false,
            &ret_id128,
            Id128FormatFlag::ID128_FORMAT_PLAIN,
        )?;
        saved_umask = saved_umask.bitand(SFlag::from_bits_truncate(0o0777));
    }

    mount::mount(
        Some(run_machine_id),
        etc_machine_id,
        None::<&str>,
        mount::MsFlags::MS_BIND,
        None::<&str>,
    )?;

    mount::mount(
        None::<&str>,
        etc_machine_id,
        None::<&str>,
        mount::MsFlags::MS_BIND | mount::MsFlags::MS_RDONLY | mount::MsFlags::MS_REMOUNT,
        None::<&str>,
    )?;

    Ok(ret_id128)
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
