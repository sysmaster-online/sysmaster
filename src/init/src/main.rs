use std::io::Error;
use std::path::Path;
use std::process::Command;

use nix::unistd::Pid;
use nix::sys::wait::waitpid;
use nix::sys::wait::WaitPidFlag;

const PROCESS1_PATH: &'static str = "/usr/lib/process1/process1";
const SYSTEMD_PATH: &'static str = "/usr/lib/systemd/systemd";

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

    let mut command = Command::new(s);
    let mut child = command.spawn()?;
    loop {
        let result = child.try_wait();
        match result {
            Ok(Some(status)) => {
                if status.success() {
                    println!("Successfully finished");  // shutdown
                    break;
                } else {
                    println!("Exited with error, Again");
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
    let _result = waitpid(Pid::from_raw(-1),
                                Some(WaitPidFlag::WNOHANG));
    send_signal()
}

fn send_signal() {}
