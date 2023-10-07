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

use std::io::Result;
use std::{fs::File, io::Read, path::Path};

#[derive(Debug)]
pub struct Config {
    pub path: String,
    pub timecnt: usize,
    pub timewait: u64,
    pub bin: String,
}

impl Config {
    fn parse_config_line(line: &str) -> Option<(String, String)> {
        let mut iter = line.splitn(2, '=');
        let key = iter.next()?.trim();
        let value = iter.next()?.trim();

        Some((key.to_string(), value.to_string()))
    }

    fn parse_content(&mut self, content: &str) {
        for (_, line) in content.lines().enumerate() {
            let trimmed_line = line.trim();
            if trimmed_line.is_empty() || trimmed_line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = Config::parse_config_line(trimmed_line) {
                if value.is_empty() {
                    log::warn!("Do not have values for {}", key);
                    continue;
                }
                self.parse_value(key.as_str(), value.as_str());
            }
        }
    }

    fn parse_value(&mut self, key: &str, value: &str) {
        match key {
            "timecnt" => match value.parse::<usize>() {
                Ok(v) => self.timecnt = v,
                Err(e) => {
                    log::warn!(
                        "Parse timecnt failed: {}, use default({})!",
                        e,
                        self.timecnt
                    );
                }
            },
            "timewait" => match value.parse::<u64>() {
                Ok(v) => self.timewait = v,
                Err(e) => {
                    log::warn!(
                        "Parse timewait failed: {}, use default({})!",
                        e,
                        self.timewait
                    );
                }
            },
            "bin" => match value.parse::<String>() {
                Ok(v) => self.bin = v,
                Err(e) => {
                    log::warn!("Parse bin failed: {}, use default({})!", e, self.bin);
                }
            },
            _ => log::warn!("Parse error, use default config: {:?}!", self),
        }
    }

    pub fn load(path: Option<String>) -> Result<Self> {
        let mut config = Self::default();
        let path = path.unwrap_or_else(|| config.path.clone());

        if !Path::new(&path).exists() {
            return Ok(config);
        }

        let mut content = String::new();
        if let Err(e) = File::open(&path).map(|mut f| f.read_to_string(&mut content)) {
            log::info!("Failed to read {}: {}, use default value", path, e);
            return Ok(config);
        };
        config.parse_content(&content);
        log::debug!("Loaded config: {:?}", config);

        Ok(config)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            path: "/etc/sysmaster/init.conf".to_string(),
            timecnt: 10,
            timewait: 90,
            #[cfg(test)]
            bin: "ls".to_string(),
            #[cfg(not(test))]
            bin: "/usr/lib/sysmaster/sysmaster".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_load_file() {
        use std::io::Write;
        let content = "
#[config(default = 10)]
timecnt = 9
#[config(default = 90)]
    timewait =1
#[config(default = \"/usr/lib/sysmaster/sysmaster\")]
bin = /bin/ls
#[config(default = \"/run/sysmaster/init.sock\")]
socket = init.sock
";
        let file_path = "./init.conf";

        if let Ok(mut file) = std::fs::File::create(file_path) {
            if let Err(err) = file.write_all(content.as_bytes()) {
                eprintln!("Write file error: {}.", err);
            } else {
                println!("Success to write.");
            }
        } else {
            eprintln!("Failed to write file.");
        }
        let config = super::Config::load(Some(file_path.to_string())).unwrap();
        assert_eq!(config.timecnt, 9);
        assert_eq!(config.timewait, 1);
        assert_eq!(config.bin, "/bin/ls");
        std::fs::remove_file(file_path).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_parse_fail() {
        let content = "
#[config(default = 10)]
timecnt = 9
#[config(default = 90)]
    timewait =1s
#[config(default = \"/usr/lib/sysmaster/sysmaster\")]
bin = /bin/ls
#[config(default = \"/run/sysmaster/init.sock\")]
socket = init.sock
";
        let mut config = super::Config::default();
        config.parse_content(content);
        assert_eq!(config.timecnt, 9);
        assert_eq!(config.timewait, 1);
        assert_eq!(config.bin, "/bin/ls");
    }

    #[test]
    #[should_panic]
    fn test_parse_fail2() {
        let content = "
#[config(default = 10)]
timecnt = 9
#[config(default = 90)]
    timewait =1=
#[config(default = \"/usr/lib/sysmaster/sysmaster\")]
bin = /bin/ls
#[config(default = \"/run/sysmaster/init.sock\")]
socket = init.sock
";
        let mut config = super::Config::default();
        config.parse_content(content);
        assert_eq!(config.timecnt, 9);
        assert_eq!(config.timewait, 1);
        assert_eq!(config.bin, "/bin/ls");
    }

    #[test]
    fn test_parse_success() {
        let content = "
#[config(default = 10)]
timecnt = 9
#[config(default = 90)]
    timewait =1
#[config(default = \"/usr/lib/sysmaster/sysmaster\")]
bin = /bin/ls
#[config(default = \"/run/sysmaster/init.sock\")]
socket = \"init.sock\"
";
        let mut config = super::Config::default();
        config.parse_content(content);
        assert_eq!(config.timecnt, 9);
        assert_eq!(config.timewait, 1);
        assert_eq!(config.bin, "/bin/ls");
    }

    #[test]
    fn test_parse_multi() {
        let content = "
#[config(default = 10)]
timecnt = 9
#[config(default = 90)]
    timewait =1
    timewait =1
#[config(default = \"/usr/lib/sysmaster/sysmaster\")]
bin = /bin/ls
#[config(default = \"/run/sysmaster/init.sock\")]
socket = \"init.sock\"
socket
";
        let mut config = super::Config::default();
        config.parse_content(content);
        assert_eq!(config.timecnt, 9);
        assert_eq!(config.timewait, 1);
        assert_eq!(config.bin, "/bin/ls");
    }

    #[test]
    fn test_load_fail_defconfig() {
        let config = super::Config::load(Some("/path/to/init.conf".to_string())).unwrap();
        assert_eq!(config.timecnt, 10);
        assert_eq!(config.timewait, 90);
        assert_eq!(config.bin, "ls");
    }
}
