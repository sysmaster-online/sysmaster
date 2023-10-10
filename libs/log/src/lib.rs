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

//!
pub mod inner;
pub mod logger;

/// reexport log::Log
pub use log::max_level;
pub use log::set_max_level;
pub use log::Log;
pub use log::{Level, LevelFilter};
pub use log::{Metadata, MetadataBuilder};
pub use log::{Record, RecordBuilder};

pub use logger::get_open_when_needed;
pub use logger::set_open_when_needed;
/// Reinit the logger based on the previous configuration
pub fn reinit() {
    inner::reinit();
}

pub use logger::init_log;
/// Initialize console and syslog logger.
pub fn init_log_to_console(name: &str, level: crate::Level) {
    init_log(name, level, vec!["console"], "", 0, 0, false);
}

/// Initialize console and syslog logger.
pub fn init_log_to_console_syslog(name: &str, level: crate::Level) {
    init_log(name, level, vec!["console", "syslog"], "", 0, 0, false);
}

/// Initialize console and syslog logger.
pub fn init_log_to_file(
    name: &str,
    level: crate::Level,
    file_path: &str,
    file_size: u32,
    file_number: u32,
    open_when_needed: bool,
) {
    init_log(
        name,
        level,
        vec!["file"],
        file_path,
        file_size,
        file_number,
        open_when_needed,
    );
}

/// Initialize kmsg logger.
pub fn init_log_to_kmsg(name: &str, level: crate::Level) {
    init_log(name, level, vec!["kmsg"], "", 0, 0, false);
}

/// Initialize kmsg and console logger.
pub fn init_log_to_kmsg_console(name: &str, level: crate::Level) {
    init_log(name, level, vec!["kmsg", "console"], "", 0, 0, false);
}

#[cfg(test)]
mod tests {
    use crate::{init_log, reinit, Level};

    #[test]
    fn test_init_log_to_console() {
        init_log("test", Level::Debug, vec!["console"], "", 0, 0, false);
        crate::error!("hello, error!");
        crate::set_max_level(Level::Info.to_level_filter());
        crate::info!("hello, info!"); /* Won't print */
        crate::debug!("hello debug!");
        init_log("test", Level::Debug, vec!["syslog"], "", 0, 0, false);
        crate::debug!("hello debug2!"); /* Only print in the syslog */
        reinit();
        crate::debug!("hello debug3!"); /* Only print in the syslog */

        init_log(
            "test",
            Level::Debug,
            vec!["kmsg", "console"],
            "",
            0,
            0,
            false,
        );
        crate::info!("hello, kmsg!");
    }
}
