use std::process::Command;

use serde_derive::Deserialize;

use crate::{Monitor, Switch, SysMonitor, SysMonitorError};

const CONFIG_FILE_PATH: &str = "/etc/sysmonitor/zombie";

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "UPPERCASE")]
pub struct ZombieCount {
    pub(crate) config: Switch,
    #[serde(default = "alarm_default")]
    pub alarm: u32,
    #[serde(default = "resume_default")]
    pub resume: u32,
    #[serde(default = "period_default")]
    pub period: u32,
    pub status: bool,
}

fn alarm_default() -> u32 {
    500
}

fn resume_default() -> u32 {
    400
}

fn period_default() -> u32 {
    60
}

impl Monitor for ZombieCount {
    fn config_path(&self) -> &str {
        CONFIG_FILE_PATH
    }

    fn load(&mut self, content: String, sysmonitor: SysMonitor) {
        let monitor: Self = toml::from_str(content.as_str()).unwrap();
        *self = ZombieCount {
            config: Switch {
                monitor: sysmonitor.zombie_monitor,
                alarm: sysmonitor.zombie_alarm,
            },
            ..monitor
        };
    }

    fn is_valid(&self) -> bool {
        self.alarm > self.resume
    }

    fn check_status(&mut self) -> Result<(), SysMonitorError> {
        let cmd = "ps -A -o stat,ppid,pid,cmd | grep -e '^[Zz]' | awk '{print $0}' | wc -l";
        let output = Command::new("bash").arg("-c").arg(cmd).output()?;
        let out = String::from_utf8(output.stdout)?;
        let count: u32 = out.replace("\n", "").parse()?;

        println!("zombie count: {}", count);

        if count >= self.alarm && !self.status {
            let _ = Command::new("/usr/libexec/sysmonitor/getzombieparent.py").output()?;
        } else if count <= self.resume && self.status {
        }

        return Ok(());
    }

    fn report_alarm(&self) {}
}

#[cfg(test)]
mod tests {
    use crate::zombie::ZombieCount;
    use crate::Monitor;

    #[test]
    fn test_check_status() {
        let mut z = ZombieCount::default();
        match z.check_status() {
            Ok(_) => {}
            Err(e) => {
                println!("error: {:?}", e);
            }
        }
    }
}
