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

//! the management of the unit file lookup path

/// unit lookup path in /etc
pub const ETC_SYSTEM_PATH: &str = "/etc/sysmaster/system";
/// unit lookup path in /run
pub const RUN_SYSTEM_PATH: &str = "/run/sysmaster/system";
/// unit lookup path in /usr/lib
pub const LIB_SYSTEM_PATH: &str = "/usr/lib/sysmaster/system";

/// struct LookupPaths
#[derive(Debug, Clone)]
pub struct LookupPaths {
    /// Used to search fragment, dropin, updated
    pub search_path: Vec<String>,
    /// Used to search preset file
    pub preset_path: Vec<String>,
    /// generator paths
    pub generator: String,
    /// generator early paths
    pub generator_early: String,
    /// generator late paths
    pub generator_late: String,
    /// transient paths
    pub transient: String,
    /// transient paths
    pub persistent_path: String,
}

impl LookupPaths {
    /// new
    pub fn new() -> Self {
        LookupPaths {
            generator: String::from(""),
            generator_early: String::from(""),
            generator_late: String::from(""),
            transient: String::from(""),
            search_path: Vec::new(),
            persistent_path: String::from(""),
            preset_path: Vec::new(),
        }
    }

    /// init lookup paths
    pub fn init_lookup_paths(&mut self) {
        self.search_path.push(LIB_SYSTEM_PATH.to_string());
        self.search_path.push(RUN_SYSTEM_PATH.to_string());
        self.search_path.push(ETC_SYSTEM_PATH.to_string());

        self.preset_path
            .push(format!("{}/{}", ETC_SYSTEM_PATH, "system-preset"));
        self.preset_path
            .push(format!("{}/{}", LIB_SYSTEM_PATH, "system-preset"));

        self.persistent_path = ETC_SYSTEM_PATH.to_string();
    }
}

impl Default for LookupPaths {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::logger;

    use super::LookupPaths;
    #[test]
    fn test_init_lookup_paths() {
        logger::init_log_to_console("test_init_lookup_paths", log::LevelFilter::Trace);
        let mut lp = LookupPaths::default();
        lp.init_lookup_paths();
        assert_eq!(
            lp.search_path,
            vec![
                "/usr/lib/sysmaster/system",
                "/run/sysmaster/system",
                "/etc/sysmaster/system"
            ]
        );
        assert_eq!(
            lp.preset_path,
            vec![
                "/etc/sysmaster/system/system-preset",
                "/usr/lib/sysmaster/system/system-preset"
            ]
        );
        assert_eq!(lp.persistent_path, "/etc/sysmaster/system")
    }
}
