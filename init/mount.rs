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

use nix::{mount::MsFlags, sys::statfs};
use std::path::Path;

pub fn setup_mount_early() {
    let filesystems = [
        (
            Some("sysfs"),
            "/sys",
            Some("sysfs"),
            MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC | MsFlags::MS_NODEV,
            None,
        ),
        (
            Some("proc"),
            "/proc",
            Some("proc"),
            MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC | MsFlags::MS_NODEV,
            None,
        ),
        (
            Some("devtmpfs"),
            "/dev",
            Some("devtmpfs"),
            MsFlags::MS_NOSUID | MsFlags::MS_STRICTATIME,
            Some("mode=755,size=4m,nr_inodes=64K"),
        ),
        (
            Some("tmpfs"),
            "/run",
            Some("tmpfs"),
            MsFlags::MS_NOSUID | MsFlags::MS_STRICTATIME | MsFlags::MS_NODEV,
            Some("mode=755,size=20%,nr_inodes=800k"),
        ),
    ];

    for (source, target, fstype, flags, data) in filesystems {
        let target = Path::new(target);

        if !target.exists() {
            if let Err(e) = std::fs::create_dir(target) {
                log::warn!("Failed to create mount point {}: {}", target.display(), e);
            }
        }

        if let Err(errno) = statfs::statfs(target) {
            if errno == nix::errno::Errno::ENODEV {
                if let Err(e) = nix::mount::mount(source, target, fstype, flags, data) {
                    log::warn!("Failed to create mount point {}: {}", target.display(), e);
                }
            }
        }
    }

    log::warn!("File systems early mounted successfully.");
}
