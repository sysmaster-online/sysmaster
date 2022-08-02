use crate::CgType;

use super::CgFlags;
use super::CgroupErr;
use nix::sys::signal::Signal;
use nix::sys::statfs::statfs;
use nix::sys::statfs::CGROUP2_SUPER_MAGIC;
use nix::sys::statfs::CGROUP_SUPER_MAGIC;
use nix::sys::statfs::TMPFS_MAGIC;
use nix::unistd::Pid;
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader};
use std::io::{Error as IOError, ErrorKind};
use std::path::PathBuf;
use walkdir::{DirEntry, WalkDir};

use utils::IN_SET;

const CG_BASE_DIR: &str = "/sys/fs/cgroup";
const CG_UNIFIED_DIR: &str = "/sys/fs/cgroup/unified";
const CG_V1_DIR: &str = "/sys/fs/cgroup/process1";

pub fn cg_type() -> Result<CgType, CgroupErr> {
    // 查询cgroup的版本类型
    let stat = if let Ok(s) = statfs(CG_BASE_DIR) {
        s
    } else {
        return Err(CgroupErr::NotSupported);
    };

    if stat.filesystem_type() == CGROUP2_SUPER_MAGIC {
        return Ok(CgType::UnifiedV2);
    }

    if stat.filesystem_type() == TMPFS_MAGIC {
        match statfs(CG_UNIFIED_DIR) {
            Ok(s) => {
                if s.filesystem_type() == CGROUP2_SUPER_MAGIC {
                    return Ok(CgType::UnifiedV1);
                }
            }
            Err(_) => {}
        }

        match statfs(CG_V1_DIR) {
            Ok(s) => {
                let fy = s.filesystem_type();
                if fy == CGROUP2_SUPER_MAGIC {
                    return Ok(CgType::UnifiedV1);
                } else if fy == CGROUP_SUPER_MAGIC {
                    return Ok(CgType::Legacy);
                } else {
                    return Ok(CgType::None);
                }
            }
            Err(_) => {
                return Err(CgroupErr::NotSupported);
            }
        }
    }

    return Err(CgroupErr::NotSupported);
}

fn cgtype_to_path(cg_type: CgType) -> &'static str {
    match cg_type {
        CgType::None => "",
        CgType::UnifiedV1 => CG_UNIFIED_DIR,
        CgType::UnifiedV2 => CG_BASE_DIR,
        CgType::Legacy => CG_V1_DIR,
    }
}

fn cg_abs_path(cg_path: &PathBuf, suffix: &PathBuf) -> Result<PathBuf, CgroupErr> {
    let cg_type = cg_type()?;
    let base_path = cgtype_to_path(cg_type);
    let path_buf: PathBuf = PathBuf::from(base_path);
    return Ok(path_buf.join(cg_path).join(suffix));
}

pub fn cg_attach(pid: Pid, cg_path: &PathBuf) -> Result<(), CgroupErr> {
    log::debug!("attach pid {} to path {:?}", pid, cg_path);
    let cg_procs = cg_abs_path(cg_path, &PathBuf::from("cgroup.procs"))?;

    if !cg_procs.exists() {
        return Err(CgroupErr::IoError(std::io::Error::from(
            std::io::ErrorKind::NotFound,
        )));
    }

    fs::write(cg_procs, format!("{}\n", pid.to_string())).map_err(|e| CgroupErr::IoError(e))?;

    Ok(())
}

pub fn cg_create(cg_path: &PathBuf) -> Result<(), CgroupErr> {
    log::debug!("cgroup create path {:?}", cg_path);
    let abs_cg_path: PathBuf = cg_abs_path(cg_path, &PathBuf::from(""))?;
    fs::create_dir_all(abs_cg_path.clone()).map_err(|e| CgroupErr::IoError(e))?;

    Ok(())
}

pub fn cg_escape(id: &str) -> &str {
    // 系统默认的文件目录冲突时，添加前缀字符
    return id;
}

