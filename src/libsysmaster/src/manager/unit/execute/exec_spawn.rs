use super::exec_base::{ExecCmdError, ExecParameters};
use super::ExecContext;
use crate::manager::unit::unit_entry::Unit;
use crate::manager::unit::unit_rentry::ExecCommand;
use libcgroup;
use log;
use nix::fcntl::FcntlArg;
use nix::unistd::{self, ForkResult, Pid};
use regex::Regex;
use std::path::PathBuf;
use std::process;
use std::rc::Rc;
use std::thread;
use std::time::Duration;
use walkdir::DirEntry;
use walkdir::WalkDir;

use libutils::fd_util;

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
        ctx: Rc<ExecContext>,
    ) -> Result<Pid, ExecCmdError> {
        let ret = unsafe { unistd::fork() };

        match ret {
            Ok(ForkResult::Parent { child }) => {
                log::debug!("child pid is :{}", child);
                libcgroup::cg_attach(child, &unit.cg_path())
                    .map_err(|e| ExecCmdError::CgroupError(e.to_string()))?;
                Ok(child)
            }
            Ok(ForkResult::Child) => {
                thread::sleep(Duration::from_secs(2));
                exec_child(unit, cmdline, params, ctx);
                process::exit(0);
            }
            Err(_e) => Err(ExecCmdError::SpawnError),
        }
    }
}

fn exec_child(unit: &Unit, cmdline: &ExecCommand, params: &ExecParameters, ctx: Rc<ExecContext>) {
    log::debug!("exec context params: {:?}", ctx.envs());

    for (key, value) in ctx.envs() {
        params.add_env(&key, value.to_string());
    }

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

    let mut envs = build_environment(unit, params.fds().len());
    envs.append(&mut params.envs());

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

    log::debug!("exec child envs to execve is: {:?}", envs_cstr);
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
    let cmd = std::ffi::CString::new(cmdline.path().clone()).unwrap();
    let exec_name = std::ffi::CString::new(cmdline.path().clone()).unwrap();

    let mut args = Vec::new();
    args.push(exec_name);

    let var_regex = Regex::new(r"(\$[A-Z_]+)|(\$\{[A-Z_]+\})").unwrap();
    for arg in cmdline.argv() {
        let cap = var_regex.captures(arg);
        if let Some(cap) = cap {
            let match_result = {
                if let Some(mat) = cap.get(1) {
                    Some(mat.as_str())
                } else {
                    cap.get(2).map(|mat| mat.as_str())
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
    let opend_dir = PathBuf::from(format!("/proc/{}/fd", nix::unistd::getpid()));
    for entry in WalkDir::new("/proc/self/fd")
        .min_depth(1)
        .into_iter()
        .filter_entry(|e| !is_valid_fd(e))
    {
        entry.map_or_else(
            |_e| {
                log::error!("walf dir error {:?}", _e);
            },
            |_e| {
                let file_name = _e.file_name().to_str().unwrap();
                let fd = file_name.parse::<i32>().unwrap();
                if fds.contains(&fd) {
                    log::debug!("close file name is {}", file_name);
                    return;
                }

                let link_name = std::fs::read_link(_e.path()).map_or(PathBuf::from(""), |e| e);
                if link_name == opend_dir {
                    log::debug!("not close self opened fd");
                    return;
                }

                log::debug!("socket fds: {:?}, close fd {}", fds, fd);
                fd_util::close(fd);
            },
        );
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

            let nfd = if let Ok(fd) = nix::fcntl::fcntl(fds[i as usize], FcntlArg::F_DUPFD(i + 3)) {
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
    for fd in fds {
        if let Err(_e) = fd_util::fd_nonblock(*fd, false) {
            return false;
        }

        if let Err(_e) = fd_util::fd_cloexec(*fd, false) {
            return false;
        }
    }

    true
}
