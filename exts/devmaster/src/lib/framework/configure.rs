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

use crate::rules::rule_load::DEFAULT_RULES_DIRS;
use confique::Config;
use serde::{Deserialize, Serialize};

pub(crate) const DEFAULT_CONFIG: &str = "/etc/devmaster/config.toml";

#[derive(Config, Serialize, Deserialize, Debug)]
pub(crate) struct Conf {
    pub(crate) rules_d: Vec<String>,
}

impl Default for Conf {
    fn default() -> Self {
        Conf {
            rules_d: DEFAULT_RULES_DIRS.to_vec(),
        }
    }
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
        assert_eq!(config.rules_d, vec!["/root/rules.d".to_string()]);
        fs::remove_file("./tmp.toml").unwrap();

        let default_conf = Conf::builder().load().unwrap_or_default();
        assert_eq!(default_conf.rules_d, DEFAULT_RULES_DIRS.to_vec());
    }
}
