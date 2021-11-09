use std::fs::write;
use std::io::{self, Error};
use std::mem::MaybeUninit;
use std::path::Path;
use std::process::{Child, Command};

use libc::{siginfo_t, waitid};
use nix::libc;
use nix::sys::wait::waitpid;
use nix::unistd::Pid;

const PROCESS1_PATH: &'static str = "/usr/lib/process1/process1";
const SYSTEMD_PATH: &'static str = "/usr/lib/systemd/systemd";
const PIDFILE: &'static str = "/.init.pid";
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
    println!("Running to execute command : {:?}", s);

    let mut error_count = 0;
    let mut child = create_init(s)?;
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
        } else if pid == child.id() as i32 {
            if let Ok(status) = child.wait() {
                if !status.success() {
                    error_count += 1;
                    if error_count >= 3 {
                        break;
                    }
                    println!("Exited with error, Again");
                    child = create_init(s)?;
                }
            }
        } else {
            println!("Receiving {} exited", pid);
            send_signal(child.id(), siginfo);
            let _result = waitpid(Pid::from_raw(pid), None);
            println!("Reaped {}", pid);
        }
    }
    Ok(())
}

fn create_init(s: &String) -> io::Result<Child> {
    let child = Command::new(s).spawn()?;
    let _ = write(String::from(PIDFILE), child.id().to_string());
    Ok(child)
}

fn send_signal(pid: u32, mut siginfo: MaybeUninit<siginfo_t>) {
    unsafe {
        siginfo.assume_init_mut().si_code -= SELF_CODE_OFFSET;
        if libc::syscall(libc::SYS_rt_sigqueueinfo, pid, libc::SIGRTMIN() + SELF_SIG_OFFSET,
                         siginfo.as_mut_ptr()) < 0 {
            println!("send signal error:{}", Error::last_os_error())
        }
    }
}