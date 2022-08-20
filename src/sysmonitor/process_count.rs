//! 进程数量监控项的实现
use procfs::sys::kernel::pid_max;
use serde_derive::Deserialize;

use std::cmp::max;
use utils::Error;

use crate::{Monitor, Switch, SysMonitor};

const CONFIG_FILE_PATH: &str = "/etc/sysmonitor/pscnt";

/// 首先定义了一个结构体，使用了serde crate的Deserialize trait、default特性、rename_all特性和字段的default方法特性，
/// 和sysmonitor结构体一样
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

/// 默认值参考用户手册
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

/// 然后实现上述的trait
impl Monitor for ProcessCount {
    fn config_path(&self) -> &str {
        CONFIG_FILE_PATH
    }

    /// monitor是已经由我们制定的默认值函数初始化过的结构体，因此返回self的时候只需要修改部分字段就可以了，其他的可以使用rust的..特性由monitor结构体赋值。
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

    /// 检查是否是合法配置的函数，可以参考原有的sysmonitor代码或者用户手册里的描述实现，由一些布尔表达式组成。
    fn is_valid(&self) -> bool {
        self.alarm > self.resume
            && self.resume > 0
            && self.period > 0
            && 0.0 < self.resume_ratio
            && self.resume_ratio < self.alarm_ratio
            && self.alarm_ratio < 100.0
            && self.show_top_proc_num < 1024
    }

    /// 实现真正的业务流程函数check_status
    fn check_status(&mut self) -> Result<(), Error> {
        // 使用procfs crate列出所有的进程
        let all_processes = procfs::process::all_processes()?;
        let proc_num = all_processes.len() as u32;

        let mut thread_num = proc_num;
        if self.show_top_proc_num > 0 {
            // 计算当前系统的线程数量，使用fold函数式计算方法，类似python中的reduce，将每个进程的num_threads累加到num中
            thread_num = all_processes
                .iter()
                .fold(0, |num, process| num + process.stat.num_threads as u32);
        }

        println!("{} {}", proc_num, thread_num);
        let pid_max = pid_max()?;
        if pid_max == 0 {
            return Err(Error::Other {
                msg: "pid_max is 0",
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
        // 如果超出了告警值则告警，并更新status状态
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
