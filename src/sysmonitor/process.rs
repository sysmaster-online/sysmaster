//! 关键进程的监控
use nix::libc::pid_t;
use nix::sys::signal::kill;
use nix::sys::signal::Signal;
use nix::unistd::Pid;
use serde_derive::Deserialize;

use std::io;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, ExitStatus};
use std::thread::sleep;
use std::time::Duration;
use utils::Error;

use crate::process_monitor_period_default;
use crate::{Monitor, Switch, SysMonitor};

const CONFIG_FILE_PATH: &str = "/etc/sysmonitor/process";
const PROCESS_EXIT_TIMEOUT: u64 = 10;

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct ProcessMonitor {
    pub(crate) config: Switch,
    pub name: String,
    #[serde(default)]
    recover_command: String,
    #[serde(default)]
    monitor_command: String,
    #[serde(default)]
    stop_command: String,
    #[serde(default)]
    uid: u32,
    #[serde(default)]
    check_as_param: bool,
    #[serde(default = "monitor_mode_default")]
    monitor_mode: String,
    #[serde(default = "process_monitor_period_default")]
    monitor_period: u64,
    #[serde(default)]
    usr_cmd_alarm: bool,
    #[serde(default)]
    alarm_command: String,
    #[serde(default)]
    alarm_recover_command: String,
    #[serde(default)]
    timeout: u64,
}

fn monitor_mode_default() -> String {
    "serial".to_string()
}

impl Monitor for ProcessMonitor {
    fn config_path(&self) -> &str {
        CONFIG_FILE_PATH
    }

    fn load(&mut self, content: String, sysmonitor: SysMonitor) {
        let monitor: Self = toml::from_str(content.as_str()).unwrap();
        *self = ProcessMonitor {
            config: Switch {
                monitor: sysmonitor.process_monitor,
                alarm: sysmonitor.process_alarm,
            },
            ..monitor
        };
    }

    /// 只支持串行和并行两种模式
    fn is_valid(&self) -> bool {
        (self.monitor_mode == "serial" || self.monitor_mode == "parallel")
            && self.monitor_period > 0
    }

    fn check_status(&mut self) -> Result<(), Error> {
        self.process_monitor_start(self.timeout);
        Ok(())
    }

    fn report_alarm(&self) {}
}

impl ProcessMonitor {
    fn process_monitor_start(&mut self, timeout: u64) {
        loop {
            self.reload_tasks();
            // 检查进程是否还存在，存在的话则恢复告警，不存在的话则告警并恢复
            if self.check_service_exist() {
                let _ = self.process_alarm_recover();
            } else {
                let _ = self.process_alarm();
                let _ = self.process_recover(timeout);
            }
            sleep(Duration::from_secs(self.monitor_period))
        }
    }

    fn reload_tasks(&mut self) {}

    fn get_process_check_timeout(&self) -> u64 {
        self.monitor_period + 344
    }

    /// 只重复检查两次，若还超时则返回false
    fn check_service_exist(&mut self) -> bool {
        for _ in 0..2 {
            match self.check_process_exist() {
                Ok(true) => return true,
                Ok(false) => continue,
                Err(_) => {
                    return false;
                }
            }
        }
        false
    }

    /// 检查进程是否存在
    fn check_process_exist(&mut self) -> io::Result<bool> {
        command_wait(
            self.monitor_command.clone(),
            self.stop_command.clone(),
            self.uid,
            self.get_process_check_timeout(),
        )
    }

    /// 进程恢复
    fn process_recover(&self, timeout: u64) -> io::Result<bool> {
        command_wait(
            self.recover_command.clone(),
            self.stop_command.clone(),
            self.uid,
            timeout,
        )
    }

    /// 执行告警的命令
    fn process_alarm(&mut self) -> io::Result<bool> {
        command_wait(
            self.alarm_command.clone(),
            "".to_string(),
            self.uid,
            self.timeout,
        )
    }

    /// 执行告警恢复的命令
    fn process_alarm_recover(&mut self) -> io::Result<bool> {
        command_wait(
            self.alarm_recover_command.clone(),
            "".to_string(),
            self.uid,
            self.timeout,
        )
    }
}

/// 将生成子进程，执行命令并等待返回结果这一模式抽象成一个函数，command为需要执行的命令，stop_command为超时之后停止的命令
fn command_wait(command: String, stop_command: String, uid: u32, timeout: u64) -> io::Result<bool> {
    let mut child = Command::new(command).uid(uid).spawn()?;

    if timeout > 0 {
        wait_child(&mut child, timeout, 100)?;
        if !stop_command.is_empty() {
            let _stop = Command::new(stop_command).uid(uid).output();
            process_exit(&mut child)?;
        }
    } else {
        return match child.wait() {
            Ok(status) => Ok(status.success()),
            Err(e) => Err(e),
        };
    }

    Ok(false)
}

/// 杀死一个子进程
fn process_exit(child: &mut Child) -> io::Result<ExitStatus> {
    kill(Pid::from_raw(child.id() as pid_t), Signal::SIGTERM)?;
    sleep(Duration::from_secs(1));
    wait_child(child, PROCESS_EXIT_TIMEOUT, 1000)?;
    child.kill()?;
    child.wait()
}

/// 带超时时间的等待子进程执行命令并获取ExitStatus
fn wait_child(child: &mut Child, timeout_s: u64, sleep_ms: u64) -> io::Result<Option<ExitStatus>> {
    for _i in 0..timeout_s * 1000 / sleep_ms {
        match child.try_wait() {
            Ok(None) => {
                sleep(Duration::from_millis(sleep_ms));
            }
            r => {
                return r;
            }
        };
    }
    Ok(None)
}
