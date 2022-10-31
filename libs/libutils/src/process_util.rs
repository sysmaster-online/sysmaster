use nix::errno::errno;
use nix::libc::{kill, ESRCH};
use nix::sys::wait::{waitpid, WaitPidFlag};
use nix::unistd::Pid;
use std::collections::HashSet;
use std::fs::read_dir;
use std::io::{Error, ErrorKind};
use std::path::Path;
use std::thread::sleep;
use std::time::{Duration, SystemTime};

use crate::file_util;

pub fn process_state(pid: Pid) -> Result<char, Error> {
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
        return Err(Error::new(
            ErrorKind::Other,
            "invalid process stat format".to_string(),
        ));
    }

    let p_stat: Vec<char> = stat[3].trim().chars().collect();

    if p_stat.is_empty() {
        return Err(Error::new(
            ErrorKind::Other,
            "invalid process state".to_string(),
        ));
    }
    Ok(p_stat[0])
}

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

pub fn valid_pid(pid: Pid) -> bool {
    if pid <= Pid::from_raw(0) {
        return false;
    }

    true
}

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

pub fn wait_pids(mut pids: HashSet<i32>, timeout: u64) -> HashSet<i32> {
    let now = SystemTime::now();
    let until = now + Duration::from_micros(timeout);

    // remove PID1, we shouldn't wait our self.
    pids.remove(&1);

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
