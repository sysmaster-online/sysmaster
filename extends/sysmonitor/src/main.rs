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

//! sysmonitor main process. As part of system monitoring,
//! process number monitoring, zombie process number monitoring,
//! process fd number monitoring and key process monitoring.
use serde_derive::Deserialize;

use basic::Error;
use std::default::Default;
use std::fs;
use std::fs::File;
use std::io::{self, Read};

use crate::process::ProcessMonitor;
use crate::process_count::ProcessCount;
use crate::process_fd::ProcessFd;
use crate::zombie::ZombieCount;

#[allow(dead_code)]
mod process;
mod process_count;
mod process_fd;
mod zombie;

/// default configuration file path
const CONFIG_FILE_PATH: &str = "/etc/sysconfig/sysmonitor";

/// First define a trait, which encapsulates several features
pub trait Monitor {
    /// Each monitor will have its own configuration file path,
    fn config_path(&self) -> &str;
    /// convert the configuration file into a structure
    fn load(&mut self, content: String, sysmonitor: SysMonitor);
    /// checks whether the configuration item is legal,
    fn is_valid(&self) -> bool;
    /// Check the current monitoring of the indicators concerned,
    fn check_status(&mut self) -> Result<(), Error>;
    /// report an alarm
    fn report_alarm(&self);
}

/// Monitor structure
#[allow(dead_code)]
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

/// have common control options, monitor is enabled by default, and alarm is disabled by default
#[allow(dead_code)]
#[derive(Debug, Default, Deserialize)]
pub struct Switch {
    monitor: bool,
    alarm: bool,
}

/// loading configuration
pub fn config_file_load(file_path: &str) -> io::Result<String> {
    let mut file = File::open(file_path)?;

    let mut buf = String::new();
    file.read_to_string(&mut buf)?;
    Ok(buf)
}

fn on() -> bool {
    true
}

/// the default Monitoring period value
pub fn process_monitor_period_default() -> u64 {
    3
}

/// after failure to restore
fn process_recall_default_period() -> u32 {
    1
}

/// Default value for process restart time
fn process_restart_default_timeout() -> u32 {
    90
}

/// Default value of alarm suppression
fn process_alarm_suppress_num_default() -> u32 {
    5
}

fn main() -> io::Result<()> {
    // Generate sysmonitor structure from configuration file
    let toml_str = fs::read_to_string(CONFIG_FILE_PATH)?;
    let sysmonitor: SysMonitor = toml::from_str(&toml_str).unwrap();

    // Currently supports four, the number of processes,
    // the number of zombie processes, the number of process fd monitoring and key process monitoring,
    // the array can be modified later
    let monitors: [&mut dyn Monitor; 4] = [
        &mut ProcessCount::default(),
        &mut ZombieCount::default(),
        &mut ProcessFd::default(),
        &mut ProcessMonitor::default(),
    ];
    for monitor in monitors {
        let contents = fs::read_to_string(monitor.config_path())?;
        monitor.load(contents, sysmonitor.clone());
        monitor.is_valid();
        let _ = monitor.check_status();
    }
    Ok(())
}
