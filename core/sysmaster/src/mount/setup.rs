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

//! mount the cgroup systems
use basic::machine::Machine;
use basic::{cmdline, machine, mount};
use bitflags::bitflags;
use cgroup::{self, CgController, CgType, CG_BASE_DIR};
use core::error::*;
use nix::unistd::Pid;
use nix::{
    errno::Errno,
    fcntl::{AtFlags, OFlag},
    mount::MsFlags,
    sys::stat::Mode,
    unistd::AccessFlags,
};
use std::os::unix::prelude::AsRawFd;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

const EARLY_MOUNT_NUM: u8 = 4;

type Callback = fn() -> bool;

#[cfg(feature = "linux")]
lazy_static! {
    static ref MOUNT_TABLE: Vec<MountPoint> = {
        let table: Vec<MountPoint> = vec![
        MountPoint {
            source: String::from("proc"),
            target: String::from("/proc"),
            fs_type: String::from("proc"),
            options: None,
            flags: MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC | MsFlags::MS_NODEV,
            callback: None,
            mode: MountMode::MNT_FATAL | MountMode::MNT_IN_CONTAINER,
        },
        MountPoint {
            source: String::from("sysfs"),
            target: String::from("/sys"),
            fs_type: String::from("sysfs"),
            options: None,
            flags: MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC | MsFlags::MS_NODEV,
            callback: None,
            mode: MountMode::MNT_FATAL | MountMode::MNT_IN_CONTAINER,
        },
        MountPoint {
            source: String::from("devtmpfs"),
            target: String::from("/dev"),
            fs_type: String::from("devtmpfs"),
            options: Some("mode=755,size=4m,nr_inodes=1m".to_string()),
            flags: MsFlags::MS_NOSUID | MsFlags::MS_STRICTATIME,
            callback: None,
            mode: MountMode::MNT_FATAL | MountMode::MNT_IN_CONTAINER,
        },
        MountPoint {
            source: String::from("tmpfs"),
            target: String::from("/run"),
            fs_type: String::from("tmpfs"),
            options: Some("mode=755,size=20%,nr_inodes=800K".to_string()),
            flags: MsFlags::MS_NOSUID | MsFlags::MS_NODEV | MsFlags::MS_STRICTATIME,
            callback: None,
            mode: MountMode::MNT_FATAL | MountMode::MNT_IN_CONTAINER,
        },
        MountPoint {
            source: String::from("devpts"),
            target: String::from("/dev/pts"),
            fs_type: String::from("devpts"),
            options: Some("mode=620,gid=5".to_string()),
            flags: MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC,
            callback: None,
            mode: MountMode::MNT_IN_CONTAINER,
        },
        MountPoint {
            source: String::from("tmpfs"),
            target: String::from("/dev/shm"),
            fs_type: String::from("tmpfs"),
            options: Some("mode=1777".to_string()),
            flags: MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC | MsFlags::MS_STRICTATIME,
            callback: None,
            mode: MountMode::MNT_FATAL | MountMode::MNT_IN_CONTAINER,
        },
        // table.push(MountPoint {
        //     source: String::from("securityfs"),
        //     target: String::from("/sys/kernel/security"),
        //     fs_type: String::from("securityfs"),
        //     options: None,
        //     flags: MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC | MsFlags::MS_NODEV,
        // });
        // table.push(MountPoint {
        //     source: String::from("tmpfs"),
        //     target: String::from("/dev/shm"),
        //     fs_type: String::from("tmpfs"),
        //     options: Some("1777".to_string()),
        //     flags: MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC | MsFlags::MS_NODEV,
        // });

        // the first remount only for test, will be delete later.
        MountPoint {
            source: String::from("tmpfs"),
            target: String::from("/sys/fs/cgroup"),
            fs_type: String::from("tmpfs"),
            options: None,
            flags: MsFlags::MS_REMOUNT | MsFlags::MS_NOEXEC | MsFlags::MS_NODEV,
            callback: Some(cg_legacy_wanted),
            mode: MountMode::MNT_WRITABLE | MountMode::MNT_IN_CONTAINER,
        },

        MountPoint {
            source: String::from("cgroup2"),
            target: String::from("/sys/fs/cgroup"),
            fs_type: String::from("cgroup2"),
            options: Some("nsdelegate".to_string()),
            flags: MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC | MsFlags::MS_NODEV,
            callback: Some(cg_unified_wanted),
            mode: MountMode::MNT_WRITABLE | MountMode::MNT_IN_CONTAINER,
        },

        MountPoint {
            source: String::from("cgroup2"),
            target: String::from("/sys/fs/cgroup"),
            fs_type: String::from("cgroup2"),
            options: None,
            flags: MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC | MsFlags::MS_NODEV,
            callback: Some(cg_unified_wanted),
            mode: MountMode::MNT_WRITABLE | MountMode::MNT_IN_CONTAINER,
        },

        MountPoint {
            source: String::from("tmpfs"),
            target: String::from("/sys/fs/cgroup"),
            fs_type: String::from("tmpfs"),
            options: Some("mode=755".to_string()),
            flags: MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC | MsFlags::MS_NODEV| MsFlags::MS_STRICTATIME,
            callback: Some(cg_legacy_wanted),
            mode: MountMode::MNT_FATAL | MountMode::MNT_IN_CONTAINER,
        },

        MountPoint {
            source: String::from("cgroup2"),
            target: String::from("/sys/fs/cgroup/unified"),
            fs_type: String::from("cgroup2"),
            options: Some("nsdelegate".to_string()),
            flags: MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC | MsFlags::MS_NODEV,
            callback: Some(cg_unifiedv1_wanted),
            mode: MountMode::MNT_WRITABLE | MountMode::MNT_IN_CONTAINER,
        },

        MountPoint {
            source: String::from("cgroup2"),
            target: String::from("/sys/fs/cgroup/unified"),
            fs_type: String::from("cgroup2"),
            options: None,
            flags: MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC | MsFlags::MS_NODEV,
            callback: Some(cg_unifiedv1_wanted),
            mode: MountMode::MNT_WRITABLE | MountMode::MNT_IN_CONTAINER,
        },

        MountPoint {
            source: String::from("cgroup"),
            target: String::from("/sys/fs/cgroup/sysmaster"),
            fs_type: String::from("cgroup"),
            options: Some("none,name=sysmaster".to_string()),
            flags: MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC | MsFlags::MS_NODEV,
            callback: Some(cg_legacy_wanted),
            mode: MountMode::MNT_WRITABLE | MountMode::MNT_IN_CONTAINER,
        },

        MountPoint {
            source: String::from("cgroup"),
            target: String::from("/sys/fs/cgroup/systemd"),
            fs_type: String::from("cgroup"),
            options: Some("none,name=systemd".to_string()),
            flags: MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC | MsFlags::MS_NODEV,
            callback: Some(cg_legacy_wanted),
            mode: MountMode::MNT_WRITABLE | MountMode::MNT_IN_CONTAINER | MountMode::MNT_NOT_HOST,
        }

        ];
        table
    };
}

