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

#![allow(deprecated)]
#[cfg(all(feature = "plugin", feature = "noplugin"))]
compile_error!("feature plugin and noplugin cannot be enabled at the same time");

mod job;
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
#[cfg(feature = "plugin")]
mod plugin;
mod unit;
mod utils;

#[macro_use]
extern crate lazy_static;
extern crate clap;
use crate::manager::config::ManagerConfig;
use crate::manager::{Action, Manager, Mode, MANAGER_ARGS_SIZE_MAX};
use crate::mount::setup;
use basic::logger::{self};
use clap::Parser;
use core::error::*;
use core::rel;
use libc::{c_int, getpid, getppid, prctl, PR_SET_CHILD_SUBREAPER};
use log::{self, LevelFilter};
use nix::sys::signal::{self, SaFlags, SigAction, SigHandler, SigSet, Signal};
use std::convert::TryFrom;
use std::env::{self};
use std::os::unix::process::CommandExt;
use std::process::{exit, Command};
use std::rc::Rc;

/// parse program arguments
#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    #[clap(long)]
    /// Reload the configuration.
    deserialize: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    //------------------------Code placed at the top-----------------------------

    /* systemd use reset_all_signal_handlers, we can't because sysmaster is not PID1.
     * We choose to ignore all signals explicitly, and then register signals we are
     * interested in. But there are still some differences, see docs/man/signal.md
     * for details. */
    ignore_all_signals();

    // The registration signal is at the beginning and has the highest priority!
    register_reexec_signal(true);

    //---------------------------------------------------------------------------

    let manager_config = Rc::new(ManagerConfig::new(None));
    logger::init_log(
        "sysmaster",
        match manager_config.LogLevel.as_str() {
            "debug" => LevelFilter::Debug,
            "info" => LevelFilter::Info,
            "trace" => LevelFilter::Trace,
            "warn" => LevelFilter::Warn,
            "error" => LevelFilter::Error,
            _ => {
                println!("unsupported log level, set log level to off");
                LevelFilter::Off
            }
        },
        &manager_config.LogTarget,
        manager_config.LogFileSize,
        manager_config.LogFileNumber,
    );
    log::info!("sysmaster running in system mode.");

    // temporary annotation for repeat mount

    // mount_setup::mount_setup_early().map_err(|e| {
    //     log::error!("failed to mount early mount point, errno: {}", e);
    //     format!("failed to mount early mount point, errno: {}", e)
    // })?;

    setup::mount_setup()?;

    rel::reli_dir_prepare()?;
    let self_recovery_enable = rel::reli_debug_get_switch();
    log::info!("sysmaster self_recovery_enable: {}.", self_recovery_enable);

    initialize_runtime(self_recovery_enable)?;

    let manager = Manager::new(Mode::System, Action::Run, manager_config);

    // enable clear
    if !self_recovery_enable && !args.deserialize {
        manager.debug_clear_restore();
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

fn initialize_runtime(self_recovery_enable: bool) -> Result<()> {
    if self_recovery_enable {
        install_crash_handler();
        log::info!("install crash handler.");
    } else {
        // if self_recovery_enable is false unregister init's reexec signal.
        register_reexec_signal(false);
    }

    #[cfg(feature = "linux")]
    setup::mount_cgroup_controllers().map_err(|e| Error::Other {
        msg: format!("mount cgroup controllers failed: {}", e),
    })?;

    set_child_reaper();

    Ok(())
}

fn set_child_reaper() {
    let ret = unsafe { prctl(PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) };

    if ret < 0 {
        log::warn!("failed to set child reaper, errno: {}", ret);
    }
}

fn do_reexecute(args: &[String], reload: bool) {
    let args_size = args.len().max(MANAGER_ARGS_SIZE_MAX);

    let path;
    let mut argv = [].to_vec();
    if args.is_empty() {
        let (ppath, pargv) = execarg_build_default();
        path = ppath;
        argv = pargv;
    } else {
        path = args[0].clone();
        if args.len() >= 2 {
            argv = args[1..].to_vec();
        }
    }

    // Remove '--deserialize' from the previous parameter first, as this may be a fault recovery start.
    for index in argv.iter().enumerate() {
        if index.1 == "--deserialize" {
            let idx = index.0;
            argv.remove(idx);
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
    let handler = SigHandler::SigAction(crash);
    let flags = SaFlags::SA_NODEFER;
    let action = SigAction::new(handler, flags, SigSet::empty());
    for &signal in signals.iter() {
        unsafe {
            signal::sigaction(signal, &action).expect("failed to set signal handler for crash")
        };
    }
}

extern "C" fn crash(signo: c_int, siginfo: *mut libc::siginfo_t, _con: *mut libc::c_void) {
    let signal = Signal::try_from(signo).unwrap(); // debug
    if (signal == Signal::SIGABRT && unsafe { (*siginfo).si_pid() == getppid() })
        || unsafe { (*siginfo).si_pid() == getpid() }
    {
        let args: Vec<String> = env::args().collect();
        do_reexecute(&args, false);
    }
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
    let manager_signal: Signal = Signal::SIGABRT;
    let handler = match enable {
        true => SigHandler::SigAction(crash_reexec),
        false => SigHandler::SigAction(crash_none),
    };
    let flags = SaFlags::SA_NODEFER;
    let action = SigAction::new(handler, flags, SigSet::empty());

    unsafe { signal::sigaction(manager_signal, &action).expect("failed to set signal handler") };
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
