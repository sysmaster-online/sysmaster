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

use super::CgFlags;
use crate::error::*;
use crate::CgType;
use basic::do_entry_or_return_io_error;
use basic::CGROUP_SYSMASTER;
use basic::INIT_SCOPE;
use basic::IN_SET;
use basic::SYSMASTER_SLICE;
use nix::libc;
use nix::sys::signal::Signal;
use nix::sys::statfs::{statfs, FsType};
use nix::unistd::Pid;
use std::collections::HashSet;
use std::fs;
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::path::PathBuf;
use walkdir::{DirEntry, WalkDir};

#[cfg(target_env = "musl")]
type FsTypeT = libc::c_ulong;

#[cfg(not(target_env = "musl"))]
type FsTypeT = libc::c_long;

/// the base dir of the cgroup
#[cfg(feature = "linux")]
pub const CG_BASE_DIR: &str = "/sys/fs/cgroup";
#[cfg(feature = "linux")]
const CGROUP_PROCS: &str = "cgroup.procs";

const CG_UNIFIED_DIR: &str = "/sys/fs/cgroup/unified";
const CG_V1_DIR: &str = "/sys/fs/cgroup/sysmaster";

const CG_SYSTEMD_DIR: &str = "/sys/fs/cgroup/systemd";

/// the base dir of the cgroup
#[cfg(feature = "hongmeng")]
pub const CG_BASE_DIR: &str = "/run/sysmaster/cgroup";
#[cfg(feature = "hongmeng")]
const CGROUP_PROCS: &str = "procs";

/// return the cgroup mounted type, if not support cgroup return CgroupErr.
pub fn cg_type() -> Result<CgType> {
    let stat = statfs(CG_BASE_DIR).map_err(|_| Error::NotSupported)?;

    if stat.filesystem_type() == FsType(libc::CGROUP2_SUPER_MAGIC as FsTypeT) {
        return Ok(CgType::UnifiedV2);
    }

    if stat.filesystem_type() != FsType(libc::TMPFS_MAGIC as FsTypeT) {
        return Err(Error::NotSupported);
    }

    if let Ok(s) = statfs(CG_UNIFIED_DIR) {
        if s.filesystem_type() == FsType(libc::CGROUP2_SUPER_MAGIC as FsTypeT) {
            return Ok(CgType::UnifiedV1);
        }
    }

    if let Ok(s) = statfs(CG_V1_DIR) {
        let fy = s.filesystem_type();
        if fy == FsType(libc::CGROUP_SUPER_MAGIC as FsTypeT) {
            return Ok(CgType::Legacy);
        }
    }

    if let Ok(s) = statfs(CG_SYSTEMD_DIR) {
        let fy = s.filesystem_type();
        if fy == FsType(libc::CGROUP_SUPER_MAGIC as FsTypeT) {
            return Ok(CgType::LegacySystemd);
        } else {
            return Ok(CgType::None);
        }
    }

    Err(Error::NotSupported)
}

fn cgtype_to_path(cg_type: CgType) -> &'static str {
    match cg_type {
        CgType::None => "",
        CgType::UnifiedV1 => CG_UNIFIED_DIR,
        CgType::UnifiedV2 => CG_BASE_DIR,
        CgType::Legacy => CG_V1_DIR,
        CgType::LegacySystemd => CG_SYSTEMD_DIR,
    }
}

#[cfg(feature = "linux")]
fn cg_abs_path(cg_path: &Path, suffix: &Path) -> Result<PathBuf> {
    let cg_type = cg_type()?;
    if cg_type == CgType::None {
        return Err(Error::NotFound {
            what: "cgroup is not mounted".to_string(),
        });
    }
    let base_path = cgtype_to_path(cg_type);
    log::debug!("cgroup root path is: {}", base_path);
    let path_buf: PathBuf = PathBuf::from(base_path);
    Ok(path_buf.join(cg_path).join(suffix))
}

#[cfg(feature = "hongmeng")]
fn cg_abs_path(cg_path: &Path, suffix: &Path) -> Result<PathBuf> {
    let path_buf: PathBuf = PathBuf::from(CG_BASE_DIR);
    Ok(path_buf.join(cg_path).join(suffix))
}

