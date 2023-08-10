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
use nix::unistd::AccessFlags;
use std::env;

/// Virtualization system
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Virtualization {
    /// not virtualization
    None = 0,
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

impl From<String> for Virtualization {
    fn from(action: String) -> Self {
        match action.as_ref() {
            "podman" => Virtualization::Podman,
            "lxc" => Virtualization::Lxc,
            "docker" => Virtualization::Docker,
            "containerd" => Virtualization::Containerd,
            _ => Virtualization::NotSupported,
        }
    }
}

/// if running in container return true, others return false
pub fn detect_container() -> Virtualization {
    if let Ok(v) = env::var("container") {
        if v.is_empty() {
            return Virtualization::None;
        }
        return Virtualization::from(v);
    }

    detect_container_files()
}

fn detect_container_files() -> Virtualization {
    match nix::unistd::access("/run/.containerenv", AccessFlags::F_OK) {
        Ok(_) => return Virtualization::Podman,
        Err(e) => {
            log::debug!("access /run/.cantainerenv error: {}", e);
        }
    }

    match nix::unistd::access("/.dockerenv", AccessFlags::F_OK) {
        Ok(_) => return Virtualization::Docker,
        Err(e) => {
            log::debug!("access /.dockerenv error: {}", e);
        }
    }

    Virtualization::None
}
