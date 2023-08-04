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

//! Implementation of process quantity monitoring item
use procfs::sys::kernel::pid_max;
use serde_derive::Deserialize;

use basic::{Error, ProcSnafu, ResultExt};
use std::cmp::max;

use crate::{Monitor, Switch, SysMonitor};

const CONFIG_FILE_PATH: &str = "/etc/sysmonitor/pscnt";

/// First define a structure, using the Deserialize trait, default trait, rename_all trait and default method trait of the field of the serde crate,
/// Same as sysmonitor structure
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "UPPERCASE")]
pub struct ProcessCount {
    pub(crate) config: Switch,
    #[serde(default = "alarm_default")]
    pub alarm: u32,
    #[serde(default = "resume_default")]
    pub resume: u32,
    #[serde(default = "period_default")]
    pub period: u32,
    #[serde(default = "alarm_ratio_default")]
    pub alarm_ratio: f32,
    #[serde(default = "resume_ratio_default")]
    pub resume_ratio: f32,
    #[serde(default = "show_top_proc_num_default")]
    pub show_top_proc_num: u32,
    pub count: u32,
    pub status: bool,
}

/// Refer to the user manual for default values
fn alarm_default() -> u32 {
    1600
}

fn resume_default() -> u32 {
    1500
}

fn period_default() -> u32 {
    60
}

fn alarm_ratio_default() -> f32 {
    90.0
}

fn resume_ratio_default() -> f32 {
    80.0
}

fn show_top_proc_num_default() -> u32 {
    10
}

/// Then implement the above trait
impl Monitor for ProcessCount {
    fn config_path(&self) -> &str {
        CONFIG_FILE_PATH
    }

    /// monitor is a structure that has been initialized by the default value function we made, so when returning self,
    /// only some fields need to be modified, and others can be assigned by the monitor structure using the .. feature of rust.
    fn load(&mut self, content: String, sysmonitor: SysMonitor) {
        let monitor: Self = toml::from_str(content.as_str()).unwrap();
        *self = ProcessCount {
            config: Switch {
                monitor: sysmonitor.pscnt_monitor,
                alarm: sysmonitor.pscnt_alarm,
            },
            ..monitor
        };
    }

    /// The function to check whether it is a valid configuration can be implemented by referring to the original
    /// sysmonitor code or the description in the user manual. It consists of some Boolean expressions.
    fn is_valid(&self) -> bool {
        self.alarm > self.resume
            && self.resume > 0
            && self.period > 0
            && 0.0 < self.resume_ratio
            && self.resume_ratio < self.alarm_ratio
            && self.alarm_ratio < 100.0
            && self.show_top_proc_num < 1024
    }

    /// Implement the real business process
    fn check_status(&mut self) -> Result<(), Error> {
        // List all processes with procfs crate
        let all_processes = procfs::process::all_processes().context(ProcSnafu)?;
        let proc_num = all_processes.len() as u32;

        let mut thread_num = proc_num;
        if self.show_top_proc_num > 0 {
            // Calculate the number of threads in the current system, use the fold function calculation method,
            // similar to reduce in python, and accumulate the num_threads of each process into num
            thread_num = all_processes
                .iter()
                .fold(0, |num, process| num + process.stat.num_threads as u32);
        }

        println!("{} {}", proc_num, thread_num);
        let pid_max = pid_max().context(ProcSnafu)?;
        if pid_max == 0 {
            return Err(Error::Other {
                msg: "pid_max is 0".to_string(),
            });
        }
        let real_alarm = max(
            self.alarm,
            (pid_max as f32 * self.alarm_ratio / 100.0) as u32,
        );
        let real_resume = max(
            self.resume,
            (pid_max as f32 * self.resume_ratio / 100.0) as u32,
        );
        self.count = proc_num;
        // If the value is exceeded, update the status
        if proc_num >= real_alarm && !self.status {
            self.report_alarm()
        } else if proc_num <= real_resume && self.status {
        }

        Ok(())
    }

    fn report_alarm(&self) {}
}

#[cfg(test)]
mod tests {
    use crate::process_count::ProcessCount;
    use crate::Monitor;

    #[test]
    fn test_decode_config() {
        let toml_str = r#"
        ALARM = 1600
        RESUME = 1500
        PERIOD = 60
        "#;

        let decoded: ProcessCount = toml::from_str(toml_str).unwrap();
        println!("{:#?}", decoded);
    }

    #[test]
    fn test_check_status() {
        let mut p = ProcessCount::default();
        let _ = p.check_status();
    }
}