/// attach the pid to the controller which is depend the cg_path
pub fn cg_attach(pid: Pid, cg_path: &Path) -> Result<()> {
    log::debug!("attach pid {} to path {:?}", pid, cg_path);
    let cg_procs = cg_abs_path(cg_path, &PathBuf::from(CGROUP_PROCS))?;

    if !cg_procs.exists() {
        return Err(Error::NotFound {
            what: cg_procs.to_string_lossy().to_string(),
        });
    }

    let p = if pid == Pid::from_raw(0) {
        nix::unistd::getpid()
    } else {
        pid
    };

    fs::write(cg_procs, format!("{}\n", p)).context(IoSnafu)?;

    Ok(())
}

/// create the cg_path which is relative to cg_abs_path.
pub fn cg_create(cg_path: &Path) -> Result<()> {
    log::debug!("cgroup create path {:?}", cg_path);
    let abs_cg_path: PathBuf = cg_abs_path(cg_path, &PathBuf::from(""))?;
    do_entry_or_return_io_error!(fs::create_dir_all, abs_cg_path, "create");

    Ok(())
}

/// escape the cg_path which is conflicts with controller name.
pub fn cg_escape(id: &str) -> &str {
    id
}

fn get_pids(cg_path: &Path, item: &str) -> Result<Vec<Pid>> {
    let path = cg_abs_path(cg_path, &PathBuf::from(item))?;
    let file = fs::OpenOptions::new()
        .read(true)
        .open(path)
        .context(IoSnafu)?;

    let reader = BufReader::new(file);
    let mut pids = Vec::new();
    for line in reader.lines() {
        let line = line.context(IoSnafu)?;
        let pid = Pid::from_raw(
            line.trim_matches(|c: char| !c.is_numeric())
                .parse::<i32>()
                .context(ParseIntSnafu)?,
        );

        pids.push(pid);
    }

    Ok(pids)
}

/// return all the pids in the cg_path, read from cgroup.procs.
pub fn cg_get_pids(cg_path: &Path) -> Vec<Pid> {
    match get_pids(cg_path, CGROUP_PROCS) {
        Ok(pids) => pids,
        Err(_) => Vec::new(),
    }
}

fn remove_dir(cg_path: &Path) -> Result<()> {
    if !cg_path.is_absolute() {
        log::error!("We only support remove absolute directory.");
        return Err(Error::NotSupported);
    }
    /* Note: We can't just call fs::remove_all_dir here. This is because /sys/fs/cgroup
     * is a pseudo file system, we can only remove directory, but can't remove regular
     * file. */
    let read_dir = match cg_path.read_dir() {
        Err(e) => {
            log::error!("Failed to read dir: {:?}", cg_path);
            return Err(Error::Io { source: e });
        }
        Ok(v) => v,
    };

    for entry in read_dir {
        let entry = match entry {
            Err(e) => {
                log::error!("Failed to get directory entry: {}", e);
                return Err(Error::Io { source: e });
            }
            Ok(v) => v,
        };
        let entry_file_type = match entry.file_type() {
            Err(e) => {
                log::error!(
                    "Failed to geth the file type of {:?}: {}",
                    entry.file_name(),
                    e
                );
                return Err(Error::Io { source: e });
            }
            Ok(v) => v,
        };
        if !entry_file_type.is_dir() {
            continue;
        }
        remove_dir(&entry.path())?;
    }

    /* Sometimes there are still tasks in cg_path, and rmdir will return EBUSY,
     * we wait 10 us for 10 times. */
    let mut try_times = 0;
    loop {
        let e = match fs::remove_dir(cg_path) {
            Ok(()) => {
                log::debug!("Successfully removed {:?}", cg_path);
                return Ok(());
            }
            Err(e) => e,
        };
        let os_errno = match e.raw_os_error() {
            None => return Err(Error::Io { source: e }),
            Some(v) => v,
        };
        if os_errno == nix::libc::EBUSY && try_times < 10 {
            std::thread::sleep(std::time::Duration::from_micros(10));
            try_times += 1;
            continue;
        }
        log::error!("Failed to remove {:?}: {}", cg_path, e);
        return Err(Error::Io { source: e });
    }
}

