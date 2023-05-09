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

//! sysmaster-core bin
mod job;
mod keep_alive;
mod manager;
///

/// dependency:
///    [manager]
///       ↑
///[reliability] → [unit   job  plugin]
///            ↖  ↗
///            [butil]
///
// mount not to be here;
mod mount;
mod plugin;
mod unit;
mod utils;

#[macro_use]
extern crate lazy_static;
extern crate clap;
use crate::keep_alive::KeepAlive;
use crate::manager::config::ManagerConfig;
use crate::manager::{Action, Manager, Mode, MANAGER_ARGS_SIZE_MAX};
use crate::mount::setup;
use basic::logger::{self};
use basic::process_util;
use clap::Parser;
use libc::{c_int, getppid, prctl, PR_SET_CHILD_SUBREAPER};
use log::{self};
use nix::sys::resource::{self, Resource};
use nix::sys::signal::{self, SaFlags, SigAction, SigHandler, SigSet, Signal};
use std::convert::TryFrom;
use std::env::{self};
use std::os::unix::process::CommandExt;
use std::process::{exit, Command};
use std::rc::Rc;
use sysmaster::error::*;
use sysmaster::rel;

use constants::SIG_MANAGER_REEXEC_OFFSET;
use constants::SIG_SWITCH_ROOT_OFFSET;

/// parse program arguments
#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    #[clap(long)]
    /// Reload the configuration.
    deserialize: bool,
}

fn main() -> Result<()> {
    //------------------------Code placed at the top-----------------------------

    /* systemd use reset_all_signal_handlers, we can't because sysmaster is not PID1.
     * We choose to ignore all signals explicitly, and then register signals we are
     * interested in. But there are still some differences, see docs/man/signal.md
     * for details. */
    ignore_all_signals();

    // The registration signal is at the beginning and has the highest priority!
    register_reexec_signal(true);

    // Connect init.
    KeepAlive::init();
    //---------------------------------------------------------------------------

    let args = Args::parse();

    let manager_config = Rc::new(ManagerConfig::new(None));
    let log_file = if manager_config.LogFile.is_empty() {
        None
    } else {
        Some(manager_config.LogFile.as_str())
    };
    logger::init_log(
        "sysmaster",
        manager_config.LogLevel,
        &manager_config.LogTarget,
        log_file,
    );
    log::info!("sysmaster running in system mode.");

    // temporary annotation for repeat mount

    // mount_setup::mount_setup_early().map_err(|e| {
    //     log::error!("failed to mount early mount point, errno: {}", e);
    //     format!("failed to mount early mount point, errno: {}", e)
    // })?;

    setup::mount_setup()?;

    rel::reli_dir_prepare()?;
    let switch = rel::reli_debug_get_switch();
    log::info!("sysmaster initialize with switch: {}.", switch);

    initialize_runtime(switch)?;

    let manager = Manager::new(Mode::System, Action::Run, manager_config);

    // enable clear, mutex with install_crash_handler
    if !switch {
        if !args.deserialize {
            manager.debug_clear_restore();
        }
        // if switch is false unregister init's reexec signal.
        register_reexec_signal(false);
        log::info!("debug: clear data restored.");
    }

    manager.setup_cgroup()?;

    // startup
    manager.startup(args.deserialize)?;

    // main loop
    let ret = manager.main_loop();
    log::info!("sysmaster end its main loop with result: {:?}", ret);

    // get result
    let reexec = ret.map_or(false, |ree| ree);

    // re-exec
    if reexec {
        let args: Vec<String> = env::args().collect();
        do_reexecute(&args, true);
    }

    Ok(())
}

fn initialize_runtime(switch: bool) -> Result<()> {
    if switch {
        install_crash_handler();
        log::info!("install crash handler.");
    }

    #[cfg(feature = "linux")]
    setup::mount_cgroup_controllers().map_err(|_| Error::Other {
        msg: "mount cgroup controllers failed: {e}".to_string(),
    })?;

    set_child_reaper();
    install_switch_root_handler();

    Ok(())
}

fn set_child_reaper() {
    let ret = unsafe { prctl(PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) };

    if ret < 0 {
        log::warn!("failed to set child reaper, errno: {}", ret);
    }
}

fn do_reexecute(args: &Vec<String>, reload: bool) {
    let args_size = args.len().max(MANAGER_ARGS_SIZE_MAX);

    let path;
    let mut argv = [].to_vec();
    if args.is_empty() {
        (path, argv) = execarg_build_default();
    } else {
        path = args[0].clone();
        if args.len() >= 2 {
            argv = args[1..].to_vec();
        }
    }

    // Remove '--deserialize' from the previous parameter first, as this may be a fault recovery start.
    for index in argv.iter().enumerate() {
        if index.1 == "--deserialize" {
            argv.remove(index.0);
            break;
        }
    }

    if reload {
        argv.push("--deserialize".to_string());
    }

    assert!(argv.len() <= args_size);
    println!("do_reexecute path:{:?} argv:{:?}", path, argv);

    let mut command = Command::new(&path);
    command.args(&argv);
    let comm = command.env("MANAGER", format!("{}", unsafe { libc::getpid() }));
    let err = comm.exec();
    match err.raw_os_error() {
        Some(e) => {
            log::error!("MANAGER exit err:{:?}", e);
            exit(e);
        }
        None => exit(0),
    }
}

