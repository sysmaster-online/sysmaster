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

//! The init daemon
mod config;
mod mount;
mod runtime;
use mount::setup_mount_early;
use nix::sys::signal;
use nix::sys::signal::SaFlags;
use nix::sys::signal::SigAction;
use nix::sys::signal::SigHandler;
use nix::sys::signalfd::SigSet;
use nix::sys::stat::umask;
use nix::sys::stat::Mode;
use nix::unistd;
use nix::unistd::Pid;
#[allow(unused_imports)]
use nix::unistd::Uid;
use runtime::InitState;
use runtime::Runtime;

fn prepare_init() {
    // version
    let version = env!("CARGO_PKG_VERSION");
    let args: Vec<String> = std::env::args().collect();
    if args.contains(&String::from("--version")) || args.contains(&String::from("-V")) {
        println!("sysMaster-init version: {}!", version);
        std::process::exit(0);
    }

    log::info!("sysMaster-init version: {}", version);

    // common umask
    let mode = Mode::from_bits_truncate(0o77);
    umask(umask(mode) | Mode::from_bits_truncate(0o22));

    // euid check
    #[cfg(not(test))]
    if unistd::geteuid() != Uid::from_raw(0) {
        log::error!("Must be superuser.");
        std::process::exit(1);
    }

    if unistd::getpid() != Pid::from_raw(1) {
        log::info!("Running in the test mode.");
    }
}

fn reset_all_signal_handlers() {
    // Create an empty signal set
    let mut sigset = SigSet::empty();

    // Add all signals to the signal set
    for sig in signal::Signal::iterator() {
        if sig == signal::Signal::SIGKILL || sig == signal::Signal::SIGSTOP {
            continue; // Do not allow ignoring SIGKILL and SIGSTOP signals
        }
        sigset.add(sig);
    }

    // Set the signal handler to be ignored
    let sig_action = SigAction::new(SigHandler::SigIgn, SaFlags::SA_RESTART, SigSet::empty());
    for sig in sigset.iter() {
        unsafe {
            signal::sigaction(sig, &sig_action).expect("Failed to set signal handler!");
        }
    }
}

extern "C" fn crash_handler(_signal: i32) {
    panic!("crash_handler");
}

fn install_crash_handler() {
    let signals_crash_handler = [
        signal::SIGSEGV,
        signal::SIGILL,
        signal::SIGFPE,
        signal::SIGBUS,
        signal::SIGABRT,
    ];
    let sig_action = SigAction::new(
        SigHandler::Handler(crash_handler),
        SaFlags::SA_SIGINFO | SaFlags::SA_NODEFER,
        SigSet::empty(),
    );

    for sig in signals_crash_handler {
        unsafe {
            signal::sigaction(sig, &sig_action).expect("Failed to set crash signal handler!");
        }
    }
}

fn shutdown_init() {
    nix::unistd::sync();
    log::info!("shutdowning...");
}

fn main() -> std::io::Result<()> {
    setup_mount_early();

    log::init_log_to_kmsg_console("sysmaster-init", log::Level::Info);

    prepare_init();

    reset_all_signal_handlers();
    install_crash_handler();

    let mut rt = Runtime::new()?;
    rt.set_state(InitState::Running);

    rt.runloop()?;

    if rt.is_reexec() {
        rt.reexec();
    }
    shutdown_init();
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_main() {
        use super::*;
        prepare_init();

        reset_all_signal_handlers();
        install_crash_handler();
        let mut rt = Runtime::new().unwrap();
        rt.set_state(InitState::Running);

        rt.runloop().unwrap();

        if rt.is_reexec() {
            rt.reexec();
        }
        shutdown_init();
    }
}
