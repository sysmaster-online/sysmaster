use super::CgFlags;
use super::CgroupErr;
use nix::sys::signal::Signal;
use nix::sys::statfs::statfs;
use nix::sys::statfs::CGROUP2_SUPER_MAGIC;
use nix::sys::statfs::CGROUP_SUPER_MAGIC;
use nix::sys::statfs::TMPFS_MAGIC;
use nix::unistd::Pid;
use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use utils::IN_SET;

const CG_BASE_DIR: &str = "/sys/fs/cgroup";
const CG_UNIFIED_DIR: &str = "/sys/fs/cgroup/unified";
const CG_V1_DIR: &str = "/sys/fs/cgroup/process1";

#[derive(Debug)]
enum CgType {
    None,
    UnifiedV1,
    UnifiedV2,
    Legacy,
}

fn cg_type() -> Result<CgType, CgroupErr> {
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
    let abs_cg_path: PathBuf = cg_abs_path(cg_path, &PathBuf::from(""))?;
    log::debug!("cgroup create path {:?}", abs_cg_path.clone());
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

mod tests {
    #[test]
    fn test_get_cg_abs_path() {
        use std::path::PathBuf;
        let cg_path = PathBuf::from("system.slice");
        if let Ok(p) = super::cg_abs_path(&cg_path, &PathBuf::from("")) {
            assert_eq!(
                p,
                PathBuf::from("/sys/fs/cgroup/systemd.slice/cgroup.procs")
            )
        }
    }

    #[test]
    fn test_cg_file_type() {
        println!("file type is {:?}", super::cg_type());
    }
}