fn install_crash_handler() {
    let signals = vec![
        Signal::SIGSEGV,
        Signal::SIGILL,
        Signal::SIGFPE,
        Signal::SIGBUS,
        Signal::SIGQUIT,
        Signal::SIGABRT,
        Signal::SIGSYS,
    ];
    let handler = SigHandler::Handler(crash);
    let flags = SaFlags::SA_NODEFER;
    let action = SigAction::new(handler, flags, SigSet::empty());
    for &signal in signals.iter() {
        unsafe {
            signal::sigaction(signal, &action).expect("failed to set signal handler for crash")
        };
    }
}

extern "C" fn crash(signo: c_int) {
    let _signal = Signal::try_from(signo).unwrap(); // debug

    let args: Vec<String> = env::args().collect();
    do_reexecute(&args, false);
}

fn execarg_build_default() -> (String, Vec<String>) {
    let path = env::current_exe().unwrap();
    let str_path = String::from(path.to_str().unwrap());

    let mut argv = [].to_vec();
    let args: Vec<String> = env::args().collect();
    if args.len() >= 2 {
        argv = args[1..].to_vec();
    }
    (str_path, argv)
}

extern "C" fn crash_reexec(_signo: c_int, siginfo: *mut libc::siginfo_t, _con: *mut libc::c_void) {
    unsafe {
        if (*siginfo).si_pid() == getppid() {
            let args: Vec<String> = env::args().collect();
            do_reexecute(&args, false);
        }
    };
}

extern "C" fn crash_none(_signo: c_int, _siginfo: *mut libc::siginfo_t, _con: *mut libc::c_void) {
    // nothing to do.
}

fn register_reexec_signal(enable: bool) {
    let manager_signal: signal::Signal =
        unsafe { std::mem::transmute(libc::SIGRTMIN() + SIG_MANAGER_REEXEC_OFFSET) };
    let handler = match enable {
        true => SigHandler::SigAction(crash_reexec),
        false => SigHandler::SigAction(crash_none),
    };
    let flags = SaFlags::SA_NODEFER;
    let action = SigAction::new(handler, flags, SigSet::empty());

    unsafe { signal::sigaction(manager_signal, &action).expect("failed to set signal handler") };
}

fn install_switch_root_handler() {
    let manager_signal: signal::Signal =
        unsafe { std::mem::transmute(libc::SIGRTMIN() + SIG_SWITCH_ROOT_OFFSET) };
    let handler = SigHandler::Handler(switch_root);
    let flags = SaFlags::SA_NODEFER;
    let action = SigAction::new(handler, flags, SigSet::empty());

    unsafe {
        signal::sigaction(manager_signal, &action)
            .expect("failed to set switch_root signal handler")
    };
}

extern "C" fn switch_root(sig: libc::c_int) {
    if sig != libc::SIGRTMIN() + SIG_SWITCH_ROOT_OFFSET {
        return;
    }
    log::info!("sysMaster switch root");
    let mut pids = process_util::kill_all_pids(15);
    pids = process_util::wait_pids(pids, 10000000);

    if !pids.is_empty() {
        pids = process_util::kill_all_pids(9);
        process_util::wait_pids(pids, 10000000);
    }

    reset_all_signal_handlers();
    reset_signal_mask();
    rlimit_nofile_safe();
    exit(0);
}

fn rlimit_nofile_safe() {
    let mut limit = match resource::getrlimit(Resource::RLIMIT_NOFILE) {
        Ok(limit) => limit,
        Err(e) => {
            log::warn!("Failed to query RLIMIT_NOFILE: {}", e);
            return;
        }
    };

    log::info!("limit: {}, {}", limit.0, limit.1);
    if limit.0 <= nix::sys::select::FD_SETSIZE as u64 {
        return;
    }

    limit.0 = nix::sys::select::FD_SETSIZE as u64;

    if let Err(e) = resource::setrlimit(Resource::RLIMIT_NOFILE, limit.0, limit.1) {
        log::warn!("Failed to set RLIMIT_NOFILE: {}", e);
    }
}

fn reset_all_signal_handlers() {
    for sig in nix::sys::signal::Signal::iterator() {
        /* SIGKILL and SIGSTOP is invalid, see sigaction(2) */
        if sig == Signal::SIGKILL || sig == Signal::SIGSTOP {
            continue;
        }
        let flags = SaFlags::SA_RESTART;
        let sig_handler = SigHandler::SigDfl;
        let sig_action = SigAction::new(sig_handler, flags, SigSet::empty());
        unsafe {
            if let Err(e) = signal::sigaction(sig, &sig_action) {
                log::warn!("Failed to reset signal {}: {e}", sig);
            }
        }
    }
}

fn ignore_all_signals() {
    /* nix::sys::signal::Signal doesn't support SIGRTMAX, use libc. */
    for sig in 1..libc::SIGRTMAX() + 1 {
        if [libc::SIGKILL, libc::SIGSTOP].contains(&sig) {
            continue;
        }
        let mut sig_action: libc::sigaction = unsafe { std::mem::zeroed() };
        sig_action.sa_flags = libc::SA_RESTART;
        sig_action.sa_sigaction = libc::SIG_IGN;
        let r = unsafe { libc::sigaction(sig, &sig_action, std::ptr::null_mut()) };
        if r < 0 {
            log::warn!(
                "Failed to ignore signal {}: {}",
                sig,
                nix::Error::from_i32(r)
            );
        }
    }
}

fn reset_signal_mask() {
    if let Err(e) = signal::sigprocmask(
        signal::SigmaskHow::SIG_SETMASK,
        Some(&SigSet::empty()),
        None,
    ) {
        log::warn!("reset sigprocmask failed:{}", e);
    }
}
