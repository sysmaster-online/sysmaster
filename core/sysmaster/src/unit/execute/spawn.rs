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

use super::super::entry::Unit;
use basic::fd;
use core::error::*;
use core::exec::{ExecCommand, ExecContext, ExecFlags, ExecParameters, WorkingDirectory};
use nix::fcntl::FcntlArg;
use nix::sys::signal::{pthread_sigmask, SigmaskHow};
use nix::sys::signalfd::SigSet;
use nix::sys::stat::Mode;
use nix::unistd::{self, chroot, setresgid, setresuid, ForkResult, Gid, Pid, Uid};
use regex::Regex;
use std::fs::Permissions;
use std::os::unix::prelude::PermissionsExt;
use std::path::PathBuf;
use std::process;
use std::rc::Rc;
use walkdir::DirEntry;
use walkdir::WalkDir;

pub(in crate::unit) struct ExecSpawn;

impl ExecSpawn {
    pub(in crate::unit) fn new() -> ExecSpawn {
        ExecSpawn
    }

    pub(in crate::unit) fn spawn(
        &self,
        unit: &Unit,
        cmdline: &ExecCommand,
        params: &mut ExecParameters,
        ctx: Rc<ExecContext>,
    ) -> Result<Pid> {
        let ret = unsafe { unistd::fork() };
        params.set_cgroup_path(&unit.cg_path());
        match ret {
            Ok(ForkResult::Parent { child }) => {
                log::debug!("child pid is :{}", child);
                cgroup::cg_attach(child, &unit.cg_path()).context(CgroupSnafu)?;
                Ok(child)
            }
            Ok(ForkResult::Child) => {
                let set = SigSet::empty();
                if pthread_sigmask(SigmaskHow::SIG_SETMASK, Some(&set), None).is_err() {
                    log::info!("Failed to reset the sigmask of child process, ignoring.");
                }
                exec_child(unit, cmdline, params, ctx);
                process::exit(0);
            }
            Err(_e) => Err(Error::SpawnError),
        }
    }
}

fn apply_user_and_group(exec_ctx: Rc<ExecContext>, params: &ExecParameters) -> Result<()> {
    let user = match exec_ctx.user() {
        // ExecParameters.set_user() has already assigned valid user if the configuration is correct
        None => {
            return Err(Error::InvalidData);
        }
        Some(v) => v,
    };
    let group = match exec_ctx.group() {
        None => {
            return Err(Error::InvalidData);
        }
        Some(v) => v,
    };
    // Skip if this is root
    if user.uid == Uid::from_raw(0) && group.gid == Gid::from_raw(0) {
        return Ok(());
    }
    // Careful: set group first, or we may get EPERM when setting group
    log::debug!("Setting process group to {}", group.name);
    setresgid(group.gid, group.gid, group.gid).context(NixSnafu)?;
    // Set environment
    params.add_env("LOGNAME", user.name.clone());
    params.add_env("USER", user.name.clone());
    // Set user
    log::debug!("Setting process user to {}", user.name);
    setresuid(user.uid, user.uid, user.uid).context(NixSnafu)
}

fn apply_root_directory(root_directory: Option<PathBuf>) -> Result<()> {
    let root_directory = match root_directory {
        None => return Ok(()),
        Some(v) => v,
    };
    chroot(&root_directory).context(NixSnafu)
}

fn apply_working_directory(working_directory: WorkingDirectory) -> Result<()> {
    let directory = match working_directory.directory() {
        None => return Ok(()),
        Some(v) => v,
    };
    if let Err(e) = std::env::set_current_dir(directory) {
        if !working_directory.miss_ok() {
            return Err(Error::Io { source: e });
        }
    }
    Ok(())
}

