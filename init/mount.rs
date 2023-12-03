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

use nix::mount::{self, MntFlags, MsFlags};
use std::{fs, os::unix::fs::MetadataExt, path::Path};

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
            Some("mode=755,size=4m,nr_inodes=1m"),
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
                println!("Failed to create mount point {}: {}", target.display(), e);
            }
        }

        if is_mount_point(target) {
            // umount first as these filesystemd should be remount
            if let Err(e) = mount::umount2(target, MntFlags::MNT_DETACH) {
                println!("umount2 {} failed:{}", target.display(), e);
                continue;
            }
        }

        if let Err(e) = nix::mount::mount(source, target, fstype, flags, data) {
            println!("Failed to mount {}: {}", target.display(), e);
        }

        println!(
            "Mounting {:?} to {:?} of type {:?} with {:?}",
            source, target, fstype, flags
        );
    }

    println!("File systems early mounted successfully.");
}

fn is_mount_point(path: &Path) -> bool {
    if let Ok(metadata) = fs::metadata(path) {
        let dev_id = metadata.dev();

        let root_dev_id = match fs::metadata("/") {
            Ok(root_metadata) => root_metadata.dev(),
            Err(_) => return false,
        };

        dev_id != root_dev_id
    } else {
        false
    }
}
