use std::io::Error;
use std::mem::MaybeUninit;
use std::path::Path;
use std::process::{Command, exit};
use std::os::unix::process::CommandExt;

use libc::{siginfo_t, waitid};
use nix::libc;
use nix::sys::wait::waitpid;
use nix::unistd::{Pid, fork, ForkResult};

const PROCESS1_PATH: &'static str = "/usr/lib/process1/process1";
const SYSTEMD_PATH: &'static str = "/usr/lib/systemd/systemd";
const SELF_CODE_OFFSET: i32 = 14;
const SELF_SIG_OFFSET: i32 = 7;

fn main() {
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

fn get_command() -> Option<String> {
    if std::env::args().count() > 2 {
        panic!("More than one para, panic!!!");
    }

    std::env::args().nth(1)
}

fn detect_init() -> String {
    if Path::new(PROCESS1_PATH).exists() {
        return String::from(PROCESS1_PATH);
    } else if Path::new(SYSTEMD_PATH).exists() {
        return String::from(SYSTEMD_PATH);
    }

    String::new()
}

fn execute_mode(s: &String) -> Result<(), Error> {
    let mut error_count = 0;
    let mut child = create_init(s);

    loop {
        let mut siginfo = MaybeUninit::<siginfo_t>::zeroed();
        let pid = unsafe {
            if waitid(libc::P_ALL, 0, siginfo.as_mut_ptr(),
                      libc::WEXITED | libc::WNOWAIT) < 0 {
                continue;
            };
            siginfo.assume_init().si_pid()
        };
        if pid <= 0 {
            continue;
        } else if pid == child.as_raw() {
            error_count += 1;
            if error_count >= 3 {
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

fn create_init(s: &String) -> Pid  {
    println!("Running to execute command : {:?}", s);

    if let Ok(ForkResult::Parent { child, .. }) = unsafe{fork()} {
        child
    } else {
        let mut command = Command::new(s);
        let comm = command.env("SYSTEMD_MAINPID", format!("{}", unsafe{ libc::getpid() }));
        comm.exec();
        exit(0)
    }
}

fn send_signal(pid: i32, mut siginfo: MaybeUninit<siginfo_t>) {
    unsafe {
        siginfo.assume_init_mut().si_code -= SELF_CODE_OFFSET;
        if libc::syscall(libc::SYS_rt_sigqueueinfo, pid, libc::SIGRTMIN() + SELF_SIG_OFFSET,
                         siginfo.as_mut_ptr()) < 0 {
            println!("send signal error:{}", Error::last_os_error())
        }
    }
}
