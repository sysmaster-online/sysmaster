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

//! parse the configuration of devmaster
//!

use super::DEFAULT_NETIF_CONFIG_DIRS;
use confique::Config;
use lazy_static::lazy_static;
use log::Level;
use std::cell::RefCell;
use std::str::FromStr;

/// default configuration path
pub const DEFAULT_CONFIG: &str = "/etc/devmaster/config.toml";

lazy_static! {
/// directories for searching rule files
pub(crate) static ref DEFAULT_RULES_DIRS: Vec<String> = vec![
    "/etc/devmaster/rules.d".to_string(),
    "/run/devmaster/rules.d".to_string(),
    "/usr/local/lib/devmaster/rules.d".to_string(),
    "/usr/lib/devmaster/rules.d".to_string(),
];
}

/// configuration of devmaster
#[derive(Debug, Default)]
pub struct DevmasterConfig {
    /// rules directories
    inner: RefCell<DevmasterConfigData>,
}

#[derive(Debug, Config, Default)]
pub(crate) struct DevmasterConfigData {
    pub(crate) rules_d: Option<Vec<String>>,
    pub(crate) max_workers: Option<u32>,
    pub(crate) log_level: Option<String>,
    pub(crate) network_d: Option<Vec<String>>,
    pub(crate) log_targets: Option<Vec<String>>,
}

impl DevmasterConfig {
    /// generate a configuration object
    pub fn new() -> DevmasterConfig {
        DevmasterConfig {
            inner: RefCell::new(DevmasterConfigData::default()),
        }
    }

    /// load the configurations object
    pub fn load(&self, path: &str) {
        match DevmasterConfigData::builder().file(path).load() {
            Ok(data) => {
                let _ = self.inner.replace(data);
            }
            Err(e) => log::error!("Failed to load '{}': {}", path, e),
        }
    }

    /// get the rules directories
    pub fn get_max_workers(&self) -> u32 {
        self.inner.borrow().max_workers.unwrap_or(3)
    }

    /// get the rules directories
    pub fn get_log_level(&self) -> Level {
        match self.inner.borrow().log_level.clone() {
            Some(level) => Level::from_str(&level).unwrap(),
            None => Level::Info,
        }
    }

    /// get the rules directories
    pub fn get_rules_d(&self) -> Vec<String> {
        self.inner
            .borrow()
            .rules_d
            .clone()
            .unwrap_or_else(|| DEFAULT_RULES_DIRS.to_vec())
    }

    /// get the network interface configuration directories
    pub fn get_netif_cfg_d(&self) -> Vec<String> {
        self.inner
            .borrow()
            .network_d
            .clone()
            .unwrap_or_else(|| DEFAULT_NETIF_CONFIG_DIRS.to_vec())
    }

    /// Get log targets. If not set, use "console" by default.
    pub fn get_log_targets(&self) -> Vec<String> {
        self.inner
            .borrow()
            .log_targets
            .clone()
            .unwrap_or_else(|| vec!["console".to_string()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_config() {
        let config_s = "
rules_d = [\"/root/rules.d\"]
network_d = [\"/root/network.d\"]
";
        fs::write("/tmp/test_config.toml", config_s).unwrap();
        let config: DevmasterConfig = DevmasterConfig::new();
        config.load("/tmp/test_config.toml");

        assert_eq!(config.get_rules_d(), vec!["/root/rules.d".to_string()]);
        assert_eq!(config.get_max_workers(), 3);
        assert_eq!(config.get_log_level(), Level::Info);
        assert_eq!(
            config.get_netif_cfg_d(),
            vec!["/root/network.d".to_string()]
        );
        fs::remove_file("/tmp/test_config.toml").unwrap();

        let default_conf = DevmasterConfig::new();
        assert_eq!(default_conf.get_rules_d(), DEFAULT_RULES_DIRS.to_vec());
        assert_eq!(default_conf.get_max_workers(), 3);
        assert_eq!(default_conf.get_log_level(), Level::Info);
        assert_eq!(
            default_conf.get_netif_cfg_d(),
            DEFAULT_NETIF_CONFIG_DIRS.to_vec()
        );
    }
}
