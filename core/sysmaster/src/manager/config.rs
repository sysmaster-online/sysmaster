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
//
#![allow(non_snake_case)]

use confique::{Config, FileFormat, Partial};

pub const SYSTEM_CONFIG: &str = "/etc/sysmaster/system.conf";
const RELI_HISTORY_MAPSIZE_DEFAULT: usize = 1048576; // 1M

#[derive(Config, Debug)]
pub struct ManagerConfig {
    #[config(default = 100)]
    pub DefaultRestartSec: u64,
    #[config(default = 90)]
    pub DefaultTimeoutSec: u64,

    #[config(default = "info")]
    pub LogLevel: String,
    #[config(default = "syslog")]
    pub LogTarget: String,
    #[config(default = 10240)]
    pub LogFileSize: u32,
    #[config(default = 10)]
    pub LogFileNumber: u32,

    #[config(default = 1048576)] // RELI_HISTORY_MAPSIZE_DEFAULT
    pub DbSize: usize,
}

impl ManagerConfig {
    #[allow(dead_code)]
    pub fn new(file: Option<&str>) -> ManagerConfig {
        type ConfigPartial = <ManagerConfig as Config>::Partial;
        let mut partial: ConfigPartial = match Partial::from_env() {
            Err(_) => return ManagerConfig::default(),
            Ok(v) => v,
        };
        partial = match confique::File::with_format(file.unwrap_or(SYSTEM_CONFIG), FileFormat::Toml)
            .load()
        {
            Err(_) => return ManagerConfig::default(),
            Ok(v) => partial.with_fallback(v),
        };
        partial = partial.with_fallback(ConfigPartial::default_values());
        match ManagerConfig::from_partial(partial) {
            Ok(v) => v,
            Err(_) => ManagerConfig::default(),
        }
    }
}

impl Default for ManagerConfig {
    fn default() -> Self {
        Self {
            DefaultRestartSec: 100,
            DefaultTimeoutSec: 90,
            LogLevel: "info".to_string(),
            LogTarget: "syslog".to_string(),
            LogFileSize: 10240,
            LogFileNumber: 10,
            DbSize: RELI_HISTORY_MAPSIZE_DEFAULT,
        }
    }
}

#[cfg(test)]
mod test {
    use libtests::get_crate_root;
    use std::path::PathBuf;

    use super::*;
    #[test]
    fn load() {
        let mut file: PathBuf = get_crate_root().unwrap();
        file.push("config/system.conf");
        let config = ManagerConfig::new(file.to_str());
        assert_eq!(config.DefaultRestartSec, 100);
    }
}