fn setup_exec_directory(exec_ctx: Rc<ExecContext>) -> Result<()> {
    /* Always change the directory's owner, because sysmaster only
     * runs under system mode. */
    let uid = match exec_ctx.user() {
        Some(v) => v.uid,
        None => Uid::from_raw(0),
    };
    let gid = match exec_ctx.group() {
        Some(v) => v.gid,
        None => Gid::from_raw(0),
    };

    let mut directories = Vec::new();
    directories.append(&mut exec_ctx.runtime_directory().directory());
    directories.append(&mut exec_ctx.state_directory().directory());

    for d in &directories {
        /* d should be prefixed already, create it if it doesn't exist */
        if !d.exists() {
            if let Err(e) = std::fs::create_dir_all(d) {
                log::error!("Failed to setup directory {:?}: {}", d, e);
                return Err(Error::Io { source: e });
            }
        }

        if let Err(e) = nix::unistd::chown(d, Some(uid), Some(gid)) {
            log::error!("Failed to set the owner of {:?}: {}", d, e);
            return Err(Error::Nix { source: e });
        }

        /* Hard-coding 755, change this when we implementing XXXDirectoryMode. */
        if let Err(e) = std::fs::set_permissions(d, Permissions::from_mode(0o755)) {
            log::error!("Failed to set the permissions of {:?}: {}", d, e);
            return Err(Error::Io { source: e });
        }
    }
    Ok(())
}

fn apply_umask(umask: Option<Mode>) -> Result<()> {
    let umask = match umask {
        None => {
            return Err(Error::InvalidData);
        }
        Some(v) => v,
    };
    nix::sys::stat::umask(umask);
    Ok(())
}