#[cfg(feature = "hongmeng")]
lazy_static! {
    static ref MOUNT_TABLE: Vec<MountPoint> = {
        let table: Vec<MountPoint> = vec![
            MountPoint {
                source: String::from("proc"),
                target: String::from("/proc"),
                fs_type: String::from("proc"),
                options: None,
                flags: MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC | MsFlags::MS_NODEV,
                callback: None,
                mode: MountMode::MNT_FATAL,
            },
            MountPoint {
                source: String::from("sysfs"),
                target: String::from("/sys"),
                fs_type: String::from("sysfs"),
                options: None,
                flags: MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC | MsFlags::MS_NODEV,
                callback: None,
                mode: MountMode::MNT_FATAL,
            },
            MountPoint {
                source: String::from("devtmpfs"),
                target: String::from("/dev"),
                fs_type: String::from("devtmpfs"),
                options: Some("mode=755,size=4m,nr_inodes=64K".to_string()),
                flags: MsFlags::MS_NOSUID | MsFlags::MS_STRICTATIME,
                callback: None,
                mode: MountMode::MNT_FATAL,
            },
            MountPoint {
                source: String::from("none"),
                target: String::from(CG_BASE_DIR),
                fs_type: String::from("resmgrfs"),
                options: None,
                flags: MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC | MsFlags::MS_NODEV,
                callback: None,
                mode: MountMode::MNT_WRITABLE,
            },
        ];
        table
    };
}

bitflags! {
    /// the mode of the mount directory
    pub struct MountMode: u8 {
        /// None Mount mode
        const MNT_NONE = 0;
        /// if the flag enabled, the mount will return error for mount failed
        const MNT_FATAL = 1 << 0;
        /// check the mount dir is writable
        const MNT_WRITABLE = 1 << 1;
        /// if the flag enabled, the mount point will be mounted in container
        const MNT_IN_CONTAINER = 1 << 2;
        /// if the flag enabled, the mount point will not be mounted on host
        const MNT_NOT_HOST = 1 << 3;
    }
}

struct MountPoint {
    source: String,
    target: String,
    fs_type: String,
    options: Option<String>,
    flags: MsFlags,
    callback: Option<Callback>,
    mode: MountMode,
}