fn cg_kill_process(
    cg_path: &Path,
    signal: Signal,
    mut flags: CgFlags,
    pids: HashSet<Pid>,
    item: &str,
) -> Result<()> {
    if IN_SET!(signal, Signal::SIGCONT, Signal::SIGKILL) {
        flags &= !CgFlags::SIGCONT;
    }

    let path = cg_abs_path(cg_path, &PathBuf::from(item))?;
    let file = fs::OpenOptions::new()
        .read(true)
        .open(path)
        .context(IoSnafu)?;

    let cur_pid = nix::unistd::getpid();

    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line.context(IoSnafu)?;
        let pid = Pid::from_raw(
            line.trim_matches(|c: char| !c.is_numeric())
                .parse::<i32>()
                .context(ParseIntSnafu)?,
        );

        if flags.contains(CgFlags::IGNORE_SELF) && cur_pid == pid {
            continue;
        }

        if pids.contains(&pid) {
            continue;
        }

        log::debug!(
            "kill pid {} in cgroup {:?} with signal {}",
            pid,
            cg_path,
            signal
        );
        match nix::sys::signal::kill(pid, signal) {
            Ok(_) => {
                if flags.contains(CgFlags::SIGCONT) {
                    match nix::sys::signal::kill(pid, Signal::SIGCONT) {
                        Ok(_) => {}
                        Err(_) => {
                            log::debug!("send SIGCONT to cgroup process failed");
                        }
                    }
                }
            }
            Err(e) => {
                log::warn!("Failed to kill control service: error: {}", e);
                return Err(Error::KillControlService {
                    what: e.to_string(),
                });
            }
        }
    }

    Ok(())
}

fn cg_kill(cg_path: &Path, signal: Signal, flags: CgFlags, pids: HashSet<Pid>) -> Result<()> {
    cg_kill_process(cg_path, signal, flags, pids, CGROUP_PROCS)?;

    Ok(())
}

/// kill all the process in the cg_path, and remove the dir of the cg_path.
/// cg_path: the controller that will be killed.
/// signal: send signal to the process in the cgroup.
/// flags: the flags that will be operated on the controller.
/// pids: not kill the process which is in the pids.
pub fn cg_kill_recursive(
    cg_path: &Path,
    signal: Signal,
    flags: CgFlags,
    pids: HashSet<Pid>,
) -> Result<()> {
    // kill cgroups
    // todo kill sub cgroups
    cg_kill(cg_path, signal, flags, pids)?;

    if flags.contains(CgFlags::REMOVE) {
        let abs_cg_path = cg_abs_path(cg_path, &PathBuf::from(""))?;
        remove_dir(&abs_cg_path)?;
    }

    Ok(())
}

/// return the supported controllers, read from /proc/cgroups, if failed return the IOError.
pub fn cg_controllers() -> Result<Vec<String>> {
    let file = File::open("/proc/cgroups").context(IoSnafu)?;

    let lines = io::BufReader::new(file).lines();
    let mut controllers = Vec::new();

    for line in lines.flatten() {
        if line.starts_with('#') {
            continue;
        }

        let r: Vec<&str> = line.split_whitespace().collect();
        if r.len() != 4 {
            continue;
        }

        // the controller was disabled
        if r[3] != "1" {
            continue;
        }
        controllers.push(r[0].to_string());
    }

    Ok(controllers)
}

#[allow(dead_code)]
fn cg_read_event(cg_path: &Path, event: &str) -> Result<String> {
    let events_path = cg_abs_path(cg_path, &PathBuf::from("cgroup.events"))?;
    let file = File::open(events_path).context(IoSnafu)?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let content = line.context(IoSnafu)?;
        let words: Vec<String> = content.split_whitespace().map(|c| c.to_string()).collect();

        if words.len() != 2 {
            continue;
        }

        if words[0].trim() != event {
            continue;
        }

        return Ok(words[1].trim().to_string());
    }

    Ok("".to_string())
}

fn cg_is_empty(cg_path: &Path) -> bool {
    let procs_path = cg_abs_path(cg_path, &PathBuf::from(CGROUP_PROCS));
    if procs_path.is_err() {
        return true;
    }

    if !procs_path.unwrap().exists() {
        return true;
    }

    if let Ok(pids) = get_pids(cg_path, CGROUP_PROCS) {
        if pids.is_empty() {
            return true;
        }
    }

    false
}