fn get_pids(cg_path: &PathBuf, item: &str) -> Result<Vec<Pid>, CgroupErr> {
    let path = cg_abs_path(cg_path, &PathBuf::from(item))?;
    let file = fs::OpenOptions::new()
        .read(true)
        .open(path)
        .map_err(|e| CgroupErr::IoError(e))?;

    let reader = BufReader::new(file);
    let mut pids = Vec::new();
    for line in reader.lines() {
        let line = line.map_err(|e| CgroupErr::IoError(e))?;
        let pid = Pid::from_raw(line.parse::<i32>().unwrap());

        pids.push(pid);
    }

    Ok(pids)
}

pub fn cg_get_pids(cg_path: &PathBuf) -> Vec<Pid> {
    match get_pids(cg_path, "cgroup.procs") {
        Ok(pids) => pids,
        Err(_) => Vec::new(),
    }
}

fn remove_dir(cg_path: &PathBuf) -> Result<(), CgroupErr> {
    let abs_cg_path: PathBuf = cg_abs_path(cg_path, &PathBuf::from(""))?;

    match fs::remove_dir_all(abs_cg_path.clone()) {
        Ok(_) => {}
        Err(e) => {
            log::debug!(
                "remove dir failed：{:?}, err: {}",
                abs_cg_path,
                e.to_string()
            );
        }
    }

    Ok(())
}

fn cg_kill_process(
    cg_path: &PathBuf,
    signal: Signal,
    mut flags: CgFlags,
    pids: HashSet<Pid>,
    item: &str,
) -> Result<(), CgroupErr> {
    if IN_SET!(signal, Signal::SIGCONT, Signal::SIGKILL) {
        flags &= !CgFlags::SIGCONT;
    }

    let path = cg_abs_path(cg_path, &PathBuf::from(item))?;
    let file = fs::OpenOptions::new()
        .read(true)
        .open(path)
        .map_err(|e| CgroupErr::IoError(e))?;

    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line.map_err(|e| CgroupErr::IoError(e))?;
        let pid = Pid::from_raw(line.parse::<i32>().unwrap());

        if pids.contains(&pid) {
            continue;
        }

        log::debug!("kill pid {} in cgroup.procs", pid);
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
                return Err(CgroupErr::KillError(e));
            }
        }
    }

    Ok(())
}

fn cg_kill(
    cg_path: &PathBuf,
    signal: Signal,
    flags: CgFlags,
    pids: HashSet<Pid>,
) -> Result<(), CgroupErr> {
    cg_kill_process(cg_path, signal, flags, pids, "cgroup.procs")?;

    Ok(())
}

pub fn cg_kill_recursive(
    cg_path: &PathBuf,
    signal: Signal,
    flags: CgFlags,
    pids: HashSet<Pid>,
) -> Result<(), CgroupErr> {
    // kill cgroups
    // todo kill sub cgroups
    cg_kill(cg_path, signal, flags, pids)?;

    if flags.contains(CgFlags::REMOVE) {
        match remove_dir(cg_path) {
            Ok(_) => {}
            Err(e) => {
                return Err(e);
            }
        }
    }

    Ok(())
}

pub fn cg_controllers() -> Result<Vec<String>, IOError> {
    let file = match File::open("/proc/cgroups") {
        Err(why) => {
            return Err(IOError::new(
                ErrorKind::Other,
                format!("Error: Open file failed detail {}{}!", why, "/proc/cgroups"),
            ))
        }
        Ok(file) => file,
    };

    let lines = io::BufReader::new(file).lines();
    let var_regex = Regex::new(r"([a-zA-Z_]+)\s+(\d+)\s+(\d+)\s+(\d+)").unwrap();
    let mut controllers = Vec::new();

    lines.for_each(|tmp| {
        if let Ok(line) = tmp {
            let cap = var_regex.captures(&line);
            if let Some(cap) = cap {
                let match_result = {
                    if let Some(mat) = cap.get(1) {
                        Some(mat.as_str())
                    } else {
                        None
                    }
                };

                if let Some(val) = match_result {
                    controllers.push(val.to_string());
                }
            }
        }
    });

    Ok(controllers)
}

