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

//! process functions
use crate::error::*;
use crate::fs;
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
use std::time::{Duration, SystemTime};

const PROCESS_FLAG_POS: usize = 8;
const PF_KTHREAD: u64 = 0x00200000;

/// The `process_state` function in Rust reads the state of a process from the `/proc` filesystem and
/// returns the first character of the process state.
///
/// Arguments:
///
/// * `pid`: The `pid` parameter is of type `Pid`, which represents a process ID. It is used to identify
/// a specific process for which we want to retrieve the process state.
///
/// Returns:
///
/// The function `process_state` returns a `Result<char>`.
pub fn process_state(pid: Pid) -> Result<char> {
    if pid == Pid::from_raw(0) || pid == nix::unistd::getpid() {
        return Ok('R');
    }

    let proc_file = format!("/proc/{:?}/stat", pid.as_raw());
    let stat_path = Path::new(&proc_file);
    let first_line = fs::read_first_line(stat_path)?;
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

/// The `alive` function in Rust checks if a process with a given PID is alive or not.
///
/// Arguments:
///
/// * `pid`: The `pid` parameter in the `alive` function is of type `Pid`. It represents the process ID
/// of a process.
///
/// Returns:
///
/// a boolean value.
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

/// The function `valid_pid` checks if a given process ID (`pid`) is valid or not.
///
/// Arguments:
///
/// * `pid`: The `pid` parameter is of type `Pid`.
///
/// Returns:
///
/// a boolean value.
pub fn valid_pid(pid: Pid) -> bool {
    if pid <= Pid::from_raw(0) {
        return false;
    }

    true
}

/// The function `kill_all_pids` in Rust iterates through the `/proc` directory and kills all processes
/// by sending a specified signal, returning a set of the killed process IDs.
///
/// Arguments:
///
/// * `signal`: The `signal` parameter is an integer representing the signal that will be sent to each
/// process. Signals are used in Unix-like operating systems to communicate with processes and can be
/// used for various purposes such as terminating a process, interrupting a process, or requesting a
/// process to reload its configuration.
///
/// Returns:
///
/// The function `kill_all_pids` returns a `HashSet` containing the PIDs (Process IDs) of the processes
/// that were killed.
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
            if let Ok(true) = ignore_proc_during_shutdown(Pid::from_raw(pid_raw)) {
                continue;
            }
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

/// The `wait_pids` function waits for a set of process IDs to be killed either by the kernel or by
/// sending a signal, with a specified timeout.
///
/// Arguments:
///
/// * `pids`: A HashSet of process IDs (pids) that need to be waited for.
/// * `timeout`: The `timeout` parameter is the maximum amount of time (in microseconds) to wait for the
/// processes to be killed.
///
/// Returns:
///
/// The function `wait_pids` returns a `HashSet<i32>`.
pub fn wait_pids(mut pids: HashSet<i32>, timeout: u64) -> HashSet<i32> {
    let now = SystemTime::now();
    let until = now + Duration::from_micros(timeout);

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
        if pids.is_empty() {
            break;
        }

        // 3. Wait or give up.
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

    let path = PathBuf::from(format!("/proc/{}/stat", pid));

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
                let _ = nix::sys::signal::kill(pid, Signal::SIGCONT);
            }
            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn ignore_proc_during_shutdown(pid: Pid) -> Result<bool> {
    if pid <= Pid::from_raw(1) {
        return Ok(true);
    }

    if pid == nix::unistd::getpid() {
        return Ok(true);
    }

    if is_kernel_thread(pid)? {
        return Ok(true);
    }

    Ok(false)
}

fn is_kernel_thread(pid: Pid) -> Result<bool> {
    if pid == Pid::from_raw(1) || pid == nix::unistd::getpid() {
        return Ok(false);
    }

    if pid <= Pid::from_raw(0) {
        return Err(Error::Invalid {
            what: format!("Invalid pid: {}", pid),
        });
    }

    let first_line = fs::read_first_line(Path::new(&format!("/proc/{}/stat", pid.as_raw())))?;
    let stat: Vec<String> = first_line
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    if stat.len() <= PROCESS_FLAG_POS {
        return Err(Error::Invalid {
            what: "process stat format".to_string(),
        });
    }

    let flag: u64 = stat[PROCESS_FLAG_POS].parse()?;
    if flag & PF_KTHREAD != 0 {
        Ok(true)
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use nix::libc::kill;
    use std::collections::HashSet;
    use std::process::Command;
    use std::thread;

    use crate::process::wait_pids;
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

    #[test]
    fn test_ignore_proc_during_shutdown() {
        assert!(
            crate::process::ignore_proc_during_shutdown(nix::unistd::Pid::from_raw(0))
                .unwrap_or(false)
        );
        if let Ok(ignore) =
            crate::process::ignore_proc_during_shutdown(nix::unistd::Pid::from_raw(1))
        {
            assert!(ignore);
        }
        if let Ok(ignore) = crate::process::ignore_proc_during_shutdown(nix::unistd::getpid()) {
            assert!(ignore);
        }
        if let Ok(mut child) = Command::new("/usr/bin/sleep").arg("2").spawn() {
            if let Ok(ignore) = crate::process::ignore_proc_during_shutdown(
                nix::unistd::Pid::from_raw(child.id().try_into().unwrap()),
            ) {
                assert!(!ignore);
            }
            child.wait().unwrap();
        }
    }
}