fn is_dir(entry: &DirEntry) -> bool {
    if entry.file_type().is_dir() {
        return false;
    }

    true
}

/// whether the cg_path cgroup is empty, return true if empty.
#[cfg(feature = "linux")]
pub fn cg_is_empty_recursive(cg_path: &Path) -> Result<bool> {
    if cg_path == Path::new("") || cg_path == Path::new("/") {
        return Ok(true);
    }

    if !cg_is_empty(cg_path) {
        return Ok(false);
    }

    match cg_type()? {
        CgType::UnifiedV1 | CgType::UnifiedV2 => match cg_read_event(cg_path, "populated") {
            Ok(v) => {
                log::debug!("cg read event value:{}", v);
                Ok(v == "0")
            }
            Err(e) => match e {
                Error::NotFound { what: _ } => Ok(true),
                _ => Err(e),
            },
        },
        CgType::Legacy | CgType::LegacySystemd => {
            let cgroup_path = cg_abs_path(cg_path, &PathBuf::from(""))?;

            for entry in WalkDir::new(cgroup_path)
                .min_depth(1)
                .max_depth(1)
                .into_iter()
                .filter_entry(|e| !is_dir(e))
            {
                if entry.is_err() {
                    continue;
                }

                let sub_cg = cg_path.join(entry.unwrap().path());
                if cg_is_empty_recursive(&sub_cg)? {
                    return Ok(false);
                }
            }

            Ok(true)
        }
        CgType::None => Ok(false),
    }
}

/// whether the cg_path cgroup is empty, return true if empty.
#[cfg(feature = "hongmeng")]
pub fn cg_is_empty_recursive(cg_path: &PathBuf) -> Result<bool> {
    if cg_path == &PathBuf::from("") || cg_path == &PathBuf::from("/") {
        return Ok(true);
    }

    if !cg_is_empty(cg_path) {
        return Ok(false);
    }

    let cgroup_path = cg_abs_path(cg_path, &PathBuf::from(""))?;

    for entry in WalkDir::new(cgroup_path)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_entry(|e| !is_dir(e))
    {
        if entry.is_err() {
            continue;
        }

        let sub_cg = cg_path.join(entry.unwrap().path());
        let exist = cg_is_empty_recursive(&sub_cg)?;
        if exist {
            return Ok(false);
        }
    }

    return Ok(true);
}

/// create cgroup path and attach pid to this cgroup
pub fn cg_create_and_attach(cg_path: &Path, pid: Pid) -> Result<bool> {
    cg_create(cg_path)?;

    cg_attach(pid, cg_path)?;

    Ok(true)
}

/// cgroup controller
pub struct CgController {
    /// pid
    pid: Pid,
    /// controller name
    controller: String,
}

impl CgController {
    /// create the controller instance
    pub fn new(controller: &str, pid: Pid) -> Result<Self> {
        let controller = if let Some(str) = controller.strip_prefix("name=") {
            str
        } else {
            controller
        };

        let s = CgController {
            pid,
            controller: controller.to_string(),
        };

        s.valid()?;
        Ok(s)
    }

    /// checks whether valid
    pub fn valid(&self) -> Result<()> {
        self.cg_pid_get_path()?;
        Ok(())
    }

    fn cg_pid_get_path(&self) -> Result<String> {
        self.cg_get_path(self.get_procfs_path())
    }

    fn get_procfs_path(&self) -> String {
        if self.pid.as_raw() == 0 {
            "/proc/self/cgroup".to_string()
        } else {
            format!("/proc/{}/cgroup", self.pid)
        }
    }