fn exec_child(unit: &Unit, cmdline: &ExecCommand, params: &ExecParameters, ctx: Rc<ExecContext>) {
    if let Some(cg_path) = params.cgroup_path() {
        if let Err(e) = cgroup::cg_attach(Pid::from_raw(0), &cg_path) {
            log::error!("Failed to migrate forked process to {:?}: {}", cg_path, e);
        }
    }

    log::debug!("exec context params: {:?}", ctx.envs());

    let _ = setup_exec_directory(ctx.clone());

    if let Err(e) = apply_umask(ctx.umask()) {
        log::error!("Failed to apply umask: {}", e.to_string());
        return;
    }

    if let Err(e) = apply_root_directory(ctx.root_directory()) {
        log::error!("Failed to apply root directory: {}", e);
        return;
    }

    #[cfg(feature = "linux")]
    if let Err(e) = apply_user_and_group(ctx.clone(), params) {
        log::error!("Failed to apply user or group: {}", e);
        return;
    }

    if let Err(e) = apply_working_directory(ctx.working_directory()) {
        log::error!("Failed to apply working directory: {}", e);
        return;
    }

    /* Set SELinuxContext */
    if let Some(context) = ctx.selinux_context() {
        let res = basic::security::set_exec_context(&context);
        if res < 0 {
            log::error!("Failed to set selinux exec context");
            if !context.starts_with('-') {
                return;
            }
        }
    }

    if let Err(e) = ctx.load_env_from_file() {
        log::error!("{}", e);
        return;
    }

    for (key, value) in ctx.envs() {
        params.add_env(&key, value.to_string());
    }

    let (cmd, args) = build_run_args(unit, cmdline, params);
    let cstr_args = args
        .iter()
        .map(|cstring| cstring.as_c_str())
        .collect::<Vec<_>>();

    log::debug!(
        "exec child command is: {}, args is: {:?}",
        cmd.to_str().unwrap(),
        args
    );

    let mut envs = build_environment(unit, params);
    envs.append(&mut params.envs());

    log::debug!("exec child env env is: {:?}", envs);

    if let Err(e) = set_all_rlimits(ctx) {
        log::error!("failed to set rlimit: {}", e.to_string());
        return;
    }

    let envs_cstr = envs.iter().map(|v| v.as_c_str()).collect::<Vec<_>>();
    let mut keep_fds = params.fds();

    /* Be careful! We have closed the log fd from here. */
    let ret = close_all_fds(&keep_fds);

    /* Reinit the logger before logging, and set the mode to open_when_needed. */
    log::set_open_when_needed(true);
    log::reinit();
    log::debug!("Logger has been re-enabled.");

    if !ret {
        return;
    }

    if !shift_fds(&mut keep_fds) {
        log::error!("Failed to shift fds");
        return;
    }

    if !flags_fds(&mut keep_fds, params.get_nonblock()) {
        log::error!("Failed to set fd flags");
        return;
    }

    log::debug!("Begin to execute {:?}, args: {:?}", cmd, cstr_args);
    if unistd::execve(&cmd, &cstr_args, &envs_cstr).is_err() {
        log::error!("Failed to execute cmd: {:?}", cmd);
        std::process::exit(1);
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

    let mut args = vec![exec_name];

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

fn build_environment(_unit: &Unit, ep: &ExecParameters) -> Vec<std::ffi::CString> {
    let mut envs = Vec::new();

    let fds = ep.fds().len();
    if fds > 0 {
        envs.push(std::ffi::CString::new(format!("LISTEN_PID={}", nix::unistd::getpid())).unwrap());

        envs.push(std::ffi::CString::new(format!("LISTEN_FDS={}", fds)).unwrap());
    }

    if ep.exec_flags().contains(ExecFlags::SOFT_WATCHDOG) && ep.watchdog_usec() > 0 {
        envs.push(
            std::ffi::CString::new(format!("WATCHDOG_PID={}", nix::unistd::getpid())).unwrap(),
        );

        envs.push(std::ffi::CString::new(format!("WATCHDOG_USEC={}", ep.watchdog_usec())).unwrap());
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
        /* 0,1,2 shouldn't be closed. */
        return true;
    }

    false
}

fn close_all_fds(fds: &[i32]) -> bool {
    let opend_dir = PathBuf::from(format!("/proc/{}/fd", nix::unistd::getpid()));
    for entry in WalkDir::new("/proc/self/fd")
        .min_depth(1)
        .into_iter()
        .filter_entry(|e| !is_valid_fd(e))
    {
        let de = match entry {
            Err(_) => continue,
            Ok(v) => v,
        };

        let file_name = de.file_name().to_str().unwrap();
        let fd = file_name.parse::<i32>().unwrap();
        if fds.contains(&fd) {
            continue;
        }

        let link_name = std::fs::read_link(de.path()).map_or(PathBuf::from(""), |e| e);
        if link_name == opend_dir {
            continue;
        }

        fd::close(fd);
    }
    true
}

fn shift_fds(fds: &mut [i32]) -> bool {
    /* moves fds[i] to i+3 */
    let mut start = 0;
    loop {
        let mut restart = -1;
        for i in start..(fds.len() as i32) {
            if fds[i as usize] == i + 3 {
                continue;
            }

            let nfd = match nix::fcntl::fcntl(fds[i as usize], FcntlArg::F_DUPFD(i + 3)) {
                Err(_) => return false,
                Ok(v) => v,
            };

            fd::close(fds[i as usize]);

            fds[i as usize] = nfd;

            /* i+3 is exactly the right place that fds[i] should shift to. If we fail, retry.
             * Careful: this will leads to a dead loop if i+3 is already used. */
            if nfd != i + 3 && restart < 0 {
                restart = i;
            }
        }
        /* Every fd has been moved to the right place, break and return. */
        if restart < 0 {
            break;
        }
        start = restart;
    }

    true
}

fn flags_fds(fds: &mut Vec<i32>, nonblock: bool) -> bool {
    for fd in fds {
        if fd::fd_nonblock(*fd, nonblock).is_err() {
            return false;
        }

        if fd::fd_cloexec(*fd, false).is_err() {
            return false;
        }
    }
    true
}

fn set_all_rlimits(ctx: Rc<ExecContext>) -> Result<()> {
    ctx.set_all_rlimits()
}
