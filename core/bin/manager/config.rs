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
use confique::Config;

pub const SYSTEM_CONFIG: &str = "/etc/sysmaster/system.toml";

#[derive(Config, Default, Debug)]
pub struct ManagerConfig {
    #[config(nested)]
    pub Manager: SectionManager,
}

#[derive(Config, Default, Debug)]
pub struct SectionManager {
    #[config(default = 100)]
    pub DefaultRestartSec: u64,
    #[config(default = 90)]
    pub DefaultTimeoutSec: u64,
}

impl ManagerConfig {
    #[allow(dead_code)]
    pub fn new(file: Option<&str>) -> ManagerConfig {
        let builder = ManagerConfig::builder().env();
        let manager_config = builder.file(file.unwrap_or(SYSTEM_CONFIG));
        match manager_config.load() {
            Ok(m) => m,
            _ => ManagerConfig::default(),
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
        file.push("config/system.toml");
        let config = ManagerConfig::new(file.to_str());
        println!("{config:?}");
        assert_eq!(config.Manager.DefaultRestartSec, 100);
    }
}
