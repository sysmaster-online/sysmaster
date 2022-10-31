//! mount the cgroup systems

use bitflags::bitflags;
use libutils::{fs_util, mount_util, path_util, proc_cmdline};
use nix::{
    errno::Errno,
    fcntl::{AtFlags, OFlag},
    mount::MsFlags,
    sys::stat::Mode,
    unistd::AccessFlags,
};
use std::{collections::HashMap, error::Error, fs, path::Path};

use libcgroup::{self, CgType};

const EARLY_MOUNT_NUM: u8 = 4;
const CGROUP_ROOT: &str = "/sys/fs/cgroup/";

type Callback = fn() -> bool;

lazy_static! {
    static ref MOUNT_TABLE: Vec<MountPoint> = {
        let table: Vec<MountPoint> = vec![
        // table.push(MountPoint {
        //     source: String::from("proc"),
        //     target: String::from("/test"),
        //     fs_type: String::from("proc"),
        //     options: None,
        //     flags: MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC | MsFlags::MS_NODEV,
        //     callback: Some(cg_unified_wanted),
        //     mode: MountMode::MNT_FATAL
        // });
        // table.push(MountPoint {
        //     source: String::from("sysfs"),
        //     target: String::from("/sys"),
        //     fs_type: String::from("sysfs"),
        //     options: None,
        //     flags: MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC | MsFlags::MS_NODEV,
        // });
        // table.push(MountPoint {
        //     source: String::from("devtmpfs"),
        //     target: String::from("/dev"),
        //     fs_type: String::from("devtmpfs"),
        //     options: Some("mode=755,size=4m,nr_inodes=64K".to_string()),
        //     flags: MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC | MsFlags::MS_STRICTATIME,
        // });
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
            mode: MountMode::MNT_WRITABLE,
        },

        MountPoint {
            source: String::from("cgroup2"),
            target: String::from("/sys/fs/cgroup"),
            fs_type: String::from("cgroup2"),
            options: Some("nsdelegate".to_string()),
            flags: MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC | MsFlags::MS_NODEV,
            callback: Some(cg_unified_wanted),
            mode: MountMode::MNT_WRITABLE,
        },

        MountPoint {
            source: String::from("cgroup2"),
            target: String::from("/sys/fs/cgroup"),
            fs_type: String::from("cgroup2"),
            options: None,
            flags: MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC | MsFlags::MS_NODEV,
            callback: Some(cg_unified_wanted),
            mode: MountMode::MNT_WRITABLE,
        },

        MountPoint {
            source: String::from("tmpfs"),
            target: String::from("/sys/fs/cgroup"),
            fs_type: String::from("tmpfs"),
            options: Some("mode=755".to_string()),
            flags: MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC | MsFlags::MS_NODEV| MsFlags::MS_STRICTATIME,
            callback: Some(cg_legacy_wanted),
            mode: MountMode::MNT_FATAL,
        },

        MountPoint {
            source: String::from("cgroup2"),
            target: String::from("/sys/fs/cgroup/unified"),
            fs_type: String::from("cgroup2"),
            options: Some("nsdelegate".to_string()),
            flags: MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC | MsFlags::MS_NODEV,
            callback: Some(cg_unifiedv1_wanted),
            mode: MountMode::MNT_WRITABLE,
        },

        MountPoint {
            source: String::from("cgroup2"),
            target: String::from("/sys/fs/cgroup/unified"),
            fs_type: String::from("cgroup2"),
            options: None,
            flags: MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC | MsFlags::MS_NODEV,
            callback: Some(cg_unifiedv1_wanted),
            mode: MountMode::MNT_WRITABLE,
        },

        MountPoint {
            source: String::from("cgroup"),
            target: String::from("/sys/fs/cgroup/sysmaster"),
            fs_type: String::from("cgroup"),
            options: Some("none,name=sysmaster".to_string()),
            flags: MsFlags::MS_NOSUID | MsFlags::MS_NOEXEC | MsFlags::MS_NODEV,
            callback: Some(cg_legacy_wanted),
            mode: MountMode::MNT_WRITABLE,
        }
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

    fn mount(&self) -> Result<(), Errno> {
        if self.callback.is_some() && !self.callback.unwrap()() {
            log::debug!("callback is not satisfied");
            return Ok(());
        }

        log::debug!("check valid mount point: {}", self.target.to_string());
        match self.invalid_mount_point(AtFlags::AT_SYMLINK_FOLLOW) {
            Ok(v) => {
                if v {
                    if self.flags.intersects(MsFlags::MS_REMOUNT) {
                        log::debug!("remount the root mount point for writable");
                        nix::mount::mount::<str, str, str, str>(
                            Some(self.source.as_str()),
                            self.target.as_str(),
                            Some(self.fs_type.as_str()),
                            self.flags,
                            None,
                        )?;
                    }
                    log::debug!("mount point maybe is already mounted");
                    return Ok(());
                }
            }
            Err(e) => {
                log::debug!("invalid mount point errno: {}", e);
                if e != Errno::ENOENT && self.mode.contains(MountMode::MNT_FATAL) {
                    return Err(e);
                }
            }
        }

        log::debug!("create target dir: {}", self.target.to_string());
        fs::create_dir_all(&self.target).map_err(|_e| Errno::EINVAL)?;

        let source = self.source.as_str();
        let target = self.target.as_str();
        let fs_type = self.fs_type.as_str();

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
        nix::mount::mount(Some(source), target, Some(fs_type), self.flags, options)?;

        if let Err(e) = nix::unistd::access(target, AccessFlags::W_OK) {
            nix::mount::umount(target)?;
            fs::remove_dir(Path::new(target)).map_err(|_e| Errno::EBUSY)?;

            return Err(e);
        }

        Ok(())
    }

    fn invalid_mount_point(&self, flags: AtFlags) -> Result<bool, Errno> {
        if path_util::path_equal(&self.target, "/") {
            return Ok(true);
        }

        // todo!()
        // symlink

        let path = Path::new(&self.target);
        let ret = fs_util::open_parent(
            path,
            OFlag::O_PATH | OFlag::O_CLOEXEC,
            Mode::from_bits(0).unwrap(),
        )?;

        let last_file_name = path.file_name().unwrap_or_default();

        let ret = mount_util::mount_point_fd_valid(ret, last_file_name.to_str().unwrap(), flags)?;

        Ok(ret)
    }
}

/// mount the minimal mount point for enable the most basic function
pub fn mount_setup_early() -> Result<(), Errno> {
    for i in 0..EARLY_MOUNT_NUM {
        MOUNT_TABLE[i as usize].mount()?;
    }

    Ok(())
}

/// mount the point of all the mount_table
pub fn mount_setup() -> Result<(), Errno> {
    for i in 0..MOUNT_TABLE.len() {
        MOUNT_TABLE[i as usize].mount()?;
    }

    Ok(())
}

/// mount all the cgroup controller subsystem
pub fn mount_cgroup_controllers() -> Result<(), Box<dyn Error>> {
    if !cg_legacy_wanted() {
        return Ok(());
    }

    let mut controllers = libcgroup::cg_controllers()?;
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

        let target = CGROUP_ROOT.to_string() + &target;
        m_point.set_target(&target);
        m_point.mount()?;

        if pair {
            symlink_controller(target.to_string(), other.to_string()).map_err(|e| {
                format!("create symlink  from {} to {} error: {}", target, other, e)
            })?;
            symlink_controller(target.to_string(), controllers[index].to_string()).map_err(
                |e| {
                    format!(
                        "create symlink  from {} to {} error: {}",
                        target, controllers[index], e
                    )
                },
            )?;
        }

        index += 1;
    }

    nix::mount::mount(
        Some("tmpfs"),
        CGROUP_ROOT,
        Some("tmpfs"),
        MsFlags::MS_REMOUNT
            | MsFlags::MS_NOSUID
            | MsFlags::MS_NOEXEC
            | MsFlags::MS_NODEV
            | MsFlags::MS_STRICTATIME
            | MsFlags::MS_RDONLY,
        Some("mode=755,size=4m,nr_inodes=1k"),
    )?;

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

fn symlink_controller(source: String, alias: String) -> Result<(), Errno> {
    let target = CGROUP_ROOT.to_string() + &alias;
    fs_util::symlink(&source, &target, false)?;
    Ok(())
}

fn cg_unified_wanted() -> bool {
    let cg_ver = libcgroup::cg_type();

    if let Ok(v) = cg_ver {
        return v == CgType::UnifiedV2;
    }

    let ret = proc_cmdline::proc_cmdline_get_bool("sysmaster.unified_cgroup_hierarchy");
    if let Ok(v) = ret {
        return v;
    }

    let ret = proc_cmdline::cmdline_get_value("cgroup_no_v1");
    if let Ok(v) = ret {
        if v.is_some() && v.unwrap() == "all" {
            return true;
        }
    }

    false
}

fn cg_legacy_wanted() -> bool {
    let cg_ver = libcgroup::cg_type();

    if let Ok(v) = cg_ver {
        return v != CgType::UnifiedV2;
    }

    true
}

fn cg_unifiedv1_wanted() -> bool {
    let cg_ver = libcgroup::cg_type();

    if let Ok(v) = cg_ver {
        return v != CgType::UnifiedV2;
    }

    let ret = proc_cmdline::proc_cmdline_get_bool("sysmaster.unified_v1_controller");
    if let Ok(v) = ret {
        return v;
    }

    false
}
