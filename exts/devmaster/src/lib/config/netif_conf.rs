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

//! parse the configuration of network interface configuration
//!

use device::Device;
use fnmatch_sys::fnmatch;
use lazy_static::lazy_static;
use serde::Deserialize;
use std::{os::raw::c_char, path::Path, rc::Rc};

use crate::log_dev;

lazy_static! {
/// directories for searching rule files
pub(crate) static ref DEFAULT_NETIF_CONFIG_DIRS: Vec<String> = vec![
    "/etc/devmaster/network".to_string(),
    "/run/devmaster/network".to_string(),
    "/usr/local/lib/devmaster/network".to_string(),
    "/usr/lib/devmaster/network".to_string(),
];
}

#[derive(Debug)]
pub(crate) struct NetifConfig {
    pub(crate) inner: NetifConfigData,
    pub(crate) abs_path: String,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
pub(crate) struct NetifConfigData {
    pub(crate) r#Match: Match,
    pub(crate) Link: Link,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
pub(crate) struct Match {
    OriginalName: Option<String>,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
pub(crate) struct Link {
    pub(crate) NamePolicy: Option<Vec<String>>,
    pub(crate) AlternativeNamesPolicy: Option<Vec<String>>,
    pub(crate) MACAddressPolicy: Option<String>,
}

impl NetifConfigData {
    pub(crate) fn match_netif(&self, netif: Rc<Device>) -> bool {
        if let Some(original_name) = &self.Match.OriginalName {
            let pattern = format!("{}\0", original_name);

            match netif.get_sysname() {
                Ok(sysname) => {
                    let source = format!("{}\0", sysname);

                    if unsafe {
                        fnmatch(
                            pattern.as_ptr() as *const c_char,
                            source.as_ptr() as *const c_char,
                            0,
                        )
                    } != 0
                    {
                        return false;
                    }
                }
                Err(e) => {
                    log_dev!(error, netif, format!("Failed to get sysname: {}", e));
                    return false;
                }
            }
        }

        true
    }
}

#[derive(Debug)]
pub(crate) struct NetifConfigCtx {
    configs: Vec<NetifConfig>,
}

impl NetifConfigCtx {
    pub(crate) fn new() -> NetifConfigCtx {
        NetifConfigCtx { configs: vec![] }
    }

    pub(crate) fn load(&mut self, dirs: Vec<String>) {
        for dir in dirs.iter() {
            let dir_path = Path::new(dir);
            if !dir_path.is_dir() {
                continue;
            }

            let dir = match std::fs::read_dir(dir) {
                Ok(d) => d,
                Err(e) => {
                    log::error!("Failed to read directory '{}': {}", dir, e);
                    return;
                }
            };

            for entry in dir {
                let file = match entry {
                    Ok(entry) => entry,
                    Err(_) => {
                        continue;
                    }
                };

                let name = match file.file_name().to_str() {
                    Some(s) => s.to_string(),
                    None => {
                        continue;
                    }
                };

                if name.ends_with(".link") {
                    let conf_path = dir_path.join(&name);

                    log::debug!("loading .link config: {:?}", conf_path);

                    let s = match std::fs::read_to_string(&conf_path) {
                        Ok(content) => content,
                        Err(e) => {
                            log::error!("Failed to read '{}': {}", name, e);
                            continue;
                        }
                    };
                    let config: NetifConfigData = match toml::from_str(&s) {
                        Ok(config) => config,
                        Err(e) => {
                            log::error!("Failed to deserialize '{}': {}", name, e);
                            continue;
                        }
                    };
                    self.configs.push(NetifConfig {
                        inner: config,
                        abs_path: conf_path.to_str().unwrap_or_default().to_string(),
                    });
                }
            }
        }
    }

    pub(crate) fn get_config(&self, netif: Rc<Device>) -> Option<&NetifConfig> {
        self.configs
            .iter()
            .find(|&config| config.inner.match_netif(netif.clone()))
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs::{create_dir_all, remove_dir_all},
        io::Write,
    };

    use super::*;

    #[test]
    fn test_netif_conf_load() {
        let link = r#"
[Match]
OriginalName = "*"

[Link]
NamePolicy = ["keep", "kernel", "database", "onboard", "slot", "path"]
AlternativeNamesPolicy = ["database", "onboard", "slot", "path"]
MACAddressPolicy = "persistent"
"#;

        let link_config: NetifConfigData = toml::from_str(link).unwrap();
        assert_eq!(link_config.Match.OriginalName.unwrap(), "*");
        assert_eq!(
            link_config.Link.NamePolicy.unwrap(),
            vec!["keep", "kernel", "database", "onboard", "slot", "path"]
        );
        assert_eq!(
            link_config.Link.AlternativeNamesPolicy.unwrap(),
            vec!["database", "onboard", "slot", "path"]
        );
        assert_eq!(link_config.Link.MACAddressPolicy.unwrap(), "persistent");
    }

    #[test]
    fn test_netif_conf_ctx_load() {
        create_dir_all("/tmp/test_netif_conf_ctx_load").unwrap();

        let mut f = std::fs::File::create("/tmp/test_netif_conf_ctx_load/test.link").unwrap();
        f.write_all(
            br#"
            [Match]
            OriginalName = "*"

            [Link]
            NamePolicy = ["keep", "kernel", "database", "onboard", "slot", "path"]
            AlternativeNamesPolicy = ["database", "onboard", "slot", "path"]
            MACAddressPolicy = "persistent"
            "#,
        )
        .unwrap();

        let mut link_ctx = NetifConfigCtx::new();
        link_ctx.load(vec!["/tmp/test_netif_conf_ctx_load".to_string()]);

        assert_eq!(link_ctx.configs.len(), 1);

        assert_eq!(
            link_ctx.configs[0].inner.Match.OriginalName,
            Some("*".to_string())
        );
        assert_eq!(
            link_ctx.configs[0].inner.Link.NamePolicy,
            Some(vec![
                "keep".to_string(),
                "kernel".to_string(),
                "database".to_string(),
                "onboard".to_string(),
                "slot".to_string(),
                "path".to_string()
            ])
        );
        assert_eq!(
            link_ctx.configs[0].inner.Link.AlternativeNamesPolicy,
            Some(vec![
                "database".to_string(),
                "onboard".to_string(),
                "slot".to_string(),
                "path".to_string()
            ])
        );
        assert_eq!(
            link_ctx.configs[0].inner.Link.MACAddressPolicy,
            Some("persistent".to_string())
        );

        remove_dir_all("/tmp/test_netif_conf_ctx_load").unwrap();
    }
}
