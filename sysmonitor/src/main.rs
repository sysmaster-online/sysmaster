use std::default::Default;
use std::fs;
use std::fs::File;
use std::io::{self, Error, Read};
use std::num::ParseIntError;
use std::string::FromUtf8Error;

use procfs::ProcError;
use serde_derive::Deserialize;

use crate::process::ProcessMonitor;
use crate::process_count::ProcessCount;
use crate::process_fd::ProcessFd;
use crate::zombie::ZombieCount;

mod process_count;
mod zombie;
mod process_fd;
mod process;

#[derive(Debug)]
pub enum SysMonitorError {
    ProcfsError(ProcError),
    IOError(Error),
    ParseError(ParseIntError),
    UtfError(FromUtf8Error),
}

impl std::convert::From<ProcError> for SysMonitorError {
    fn from(e: ProcError) -> Self {
        SysMonitorError::ProcfsError(e)
    }
}

impl std::convert::From<Error> for SysMonitorError {
    fn from(e: Error) -> Self {
        SysMonitorError::IOError(e)
    }
}

impl std::convert::From<ParseIntError> for SysMonitorError {
    fn from(e: ParseIntError) -> Self {
        SysMonitorError::ParseError(e)
    }
}

impl std::convert::From<FromUtf8Error> for SysMonitorError {
    fn from(e: FromUtf8Error) -> Self {
        SysMonitorError::UtfError(e)
    }
}

const CONFIG_FILE_PATH: &str = "/etc/sysconfig/sysmonitor";

pub trait Monitor {
    fn config_path(&self) -> &str;
    fn load(&mut self, content: String, sysmonitor: SysMonitor);
    fn is_valid(&self) -> bool;
    fn check_status(&mut self) -> Result<(), SysMonitorError>;
    fn report_alarm(&self);
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "UPPERCASE")]
pub struct SysMonitor {
    #[serde(default = "on")]
    process_monitor: bool,
    #[serde(default = "process_monitor_period_default")]
    process_monitor_period: u64,
    #[serde(default = "process_recall_default_period")]
    process_recall_period: u32,
    #[serde(default = "process_restart_default_timeout")]
    process_restart_timeout: u32,
    #[serde(default = "process_alarm_suppress_num_default")]
    process_alarm_suppress_num: u32,
    process_alarm: bool,
    #[serde(default = "on")]
    pscnt_monitor: bool,
    pscnt_alarm: bool,
    #[serde(default = "on")]
    process_fd_num_monitor: bool,
    process_fd_num_alarm: bool,
    zombie_monitor: bool,
    zombie_alarm: bool,
}

#[derive(Debug, Default, Deserialize)]
pub struct Switch {
    monitor: bool,
    alarm: bool,
}

pub fn config_file_load(file_path: &str) -> io::Result<String> {
    let mut file = File::open(file_path)?;

    let mut buf = String::new();
    file.read_to_string(&mut buf)?;
    Ok(buf)
}

fn on() -> bool { true }

pub fn process_monitor_period_default() -> u64 { 3 }

fn process_recall_default_period() -> u32 { 1 }

fn process_restart_default_timeout() -> u32 { 90 }

fn process_alarm_suppress_num_default() -> u32 { 5 }

fn main() -> io::Result<()> {
    let toml_str = fs::read_to_string(CONFIG_FILE_PATH)?;
    let sysmonitor: SysMonitor = toml::from_str(&toml_str).unwrap();

    let monitors: [&mut dyn Monitor; 4] = [&mut ProcessCount::default(), &mut ZombieCount::default(), &mut ProcessFd::default(), &mut ProcessMonitor::default()];
    for monitor in monitors {
        let contents = fs::read_to_string(monitor.config_path())?;
        monitor.load(contents, sysmonitor.clone());
        monitor.is_valid();
        let _ = monitor.check_status();
    }
    Ok(())
}