use super::exec_base::{ExecCmdError, ExecCommand, ExecParameters};
use crate::manager::unit::Unit;
use cgroup;
use log;
use nix::fcntl::FcntlArg;
use nix::unistd::{self, ForkResult, Pid};
use regex::Regex;
use std::convert::TryInto;
use std::process;
use std::thread;
use std::time::Duration;
use walkdir::DirEntry;
use walkdir::WalkDir;

use utils::fd_util;

pub(in crate::manager::unit) struct ExecSpawn;

impl ExecSpawn {
    pub(in crate::manager::unit) fn new() -> ExecSpawn {
        ExecSpawn
    }

    pub(in crate::manager::unit) fn spawn(
        &self,
        unit: &Unit,
        cmdline: &ExecCommand,
        params: &ExecParameters,
    ) -> Result<Pid, ExecCmdError> {
        unsafe {
            match unistd::fork() {
                Ok(ForkResult::Parent { child }) => {
                    log::debug!("child pid is :{}", child);
                    cgroup::cg_attach(child, &unit.cg_path())
                        .map_err(|e| ExecCmdError::CgroupError(e.to_string()))?;
                    return Ok(child);
                }
                Ok(ForkResult::Child) => {
                    thread::sleep(Duration::from_secs(2));
                    exec_child(unit, cmdline, params);
                    process::exit(0);
                }
                Err(_e) => return Err(ExecCmdError::SpawnError),
            }
        };
    }
}

fn exec_child(unit: &Unit, cmdline: &ExecCommand, params: &ExecParameters) {
    let (cmd, args) = build_run_args(unit, cmdline, params);
    let cstr_args = args
        .iter()
        .map(|cstring| cstring.as_c_str())
        .collect::<Vec<_>>();

    log::debug!(
        "exec child command is2: {}, args is: {:?}",
        cmd.to_str().unwrap(),
        args
    );

    let envs = build_environment(unit, params.fds().len());
    log::debug!("exec child env env is: {:?}", envs);

    let envs_cstr = envs.iter().map(|v| v.as_c_str()).collect::<Vec<_>>();
    let mut keep_fds = params.fds();

    let ret = close_all_fds(params.fds());
    if !ret {
        log::error!("close all needless fds failed");
        return;
    }

    if !shift_fds(&mut keep_fds) {
        log::error!("shift all fds error");
        return;
    }

    if !flags_fds(&mut keep_fds) {
        log::error!("flags set all fds error");
        return;
    }

    match unistd::execve(&cmd, &cstr_args, &envs_cstr) {
        Ok(_) => {
            log::debug!("execv returned Ok()");
        }
        Err(e) => {
            log::error!("exec child failed: {:?}", e);
            std::process::exit(1);
        }
    }
}

// contrast: build_environment
fn build_run_args(
    _unit: &Unit,
    cmdline: &ExecCommand,
    env: &ExecParameters,
) -> (std::ffi::CString, Vec<std::ffi::CString>) {
    // let command = cmdline.borrow();
    let cmd = std::ffi::CString::new(cmdline.path().clone()).unwrap();

    let exec_name = std::path::PathBuf::from(cmdline.path());
    let exec_name = exec_name.file_name().unwrap().to_str().unwrap();
    let exec_name = std::ffi::CString::new::<Vec<u8>>(exec_name.bytes().collect()).unwrap();

    let mut args = Vec::new();
    args.push(exec_name);

    let var_regex = Regex::new(r"(\$[A-Z_]+)|(\$\{[A-Z_]+\})").unwrap();
    for arg in cmdline.argv() {
        let cap = var_regex.captures(arg);
        if let Some(cap) = cap {
            let match_result = {
                if let Some(mat) = cap.get(1) {
                    Some(mat.as_str())
                } else if let Some(mat) = cap.get(2) {
                    Some(mat.as_str())
                } else {
                    None
                }
            };

            if let Some(val) = match_result {
                let v = val.trim_matches('$').trim_matches('{').trim_matches('}');
                if let Some(target) = env.get_env(v) {
                    args.push(
                        std::ffi::CString::new(var_regex.replace(arg, target).to_string()).unwrap(),
                    );
                };
            }
            continue;
        }

        args.push(std::ffi::CString::new(arg.as_str()).unwrap())
    }

    (cmd, args)
}

fn build_environment(_unit: &Unit, fds: usize) -> Vec<std::ffi::CString> {
    let mut envs = Vec::new();

    if fds > 0 {
        envs.push(std::ffi::CString::new(format!("LISTEN_PID={}", nix::unistd::getpid())).unwrap());

        envs.push(std::ffi::CString::new(format!("LISTEN_FDS={}", fds)).unwrap());
    }
    envs
}

fn is_valid_fd(entry: &DirEntry) -> bool {
    let file_name = entry.file_name().to_str().unwrap();
    let fd = if let Ok(fd) = file_name.parse::<i32>() {
        fd
    } else {
        log::debug!("close fd, filename is not valid fd");
        return true;
    };

    if fd < 3 {
        log::debug!("close fd, filename is not valid fd < 3");
        return true;
    }

    false
}

fn close_all_fds(fds: Vec<i32>) -> bool {
    for entry in WalkDir::new("/proc/self/fd")
        .min_depth(1)
        .into_iter()
        .filter_entry(|e| !is_valid_fd(e))
    {
        let entry = if let Ok(en) = entry {
            en
        } else {
            log::error!("walf dir error: ");
            continue;
        };

        // let entry = entry.unwrap();
        let file_name = entry.file_name().to_str().unwrap();
        log::debug!("close file name is {}", file_name);

        let fd = file_name.parse::<i32>().unwrap();

        if fds.contains(&fd) {
            continue;
        }

        log::debug!("socket fds: {:?}, close fd {}", fds, fd);

        fd_util::close(fd);
    }

    true
}

fn shift_fds(fds: &mut Vec<i32>) -> bool {
    let mut start = 0;
    loop {
        let mut restart = -1;
        for i in start..(fds.len() as i32) {
            if fds[i as usize] == (i as i32) + 3 {
                continue;
            }

            let nfd = if let Ok(fd) = nix::fcntl::fcntl(
                fds[i as usize],
                FcntlArg::F_DUPFD((i + 3).try_into().unwrap()),
            ) {
                fd
            } else {
                return false;
            };

            log::debug!("kill older fd: {}, new fd is: {}", fds[i as usize], nfd);
            fd_util::close(fds[i as usize]);

            fds[i as usize] = nfd;

            if nfd != (i as i32) + 3 && restart < 0 {
                restart = i;
            }
        }

        if restart < 0 {
            break;
        }
        start = restart;
    }

    true
}

fn flags_fds(fds: &mut Vec<i32>) -> bool {
    for i in 0..fds.len() {
        if let Err(_e) = fd_util::fd_nonblock(fds[i], false) {
            return false;
        }

        if let Err(_e) = fd_util::fd_cloexec(fds[i]) {
            return false;
        }
    }

    true
}
