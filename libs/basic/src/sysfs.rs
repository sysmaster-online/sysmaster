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

//! Common used sysfs functions
use std::path::Path;

///
pub struct SysFs;

impl SysFs {
    /// check if the system is running on AC Power
    pub fn on_ac_power() -> bool {
        /* 1 for true or failure, 0 for false. */
        let path = Path::new("/sys/class/power_supply");
        if !path.exists() || !path.is_dir() {
            return true;
        }
        let des = match path.read_dir() {
            Err(e) => {
                log::info!("Failed to walk /sys/class/power_supply: {}, ignoring", e);
                return true;
            }
            Ok(v) => v,
        };
        let mut found_online = false;
        let mut found_offline = false;
        for de in des {
            /* We only check the "type" file and "online" file in
             * each directory, and skip whenever failed. */
            let de = match de {
                Err(_) => {
                    continue;
                }
                Ok(v) => v,
            };
            let de_path = path.join(de.file_name());
            // 1. The content of "type" file should be "Mains".
            let contents = match std::fs::read(de_path.join("type")) {
                Err(_) => {
                    continue;
                }
                Ok(v) => v,
            };
            if !"Mains\n".to_string().into_bytes().eq(&contents) {
                continue;
            }
            // 2. The content of "online" file should be "0" or "1".
            let contents = match std::fs::read(de_path.join("online")) {
                Err(_) => {
                    continue;
                }
                Ok(v) => v,
            };
            if contents.len() != 2 || contents[1] != b'\n' {
                continue;
            }
            if contents[0] == b'1' {
                found_online = true;
                break;
            } else if contents[0] == b'0' {
                found_offline = true;
            } else {
                continue;
            }
        }
        found_online || !found_offline
    }
}