impl MountPoint {
    fn new(
        source: String,
        target: String,
        fs_type: String,
        options: Option<String>,
        flags: MsFlags,
    ) -> MountPoint {
        MountPoint {
            source,
            target,
            fs_type,
            options,
            flags,
            callback: None,
            mode: MountMode::MNT_NONE,
        }
    }

    fn set_target(&mut self, target: &str) {
        self.target = target.to_string();
    }

    fn mount(&self) -> Result<()> {
        if self.callback.is_some() && !self.callback.unwrap()() {
            log::debug!("callback is not satisfied");
            return Ok(());
        }

        log::debug!("check valid mount point: {}", self.target.to_string());
        match self.invalid_mount_point(AtFlags::AT_SYMLINK_FOLLOW) {
            Ok(v) => {
                if v && self.flags.intersects(MsFlags::MS_REMOUNT) {
                    log::debug!("remount the root mount point for writable");
                    nix::mount::mount::<str, str, str, str>(
                        Some(self.source.as_str()),
                        self.target.as_str(),
                        Some(self.fs_type.as_str()),
                        self.flags,
                        None,
                    )
                    .context(NixSnafu)?;

                    return Ok(());
                } else if v || self.flags.intersects(MsFlags::MS_REMOUNT) {
                    log::debug!("mount point is not mounted but remount flag is set or is already mounted, both ignore it");
                    return Ok(());
                }
            }
            Err(e) => {
                log::debug!("invalid mount point errno: {}", e);
                if let Error::Nix { source } = e {
                    if source != Errno::ENOENT && self.mode.contains(MountMode::MNT_FATAL) {
                        return Err(e);
                    }
                }
            }
        }

        let source = self.source.as_str();
        let target = self.target.as_str();
        let fs_type = self.fs_type.as_str();

        if fs_type == "cgroup" {
            let virtualization = machine::Machine::detect_container();
            // for systemd only mounted on virtualization machine
            if self.mode.contains(MountMode::MNT_NOT_HOST) && virtualization == Machine::None {
                return Ok(());
            }

            // check the controller is exist in /proc/self/cgroup, if not exist, not mounted for virtualization machine
            if virtualization != Machine::None && self.mode.contains(MountMode::MNT_IN_CONTAINER) {
                let t_path = PathBuf::from(target);
                let controller = t_path.file_name().unwrap();

                if let Err(_e) = CgController::new(controller.to_str().unwrap(), Pid::from_raw(0)) {
                    log::warn!("in container, mount cgroup {} but the controller is exist in /proc/self/cgroup, skip it!", target);
                    return Ok(());
                }
            }
        }

        log::debug!("create target dir: {}", self.target.to_string());
        fs::create_dir_all(&self.target).context(IoSnafu)?;

        let options = if self.options.is_none() {
            None
        } else {
            Some(self.options.as_ref().unwrap().as_str())
        };

        log::debug!(
            "mount source: {}, target: {}, type:{}, flags:{:?}, options: {:?}",
            source,
            target,
            fs_type,
            self.flags,
            options
        );
        nix::mount::mount(Some(source), target, Some(fs_type), self.flags, options)
            .context(NixSnafu)?;

        if let Err(e) = nix::unistd::access(target, AccessFlags::W_OK) {
            nix::mount::umount(target).context(NixSnafu)?;
            fs::remove_dir(Path::new(target)).context(IoSnafu)?;
            return Err(Error::Nix { source: e });
        }

        Ok(())
    }

    fn invalid_mount_point(&self, flags: AtFlags) -> Result<bool> {
        if basic::fs::path_equal(&self.target, "/") {
            return Ok(true);
        }

        // todo!()
        // symlink

        let path = Path::new(&self.target);
        let file = basic::fs::open_parent(
            path,
            OFlag::O_PATH | OFlag::O_CLOEXEC,
            Mode::from_bits(0).unwrap(),
        )?;

        let last_file_name = path.file_name().unwrap_or_default();

        let ret =
            mount::mount_point_fd_valid(file.as_raw_fd(), last_file_name.to_str().unwrap(), flags)?;

        Ok(ret)
    }
}

#[allow(dead_code)]
/// need use feature
/// mount the minimal mount point for enable the most basic function
pub fn mount_setup_early() -> Result<()> {
    for i in 0..EARLY_MOUNT_NUM {
        MOUNT_TABLE[i as usize].mount()?;
    }

    Ok(())
}

/// mount the point of all the mount_table except the early mount point
pub fn mount_setup() -> Result<()> {
    for table in MOUNT_TABLE.iter().skip(EARLY_MOUNT_NUM as usize) {
        table.mount()?;
    }

    Ok(())
}

