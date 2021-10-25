use std::io::Error;
use std::path::Path;
use std::process::Command;

use nix::sys::wait;
use nix::sys::wait::WaitPidFlag;

const PROCESS1_PATH: &str = "/usr/lib/process1/process1";
const SYSTEMD_PATH: &str = "/usr/lib/systemd/systemd";

#[derive(Debug)]
enum Proc1Error {
    IOError(Error),
}

impl std::convert::From<std::io::Error> for Proc1Error {
    fn from(e: std::io::Error) -> Self {
        Proc1Error::IOError(e)
    }
}

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
    std::env::args().nth(1)
}

fn detect_init() -> String {
    if Path::new(PROCESS1_PATH).exists() {
        return String::from(PROCESS1_PATH);
    }
    String::from(SYSTEMD_PATH)
}

fn execute_mode(s: &String) -> Result<(), Proc1Error> {
    println!("running to execute specific command {:?}", s);

    let mut command = Command::new(s);
    let mut child = command.spawn()?;
    loop {
        let result = child.try_wait();
        match result {
            Ok(Some(status)) => {
                if status.success() {
                    println!("successfully finished");  // shutdown
                    break;
                } else {
                    println!("exited with error");
                    child = command.spawn()?;
                }
            }
            _ => {}
        }
        reap_zombie()
    }
    Ok(())
}

fn reap_zombie() {
    let _result = wait::waitpid(nix::unistd::Pid::from_raw(-1),
                                Some(WaitPidFlag::WNOHANG));
    send_signal()
}

fn send_signal() {}