fn cg_read_event(cg_path: &PathBuf, event: &str) -> Result<String, CgroupErr> {
    let events_path = cg_abs_path(cg_path, &PathBuf::from("cgroup.events"))?;
    let file = File::open(events_path).map_err(|e| CgroupErr::IoError(e))?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let content = line.map_err(|e| CgroupErr::IoError(e))?;
        let words: Vec<String> = content
            .trim()
            .split_whitespace()
            .map(|c| c.to_string())
            .collect();

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

fn cg_is_empty(cg_path: &PathBuf) -> bool {
    let procs_path = cg_abs_path(cg_path, &PathBuf::from("cgroup.procs"));
    if procs_path.is_err() {
        return true;
    }

    if !procs_path.unwrap().exists() {
        return true;
    }

    if let Ok(pids) = get_pids(cg_path, "cgroup.procs") {
        if pids.len() == 0 {
            return true;
        }
    }

    return false;
}

fn is_dir(entry: &DirEntry) -> bool {
    if entry.file_type().is_dir() {
        return true;
    }

    return false;
}

pub fn cg_is_empty_recursive(cg_path: &PathBuf) -> Result<bool, CgroupErr> {
    if cg_path == &PathBuf::from("") || cg_path == &PathBuf::from("/") {
        return Ok(true);
    }

    let cg_type = cg_type()?;
    if cg_type <= CgType::Legacy {
        return Ok(false);
    }

    if cg_type == CgType::UnifiedV2 || cg_type == CgType::UnifiedV1 {
        match cg_read_event(cg_path, "populated") {
            Ok(v) => {
                log::debug!("cg read event value:{}", v.to_string());
                return Ok(v == "0");
            }
            Err(e) => match e {
                CgroupErr::IoError(_e) => {
                    if _e.kind() == ErrorKind::NotFound {
                        return Ok(true);
                    }
                }
                _ => return Err(e),
            },
        }
    } else {
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

    Ok(false)
}

mod tests {
    #[test]
    fn test_cg_create() {
        use nix::unistd::Pid;
        use std::io::ErrorKind;
        use std::path::PathBuf;

        let cg_type = if let Ok(cg_type) = super::cg_type() {
            cg_type
        } else {
            println!("cgroup is not supported");
            return;
        };

        let cg_path = PathBuf::from("system.slice");
        if let Err(e) = super::cg_create(&cg_path) {
            match e {
                crate::CgroupErr::IoError(err) => {
                    if err.kind() == ErrorKind::PermissionDenied {
                        println!("no permission to create cgroup");
                        return;
                    }
                }
                _ => {}
            }
        }

        let base_path = super::cgtype_to_path(cg_type);
        let path_buf: PathBuf = PathBuf::from(base_path);
        if let Ok(p) = super::cg_abs_path(&cg_path, &PathBuf::from("")) {
            assert_eq!(p, path_buf.join(&cg_path).join(&PathBuf::from("")),)
        }

        if let Err(_e) = super::cg_attach(Pid::from_raw(1), &cg_path) {
            println!("attach failed");
            return;
        }

        let pids = super::cg_get_pids(&cg_path);
        assert_ne!(pids.len(), 0);

        if let Err(_e) = super::cg_attach(Pid::from_raw(1), &PathBuf::from("")) {
            println!("attach pid to root cgroup, error: {:?}", _e);
            return;
        }

        let pids = super::cg_get_pids(&cg_path);
        assert_eq!(pids.len(), 0);
    }

    #[test]
    fn test_cg_file_type() {
        println!("file type is {:?}", super::cg_type());
    }

    #[test]
    fn test_cg_controllers() {
        let ret = super::cg_controllers();

        assert_ne!(ret.is_err(), true);

        for c in ret.unwrap() {
            println!("cgroup controller: {}", c);
        }
    }
}
