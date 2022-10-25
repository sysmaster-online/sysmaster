//!

use libc::c_int;
use log::{self};
use nix::sys::signal::{self, SaFlags, SigAction, SigHandler, SigSet, Signal};
use nix::unistd::{self};
use process1::manager::{Action, ManagerX, Mode, MANAGER_ARGS_SIZE_MAX};
use process1::mount::mount_setup;
use process1::{self};
use std::convert::TryFrom;
use std::env::{self};
use std::error::Error;
use std::ffi::CString;
use utils::logger::{self};

fn main() -> Result<(), Box<dyn Error>> {
    logger::init_log_with_console("process1", 4);
    log::info!("process1 running in system mode.");

    // temporary annotation for repeat mount

    // mount_setup::mount_setup_early().map_err(|e| {
    //     log::error!("failed to mount early mount point, errno: {}", e);
    //     format!("failed to mount early mount point, errno: {}", e)
    // })?;

    mount_setup::mount_setup().map_err(|e| {
        log::error!("failed to mount mount point, errno: {}", e);
        format!("failed to mount mount point, errno: {}", e)
    })?;

    process1::reli_dir_prepare().expect("reliability directory prepare failed.");
    let switch = process1::reli_debug_get_switch();
    log::info!("process1 initialize with switch: {}.", switch);

    initialize_runtime(switch)?;

    let args: Vec<String> = env::args().collect();
    let manager = ManagerX::new(Mode::System, Action::Run);

    // enable clear, mutex with install_crash_handler
    if !switch {
        manager.debug_clear_restore();
        log::info!("debug: clear data restored.");
    }

    // startup
    manager.startup().unwrap();

    // main loop
    let ret = manager.main_loop();
    log::info!("process1 end its main loop with result: {:?}", ret);

    // get result
    let reexec = ret.map_or(false, |ree| ree);

    // re-exec
    if reexec {
        do_reexecute(&args);
    }

    Ok(())
}

fn initialize_runtime(switch: bool) -> Result<(), Box<dyn Error>> {
    if switch {
        install_crash_handler();
        log::info!("install crash handler.");
    }

    mount_setup::mount_cgroup_controllers()?;

    Ok(())
}

fn do_reexecute(args: &Vec<String>) {
    let args_size = args.len().max(MANAGER_ARGS_SIZE_MAX);

    // build default arg
    let (cmd, argv) = execarg_build_default();
    assert!(argv.len() <= args_size);

    // action
    if let Err(e) = unistd::execv(&cmd, &argv) {
        log::info!("execute failed, with arg{:?} result {:?}", argv, e);
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

    // default
    let (cmd, argv) = execarg_build_default();
    if let Err(_e) = unistd::execv(&cmd, &argv) {
        // debug
    }
}

fn execarg_build_default() -> (CString, Vec<CString>) {
    let mut argv = Vec::new();

    // current execute path
    let path = env::current_exe().unwrap();
    let cmd = CString::new(path.file_name().unwrap().to_str().unwrap()).unwrap();
    argv.push(cmd.clone());

    // return
    (cmd, argv)
}
