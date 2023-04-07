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
use crate::error::*;
use crate::file_util;
use nix::errno::errno;
use nix::errno::Errno;
use nix::libc::{kill, ESRCH};
use nix::sys::signal::Signal;
use nix::sys::wait::{waitpid, WaitPidFlag};
use nix::unistd::Pid;
use procfs::process::Stat;
use std::collections::HashSet;
use std::fs::{read_dir, File};
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::{Duration, SystemTime};

///
pub fn process_state(pid: Pid) -> Result<char> {
    if pid == Pid::from_raw(0) || pid == nix::unistd::getpid() {
        return Ok('R');
    }

    let proc_file = format!("/proc/{:?}/stat", pid.as_raw());
    let stat_path = Path::new(&proc_file);
    let first_line = file_util::read_first_line(stat_path)?;
    let stat: Vec<String> = first_line
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    if stat.len() < 3 {
        return Err(Error::Invalid {
            what: "process stat format".to_string(),
        });
    }

    let p_stat: Vec<char> = stat[3].trim().chars().collect();

    if p_stat.is_empty() {
        return Err(Error::Invalid {
            what: "process state".to_string(),
        });
    }
    Ok(p_stat[0])
}

///
pub fn alive(pid: Pid) -> bool {
    if pid < Pid::from_raw(0) {
        return false;
    }

    if pid <= Pid::from_raw(1) {
        return true;
    }

    if pid == nix::unistd::getpid() {
        return true;
    }

    let ret = process_state(pid);
    if ret.is_err() {
        return false;
    }
    if ret.unwrap() == 'Z' {
        return false;
    }

    true
}

///
pub fn valid_pid(pid: Pid) -> bool {
    if pid <= Pid::from_raw(0) {
        return false;
    }

    true
}

///
pub fn kill_all_pids(signal: i32) -> HashSet<i32> {
    let mut pids: HashSet<i32> = HashSet::new();
    let proc_path = Path::new("/proc");
    let read_dir = read_dir(proc_path).unwrap();
    for entry in read_dir.flatten() {
        // Skip files.
        if let Ok(file_type) = entry.file_type() {
            if file_type.is_file() {
                continue;
            }
        }
        let file_name = String::from(entry.file_name().to_str().unwrap());
        // Check pid directory.
        if let Ok(pid_raw) = file_name.parse::<i32>() {
            unsafe {
                log::debug!("killing pid: {} by signal {}", pid_raw, signal);
                kill(pid_raw, signal);
                pids.insert(pid_raw);
            }
        } else {
            continue;
        }
    }
    // return PIDs we want to kill
    pids
}

///
pub fn wait_pids(mut pids: HashSet<i32>, timeout: u64) -> HashSet<i32> {
    let now = SystemTime::now();
    let until = now + Duration::from_micros(timeout);

    // remove PID1, we shouldn't wait our self and init.
    pids.remove(&1);
    pids.remove(&nix::unistd::getpid().into());

    loop {
        // 1. Find killed process by kernel.
        while let Ok(wait_status) = waitpid(Pid::from_raw(-1), Some(WaitPidFlag::WNOHANG)) {
            if let Some(pid) = wait_status.pid() {
                log::debug!("successfully killed pid: {} found by kernel.", pid.as_raw());
                pids.remove(&pid.as_raw());
            } else {
                break;
            }
        }
        // 2. Find killed process by sending sig: 0.
        let mut removed_pids: HashSet<i32> = HashSet::new();
        for pid in &pids {
            unsafe {
                let res = kill(*pid, 0);
                if res == 0 || errno() != ESRCH {
                    continue;
                }
                removed_pids.insert(*pid);
            }
        }
        for pid in removed_pids {
            log::debug!("successfully killed pid: {} found by ourself.", pid);
            pids.remove(&pid);
        }
        // 3. Sleep 1s to wait pid exits.
        sleep(Duration::from_secs(1));
        // 4. Wait or give up.
        if pids.is_empty() {
            break;
        }
        if SystemTime::now() >= until {
            log::info!("some pids haven't been killed yet, stop waiting.");
            break;
        }
    }
    pids
}

/// get the parent pid of the reference pid
fn get_ppid(pid: Pid) -> Result<Pid, Error> {
    if pid == Pid::from_raw(0) || pid == nix::unistd::getpid() {
        return Ok(nix::unistd::getppid());
    }

    let path = PathBuf::from(format!("/proc/{pid}/stat"));

    let stat = Stat::from_reader(File::open(path).context(IoSnafu)?).context(ProcSnafu)?;

    Ok(Pid::from_raw(stat.ppid))
}

/// return true if the pid is the child of calling process, other false.
pub fn my_child(pid: Pid) -> bool {
    if pid.as_raw() <= 1 {
        return false;
    }

    let ppid = get_ppid(pid);

    if let Ok(p) = ppid {
        return p == nix::unistd::getpid();
    }

    false
}

/// send signal to pid, send SIGCONT if the signal is not SIGCONT or SIGKILL
pub fn kill_and_cont(pid: Pid, sig: Signal) -> Result<(), Errno> {
    match nix::sys::signal::kill(pid, sig) {
        Ok(_) => {
            if sig != Signal::SIGCONT && sig != Signal::SIGKILL {
                _ = nix::sys::signal::kill(pid, Signal::SIGCONT);
            }
            Ok(())
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use nix::libc::kill;
    use std::collections::HashSet;
    use std::process::Command;
    use std::thread;

    use crate::process_util::wait_pids;
    #[test]
    fn test_wait_pids() {
        let mut pids: HashSet<i32> = HashSet::new();
        for i in 100..109 {
            let str_i = i.to_string();
            let child = Command::new("/usr/bin/sleep")
                .args([str_i.as_str()])
                .spawn()
                .expect("Failed to fork /usr/bin/sleep");
            pids.insert(child.id() as i32);
        }

        let pids_spawn = pids.clone();
        thread::spawn(move || {
            for pid in pids_spawn {
                unsafe {
                    kill(pid, 15);
                }
            }
        });

        let res = wait_pids(pids, 10000000);
        assert_eq!(res.len(), 0);
    }
}
