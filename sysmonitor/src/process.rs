use std::io;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, ExitStatus};
use std::thread::sleep;
use std::time::Duration;

use nix::libc::pid_t;
use nix::sys::signal::kill;
use nix::sys::signal::Signal;
use nix::unistd::Pid;
use serde_derive::Deserialize;

use crate::{Monitor, Switch, SysMonitor, SysMonitorError};
use crate::process_monitor_period_default;

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

fn monitor_mode_default() -> String { "serial".to_string() }

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

    fn is_valid(&self) -> bool {
        (self.monitor_mode == "serial" || self.monitor_mode == "parallel") && self.monitor_period > 0
    }

    fn check_status(&mut self) -> Result<(), SysMonitorError> {
        self.process_monitor_start(self.timeout);
        Ok(())
    }

    fn report_alarm(&self) {}
}

impl ProcessMonitor {
    fn process_monitor_start(&mut self, timeout: u64) {
        loop {
            self.reload_tasks();
            if self.check_service_exist() {
                self.process_recover(timeout);
                self.process_alarm();
            }
            sleep(Duration::from_secs(self.monitor_period))
        }
    }

    fn reload_tasks(&mut self) {}

    fn get_process_check_timeout(&self) -> u64 {
        self.monitor_period + 344
    }

    fn check_service_exist(&mut self) -> bool {
        for _ in 1..2 {
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

    fn check_process_exist(&mut self) -> io::Result<bool> {
        command_wait(self.monitor_command.clone(), self.stop_command.clone(), self.uid, self.timeout)
    }

    fn process_recover(&self, timeout: u64) -> io::Result<bool> {
        command_wait(self.recover_command.clone(), self.stop_command.clone(), self.uid, timeout)
    }

    fn process_alarm(&mut self) -> io::Result<bool> {
        command_wait(self.alarm_command.clone(), "".to_string(), self.uid, self.timeout)
    }

    fn process_alarm_recover(self) -> io::Result<bool> {
        command_wait(self.alarm_recover_command, "".to_string(), self.uid, self.timeout)
    }
}

fn command_wait(command: String, stop_command: String, uid: u32, timeout: u64) -> io::Result<bool> {
    let mut child = Command::new(command).uid(uid).spawn()?;

    if timeout > 0 {
        wait_child(&mut child, timeout, 100);
        if stop_command != "" {
            let _stop = Command::new(stop_command).uid(uid).output();
            process_exit(&mut child);
        }
    } else {
        return match child.wait() {
            Ok(status) => { Ok(status.success()) }
            Err(e) => { Err(e) }
        };
    }

    Ok(false)
}

fn process_exit(child: &mut Child) -> io::Result<ExitStatus> {
    kill(Pid::from_raw(child.id() as pid_t), Signal::SIGTERM);
    sleep(Duration::from_secs(1));
    wait_child(child, PROCESS_EXIT_TIMEOUT, 1000);
    child.kill();
    child.wait()
}

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