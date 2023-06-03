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

use confique::Config;
use lazy_static::lazy_static;
use log::LevelFilter;

/// default configuration path
pub(crate) const DEFAULT_CONFIG: &str = "/etc/devmaster/config.toml";

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
#[allow(missing_docs)]
#[derive(Config, Debug)]
pub(crate) struct Conf {
    pub(crate) rules_d: Option<Vec<String>>,
    #[config(default = 3)]
    pub(crate) children_max: u32,
    #[config(default = "info")]
    pub(crate) log_level: LevelFilter,
}

#[cfg(test)]
mod tests {
    use super::*;
    use confique::Config;
    use std::fs;

    #[test]
    fn test_config() {
        let config_s = "rules_d = [\"/root/rules.d\"]";
        fs::write("./tmp.toml", config_s).unwrap();
        let config: Conf = Conf::builder().file("./tmp.toml").load().unwrap();
        assert_eq!(config.rules_d.unwrap(), vec!["/root/rules.d".to_string()]);
        assert_eq!(config.children_max, 3);
        assert_eq!(config.log_level, LevelFilter::Info);
        fs::remove_file("./tmp.toml").unwrap();

        let default_conf = Conf::builder().load().unwrap();
        assert_eq!(default_conf.rules_d, None);
        assert_eq!(config.children_max, 3);
        assert_eq!(config.log_level, LevelFilter::Info);
    }
}