    fn cg_get_path<P: AsRef<Path>>(&self, path: P) -> Result<String> {
        let file = fs::OpenOptions::new()
            .read(true)
            .open(path)
            .context(IoSnafu)?;

        let reader = BufReader::new(file);
        for line in reader.lines() {
            let str = match line {
                Ok(str) => str,
                Err(e) => {
                    log::debug!("read line err: {}", e.to_string());
                    return Err(Error::ReadLine {
                        line: e.to_string(),
                    });
                }
            };

            let vec: Vec<&str> = str.split(':').collect();
            if vec.len() != 3 {
                return Err(Error::DataFormat { data: str });
            }
            for ctrl in vec[1].split(',') {
                if ctrl.contains(&self.controller) {
                    return Ok(vec[2].to_string());
                }
            }
        }
        Err(Error::NotFound {
            what: self.controller.to_string(),
        })
    }

    fn cg_strip_suffix<'a>(&self, str: &'a str) -> &'a str {
        if let Some(str) = str.strip_suffix(&format!("/{}", INIT_SCOPE)) {
            return str;
        }

        if let Some(str) = str.strip_suffix(&format!("/{}", SYSMASTER_SLICE)) {
            return str;
        }

        if let Some(str) = str.strip_suffix(&format!("/{}", CGROUP_SYSMASTER)) {
            return str;
        }

        str
    }

    fn cg_get_root_path(&self) -> Result<String> {
        let path = match self.cg_pid_get_path() {
            Ok(str) => str,
            Err(err) => {
                log::debug!("cg_get_root_path err: {}", err);
                return Err(err);
            }
        };
        Ok(self.cg_strip_suffix(&path).to_string())
    }

    fn get_abs_path_by_cgtype(&self, path: &str, cg_type: CgType) -> Result<String> {
        match cg_type {
            CgType::UnifiedV2 => Ok(format!("{}/{}", CG_BASE_DIR, path)),
            CgType::UnifiedV1 => Ok(format!("{}/{}", CG_UNIFIED_DIR, path)),
            CgType::Legacy => Ok(format!("{}/{}/{}", CG_BASE_DIR, self.controller, path)),
            _ => Err(Error::NotSupported),
        }
    }

    /// trim the dir
    pub fn trim(&mut self, delete_root: bool) -> Result<()> {
        let path = self.cg_get_root_path()?;
        let cg_type = cg_type()?;
        let path = self.get_abs_path_by_cgtype(&path, cg_type)?;
        // let path = cg_abs_path(&PathBuf::from(path),&PathBuf::from(""))?;
        Self::cg_remove_dir(&PathBuf::from(path), delete_root)?;
        Ok(())
    }

    fn cg_remove_dir(path: &Path, delete_root: bool) -> Result<()> {
        if !path.exists() {
            return Ok(());
        }

        if !path.is_dir() {
            return Err(Error::NotADirectory {
                path: path.to_string_lossy().to_string(),
            });
        }

        if delete_root {
            fs::remove_dir_all(path).context(IoSnafu)?;
        } else {
            for entry in (fs::read_dir(path).context(IoSnafu)?).flatten() {
                let path = entry.path();
                if path.symlink_metadata().context(IoSnafu)?.is_dir() {
                    Self::cg_remove_dir(&path, true)?;
                } else {
                    fs::remove_file(path).context(IoSnafu)?;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]

mod tests {
    use super::*;
    #[test]
    fn test_cgroup() {
        use crate::CgFlags;
        use nix::sys::signal::Signal;
        use nix::unistd::{fork, ForkResult};
        use std::path::PathBuf;
        use std::thread;
        use std::{collections::HashSet, time::Duration};

        if !nix::unistd::getuid().is_root() {
            println!("Unprivileged users cannot attach process to system.slice, skipping.");
            return;
        }

        let cg_type = if let Ok(cg_type) = super::cg_type() {
            cg_type
        } else {
            println!("cgroup is not supported");
            return;
        };

        let cg_path = PathBuf::from("sysmaster-test.slice");
        let ret = super::cg_create(&cg_path);
        assert!(ret.is_ok());

        let base_path = super::cgtype_to_path(cg_type);
        let path_buf: PathBuf = PathBuf::from(base_path);

        println!("base path is: {:?}", base_path);
        if let Ok(p) = super::cg_abs_path(&cg_path, &PathBuf::from("")) {
            assert_eq!(p, path_buf.join(&cg_path).join(PathBuf::from("")),)
        }

        let t_thread = unsafe { fork() };

        let pid = match t_thread {
            Ok(ForkResult::Parent { child }) => {
                println!("child pid is: {:?}", child);
                let ret = super::cg_attach(child, &cg_path);
                assert!(ret.is_ok());
                child
            }
            Ok(ForkResult::Child) => {
                thread::sleep(Duration::from_secs(78));
                std::process::exit(0);
            }
            Err(_e) => return,
        };

        let pids = super::cg_get_pids(&cg_path);
        assert_ne!(pids.len(), 0);
        assert!(pids.contains(&pid));

        let ret = super::cg_is_empty_recursive(&cg_path);
        assert!(ret.is_ok());
        assert!(!ret.unwrap());

        let ret = super::cg_kill_recursive(
            &cg_path,
            Signal::SIGKILL,
            CgFlags::IGNORE_SELF | CgFlags::REMOVE,
            HashSet::new(),
        );
        assert!(ret.is_ok());
        println!("kill cgroup ret is: {:?}", ret);

        thread::sleep(Duration::from_secs(1));

        let ret = super::cg_is_empty_recursive(&cg_path);
        assert!(ret.is_ok());
        assert!(ret.unwrap());

        let pids = super::cg_get_pids(&cg_path);
        assert_eq!(pids.len(), 0);
        assert!(!pids.contains(&pid));
    }

    #[test]
    fn test_cg_controllers() {
        if !nix::unistd::getuid().is_root() {
            println!("Unprivileged users cannot cg_controllers, skipping.");
            return;
        }
        let ret = super::cg_controllers();
        assert!(ret.is_ok());

        let clist = ret.unwrap();
        assert_ne!(clist.len(), 0);

        println!("supported controllers: {:?}", clist);
        let controllers = [
            "cpuset",
            "cpu",
            "cpuacct",
            "blkio",
            "memory",
            "devices",
            "freezer",
            "net_cls",
            "perf_event",
            "net_prio",
            "hugetlb",
            "pids",
            "rdma",
            "files",
        ];

        for c in clist {
            assert!(controllers.contains(&&c[..]));
        }
    }

    #[test]
    fn test_cgcontrol() {
        let res = std::process::Command::new("/usr/bin/stat")
            .args(["-fc", "%T", "/sys/fs/cgroup"])
            .output()
            .unwrap();
        if res.stdout != "tmpfs".as_bytes() {
            println!("This testcase can only run if /sys/fs/cgroup is under tmpfs, skipping");
            return;
        }

        let controller = match super::cg_type() {
            Ok(_) => "sysmaster",
            Err(_) => "systemd",
        };

        let cg0 = CgController::new(controller, Pid::from_raw(0)).unwrap();
        let cg1 = CgController::new(controller, Pid::from_raw(1)).unwrap();

        assert_eq!(cg0.get_procfs_path(), "/proc/self/cgroup");
        assert_eq!(cg1.get_procfs_path(), "/proc/1/cgroup");

        let path = cg0.cg_get_path(cg0.get_procfs_path()).unwrap();
        assert_ne!(path.len(), 0);

        assert_eq!(
            cg0.cg_strip_suffix(&format!("/test/{}", INIT_SCOPE)),
            "/test"
        );
        assert_eq!(cg0.cg_strip_suffix(&format!("/{}", INIT_SCOPE)), "");
        assert_eq!(
            cg0.cg_strip_suffix(&format!("/test/{}", SYSMASTER_SLICE)),
            "/test"
        );
        assert_eq!(
            cg0.cg_strip_suffix(&format!("/test/{}", CGROUP_SYSMASTER)),
            "/test"
        );

        println!("{}", cg0.cg_get_root_path().unwrap());

        assert_eq!(
            cg0.get_abs_path_by_cgtype("test", CgType::UnifiedV2)
                .unwrap(),
            format!("{}/{}", CG_BASE_DIR, "test")
        );
        assert_eq!(
            cg0.get_abs_path_by_cgtype("test", CgType::UnifiedV1)
                .unwrap(),
            format!("{}/{}", CG_UNIFIED_DIR, "test")
        );
        assert_eq!(
            cg0.get_abs_path_by_cgtype("test", CgType::Legacy).unwrap(),
            format!("{}/{}/{}", CG_BASE_DIR, controller, "test")
        );
    }
}
