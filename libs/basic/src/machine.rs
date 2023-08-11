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
use std::env;
use std::path::Path;

/// Virtualization system
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Machine {
    /// not virtualization
    None = 0,
    /// Host machine
    Host,
    /// Initrd
    Initrd,
    /// docker virtualization
    Docker,
    /// lxc virtualization
    Lxc,
    /// podman virtualization
    Podman,
    /// podman virtualization
    Containerd,
    /// not supported virtualization
    NotSupported,
}

impl From<String> for Machine {
    fn from(s: String) -> Self {
        match s.as_str() {
            "host" => Machine::Host,
            "initrd" => Machine::Initrd,
            "podman" => Machine::Podman,
            "lxc" => Machine::Lxc,
            "docker" => Machine::Docker,
            "containerd" => Machine::Containerd,
            _ => Machine::NotSupported,
        }
    }
}

impl Machine {
    /// Whether in initrd
    pub fn in_initrd(path: Option<&str>) -> bool {
        #[cfg(target_env = "musl")]
        type FsTypeT = libc::c_ulong;
        #[cfg(not(target_env = "musl"))]
        type FsTypeT = libc::c_long;

        let path = path.map_or("/", |path| path);
        let is_tmpfs = statfs::statfs(path).map_or(false, |s| {
            s.filesystem_type() == statfs::FsType(libc::TMPFS_MAGIC as FsTypeT)
        });

        let has_initrd_release = Path::new("/etc/initrd-release").exists();

        is_tmpfs && has_initrd_release
    }

    /// if running in container return true, others return false
    pub fn detect_container() -> Machine {
        if let Ok(v) = env::var("container") {
            if v.is_empty() {
                return Machine::None;
            }
            return Machine::from(v);
        }

        Self::detect_container_files()
    }

    fn detect_container_files() -> Machine {
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
}
