use std::cmp::max;
use std::io::{Error, ErrorKind};

use procfs::sys::kernel::pid_max;
use procfs::ProcError;
use serde_derive::Deserialize;

use crate::{Monitor, Switch, SysMonitor, SysMonitorError};

const CONFIG_FILE_PATH: &str = "/etc/sysmonitor/pscnt";

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

impl Monitor for ProcessCount {
    fn config_path(&self) -> &str {
        CONFIG_FILE_PATH
    }

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

    fn is_valid(&self) -> bool {
        self.alarm > self.resume
            && self.resume > 0
            && self.period > 0
            && 0.0 < self.resume_ratio
            && self.resume_ratio < self.alarm_ratio
            && self.alarm_ratio < 100.0
            && self.show_top_proc_num < 1024
    }

    fn check_status(&mut self) -> Result<(), SysMonitorError> {
        let all_processes = procfs::process::all_processes()?;
        let proc_num = all_processes.len() as u32;

        let mut thread_num = proc_num;
        if self.show_top_proc_num > 0 {
            thread_num = all_processes
                .iter()
                .fold(0, |num, process| num + process.stat.num_threads as u32);
        }

        println!("{} {}", proc_num, thread_num);
        let pid_max = pid_max()?;
        if pid_max == 0 {
            return Err(SysMonitorError::ProcfsError(ProcError::from(Error::new(
                ErrorKind::Other,
                "found pid_max is 0",
            ))));
        }
        let real_alarm = max(
            self.alarm,
            (pid_max as f32 * self.alarm_ratio / 100.0) as u32,
        );
        let real_resume = max(
            self.resume,
            (pid_max as f32 * self.resume_ratio / 100.0) as u32,
        );
        //  self.count = proc_num;
        // if proc_num >= real_alarm && !self.status {
        // } else if proc_num <= real_resume && self.status {
        // }

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