/// mount all the cgroup controller subsystem
pub fn mount_cgroup_controllers() -> Result<()> {
    if !cg_legacy_wanted() {
        return Ok(());
    }

    let mut controllers = cgroup::cg_controllers().context(CgroupSnafu)?;
    let mut index = 0_usize;

    while index < controllers.len() {
        let mut m_point = MountPoint::new(
            "cgroup".to_string(),
            "".to_string(),
            "cgroup".to_string(),
            None,
            MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC | MsFlags::MS_NODEV,
        );

        let pair_con = pair_controller(&controllers[index]);
        let mut pair = false;

        let (target, other) = if let Some(con) = pair_con {
            pair = true;
            for idx in index..controllers.len() {
                if controllers[idx] == con {
                    controllers.remove(idx);
                    break;
                }
            }
            (format!("{},{}", controllers[index], con), con.to_string())
        } else {
            (controllers[index].to_string(), "".to_string())
        };
        m_point.options = Some(target.to_string());
        let target_dir = Path::new(CG_BASE_DIR).join(target);
        let target = target_dir.to_str().expect("invalid cgroup path");
        m_point.set_target(target);
        m_point.mount()?;

        if pair {
            symlink_controller(target.to_string(), other.to_string())?;
            symlink_controller(target.to_string(), controllers[index].to_string())?;
        }

        index += 1;
    }

    nix::mount::mount(
        Some("tmpfs"),
        CG_BASE_DIR,
        Some("tmpfs"),
        MsFlags::MS_REMOUNT
            | MsFlags::MS_NOSUID
            | MsFlags::MS_NOEXEC
            | MsFlags::MS_NODEV
            | MsFlags::MS_STRICTATIME
            | MsFlags::MS_RDONLY,
        Some("mode=755,size=4m,nr_inodes=1k"),
    )
    .context(NixSnafu)?;

    Ok(())
}

#[cfg(feature = "hongmeng")]
/// enable memory controller for sub cgroup
pub fn enable_subtree_control(cg_base_dir: &str) -> Result<()> {
    /* hongmeng doesn't enable cgroup controller for sub cgroup. So when we create a directory under
     * /run/sysmaster/cgroup, i.e. foo.service, the file /run/sysmaster/cgroup/foo.service/controllers
     * is empty. If controllers file is empty, we can't migrate our process to this cgroup. To avoid
     * this problem, we forcely enable memory controller for sub cgroup. */
    let sub_tree_control = Path::new(cg_base_dir).join("subtree_control");
    fs::write(sub_tree_control, "+memory").context(IoSnafu)?;
    Ok(())
}

// return the pair controller which will join with the original controller
fn pair_controller(controller: &str) -> Option<String> {
    let mut pairs = HashMap::new();
    pairs.insert("cpu", "cpuacct");
    pairs.insert("net_cls", "net_prio");

    for (key, val) in pairs {
        if controller == key {
            return Some(val.to_string());
        }

        if controller == val {
            return Some(key.to_string());
        }
    }

    None
}

fn symlink_controller(source: String, alias: String) -> Result<()> {
    let target_path = Path::new(CG_BASE_DIR).join(alias);
    let target = target_path.to_str().unwrap();
    match basic::fs::symlink(&source, target, false) {
        Ok(()) => Ok(()),
        Err(basic::Error::Nix {
            source: Errno::EEXIST,
        }) => Ok(()),
        Err(e) => {
            log::debug!(
                "Failed to symlink controller from {} to {}: {}",
                target,
                source,
                e
            );
            Ok(())
        }
    }
}

fn cg_unified_wanted() -> bool {
    if let Ok(v) = cgroup::cg_type() {
        return v == CgType::UnifiedV2;
    }

    if basic::cmdline::Cmdline::default().has_param("systemd.unified_cgroup_hierarchy") {
        return true;
    }

    let v = cmdline::Cmdline::default().get_param("cgroup_no_v1");
    if v.is_some() && v.unwrap() == "all" {
        return true;
    }

    false
}

fn cg_legacy_wanted() -> bool {
    let cg_ver = cgroup::cg_type();

    if let Ok(v) = cg_ver {
        return v != CgType::UnifiedV2;
    }

    true
}

fn cg_unifiedv1_wanted() -> bool {
    let cg_ver = cgroup::cg_type();

    if let Ok(v) = cg_ver {
        if v == CgType::UnifiedV2 {
            return false;
        }
    }

    cmdline::Cmdline::default().has_param("systemd.unified_v1_controller")
}
