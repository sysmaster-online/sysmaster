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

//! hostname setup

use basic::cmdline;
use basic::os_release;
use nix::Result;
use std::fmt;
use std::fmt::Display;

const SYSTEMD_HOSTNAME_KEY: &str = "systemd.hostname";
const SYSMASTER_HOSTNAME_KEY: &str = "sysmaster.hostname";
const DEFAULT_HOSTNAME: &str = "localhost";

#[derive(Debug)]
struct Hostname {
    hostname: String,
}

impl Hostname {
    fn from_string(str_hostname: &str) -> Option<Hostname> {
        let hostname = Hostname::new(str_hostname.trim());
        if hostname.valid() {
            Some(hostname)
        } else {
            None
        }
    }

    fn from_cmdline() -> Option<Hostname> {
        let mut hostname = match cmdline::Cmdline::default().get_param(SYSTEMD_HOSTNAME_KEY) {
            None => {
                log::warn!(
                    "Failed to get proc cmdline by key {}.",
                    SYSTEMD_HOSTNAME_KEY
                );
                "".to_string()
            }
            Some(h) => h,
        };

        if hostname.is_empty() {
            hostname = match cmdline::Cmdline::default().get_param(SYSMASTER_HOSTNAME_KEY) {
                None => {
                    log::warn!(
                        "Failed to get proc cmdline by key {}.",
                        SYSMASTER_HOSTNAME_KEY
                    );
                    "".to_string()
                }
                Some(h) => h,
            };
        }
        Hostname::from_string(&hostname)
    }

    fn from_etc_hostname() -> Option<Self> {
        match std::fs::read_to_string("/etc/hostname") {
            Err(e) => {
                log::warn!("Failed to get /etc/hostname: {}", e);
                None
            }
            Ok(hostname) => Hostname::from_string(&hostname),
        }
    }

    fn new(hostname: &str) -> Self {
        Hostname {
            hostname: hostname.trim().to_string(),
        }
    }

    fn valid(&self) -> bool {
        if self.hostname.is_empty() {
            return false;
        }

        if self.hostname.len() > 64 {
            return false;
        }

        let mut dot = true;
        let mut hyphen = true;

        for c in self.hostname.chars() {
            if c == '.' {
                if dot || hyphen {
                    return false;
                }
                dot = true;
                hyphen = false;
            } else if c == '-' {
                if dot {
                    return false;
                }
                dot = false;
                hyphen = true;
            } else {
                if !c.is_ascii_alphanumeric() {
                    return false;
                }
                dot = false;
                hyphen = false;
            }
        }

        if dot {
            return false;
        }

        if hyphen {
            return false;
        }
        true
    }

    fn setup(hostname: &Self) -> Result<()> {
        if !hostname.valid() {
            return Err(nix::errno::Errno::EINVAL);
        }

        let local_hostname = Hostname::local_hostname()?;
        if local_hostname.eq(hostname) {
            log::info!("Hostname has already been set: {}.", hostname);
            return Ok(());
        }
        nix::unistd::sethostname(&hostname.hostname)
    }

    fn local_hostname() -> Result<Hostname> {
        match nix::sys::utsname::uname() {
            Err(e) => {
                log::warn!("Failed to get uname: {}", e);
                Err(e)
            }
            Ok(uts_name) => Ok(uts_name
                .nodename()
                .to_str()
                .map_or(Hostname::new(""), Hostname::new)),
        }
    }

    fn hostname_is_set() -> bool {
        Hostname::local_hostname().map_or(false, |hostname| hostname != Hostname::new("(none)"))
    }
}

impl PartialEq for Hostname {
    fn eq(&self, other: &Self) -> bool {
        self.hostname.eq(&other.hostname)
    }
}

impl Display for Hostname {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.hostname)
    }
}

impl Default for Hostname {
    fn default() -> Self {
        if let Ok(Some(str)) = os_release::get_os_release("DEFAULT_HOSTNAME") {
            return Hostname::from_string(&str).unwrap_or_else(|| Hostname::new(DEFAULT_HOSTNAME));
        }
        Hostname::new(DEFAULT_HOSTNAME)
    }
}

fn main() {
    log::init_log_to_console_syslog("hostname-setup", log::Level::Info);
    let mut op_hostname = Hostname::from_cmdline();
    if op_hostname.is_none() {
        op_hostname = Hostname::from_etc_hostname();
    }

    if op_hostname.is_none() && Hostname::hostname_is_set() {
        log::info!("Hostname has already been set, skipping.");
        return;
    }

    let hostname = op_hostname.unwrap_or_default();

    log::info!("Hostname set to: {}.", hostname);
    if let Err(e) = Hostname::setup(&hostname) {
        log::error!("Failed to set hostname: {}.", e);
        std::process::exit(e as i32);
    }
    std::process::exit(0);
}

#[cfg(test)]
mod test {
    use crate::Hostname;

    #[test]
    fn test_hostname_valid() {
        assert!(Hostname::new("foobar").valid());
        assert!(Hostname::new("foobar.com").valid());
        assert!(!Hostname::new("foobar.com.").valid());
        assert!(Hostname::new("fooBAR").valid());
        assert!(Hostname::new("fooBAR.com").valid());
        assert!(!Hostname::new("fooBAR.").valid());
        assert!(!Hostname::new("fooBAR.com.").valid());
        assert!(!Hostname::new("fööbar").valid());
        assert!(!Hostname::new("").valid());
        assert!(!Hostname::new(".").valid());
        assert!(!Hostname::new("..").valid());
        assert!(!Hostname::new("foobar.").valid());
        assert!(!Hostname::new(".foobar").valid());
        assert!(!Hostname::new("foo..bar").valid());
        assert!(!Hostname::new("foo.bar..").valid());
        assert!(!Hostname::new("xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx").valid());
        assert!(!Hostname::new(
            "au-xph5-rvgrdsb5hcxc-47et3a5vvkrc-server-wyoz4elpdpe3.openstack.local"
        )
        .valid());
        assert!(Hostname::new("local--host.localdomain").valid());
        assert!(!Hostname::new("localhost-.localdomain").valid());
        assert!(!Hostname::new("localhost.-localdomain").valid());
        assert!(Hostname::new("localhost.localdomain").valid());
    }
}
