use nix::unistd::Pid;
use std::io::{Error, ErrorKind};
use std::path::Path;

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
            format!("invalid process stat format"),
        ));
    }

    let p_stat: Vec<char> = stat[3].trim().chars().collect();

    if p_stat.len() == 0 {
        return Err(Error::new(
            ErrorKind::Other,
            format!("invalid process state"),
        ));
    }
    return Ok(p_stat[0]);
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
