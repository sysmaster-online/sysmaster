//! Daemon sysmaster or Systemd, restart the process when it exits

use std::collections::VecDeque;
use std::io::Error;
use std::mem::MaybeUninit;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{exit, Command};
use std::time::{Duration, Instant};

use libc::{siginfo_t, waitid};
use nix::libc;
use nix::sys::wait::waitpid;
use nix::unistd::{fork, ForkResult, Pid};
use signal_hook_registry::register_sigaction;

const SYSMASTER_PATH: &str = "/usr/lib/sysmaster/sysmaster";
const SYSTEMD_PATH: &str = "/usr/lib/systemd/systemd";
const TIME_INTERVAL: Duration = Duration::from_secs(10);
const FAIL_COUNT: usize = 3;
const SELF_CODE_OFFSET: i32 = 14;
const SELF_SIG_OFFSET: i32 = 7;

fn main() {
    register_signal();
    let cmd = match get_command() {
        Some(cmd) => cmd,
        None => detect_init(),
    };
    loop {
        if execute_mode(&cmd).is_ok() {
            break;
        }
    }
}

fn register_signal() {
    unsafe {
        let _ = register_sigaction(libc::SIGRTMIN() + SELF_SIG_OFFSET, |siginfo| {
            let _ = waitpid(Pid::from_raw(siginfo.si_pid()), None);
        });
    }
}

fn get_command() -> Option<String> {
    if std::env::args().count() > 2 {
        panic!("More than one para, panic!!!");
    }

    std::env::args().nth(1)
}

fn detect_init() -> String {
    if Path::new(SYSMASTER_PATH).exists() {
        return String::from(SYSMASTER_PATH);
    } else if Path::new(SYSTEMD_PATH).exists() {
        return String::from(SYSTEMD_PATH);
    }

    String::new()
}

fn execute_mode(s: &str) -> Result<(), Error> {
    let mut fail_record: VecDeque<Instant> = VecDeque::with_capacity(FAIL_COUNT);
    let mut child = create_init(s);
    loop {
        let mut siginfo = MaybeUninit::<siginfo_t>::zeroed();
        let pid = unsafe {
            if waitid(
                libc::P_ALL,
                0,
                siginfo.as_mut_ptr(),
                libc::WEXITED | libc::WNOWAIT,
            ) < 0
            {
                continue;
            };
            siginfo.assume_init().si_pid()
        };
        if pid <= 0 {
            continue;
        } else if pid == child.as_raw() {
            if need_exit(&mut fail_record) {
                println!("Manager({}) failed 3 times, exit", pid);
                let _ = waitpid(Pid::from_raw(pid), None);
                break;
            }
            child = create_init(s);
        } else {
            send_signal(pid, siginfo);
        }
        println!("Reaped child {}", pid);
        let _ = waitpid(Pid::from_raw(pid), None);
    }
    Ok(())
}

fn create_init(s: &str) -> Pid {
    println!("Running to execute command : {:?}", s);
    if let Ok(ForkResult::Parent { child, .. }) = unsafe { fork() } {
        child
    } else {
        let mut command = Command::new(s);
        let comm = command.env("SYSTEMD_MAINPID", format!("{}", unsafe { libc::getpid() }));
        comm.exec();
        exit(0)
    }
}

fn need_exit(fail_record: &mut VecDeque<Instant>) -> bool {
    fail_record.push_back(Instant::now());
    if fail_record.len() == FAIL_COUNT
        && fail_record.pop_front().unwrap().elapsed() <= TIME_INTERVAL
    {
        return true;
    }
    false
}

fn send_signal(pid: i32, mut siginfo: MaybeUninit<siginfo_t>) {
    unsafe {
        siginfo.assume_init_mut().si_code -= SELF_CODE_OFFSET;
        if libc::syscall(
            libc::SYS_rt_sigqueueinfo,
            pid,
            libc::SIGRTMIN() + SELF_SIG_OFFSET,
            siginfo.as_mut_ptr(),
        ) < 0
        {
            println!("send signal error:{}", Error::last_os_error())
        }
    }
}
