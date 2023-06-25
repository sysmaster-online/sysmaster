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
use log::{LevelFilter, Log};
use log4rs::{
    append::{
        console::{ConsoleAppender, Target},
        rolling_file::{
            policy::compound::{
                roll::fixed_window::FixedWindowRoller, trigger::size::SizeTrigger, CompoundPolicy,
            },
            RollingFileAppender,
        },
        Append,
    },
    config::{Appender, Config, Logger, Root},
    encode::pattern::PatternEncoder,
};
use nix::libc;

/// sysmaster log parttern:
///
/// ```rust,ignore
/// {d(%Y-%m-%d %H:%M:%S)} {h({l}):<5} {M} {m}{n}
/// {d(%Y-%m-%d %H:%M:%S)}: log time, i.e. `2023-03-24 11:00:23`
/// {h({l}:<5)}: log level, 5 bytes
/// {M}: the method name where the logging request was issued
/// {m}: log message
/// {n}: separator character, '\n' in linux.
/// ```
pub const LOG_PATTERN: &str = "{d(%Y-%m-%d %H:%M:%S)} {h({l}):<5} {M} {m}{n}";

struct LogPlugin(log4rs::Logger);

impl log::Log for LogPlugin {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.0.enabled(metadata)
    }

    fn log(&self, record: &log::Record) {
        self.0.log(record);
    }

    fn flush(&self) {
        Log::flush(&self.0);
    }
}

struct SysLogger;

/* This is an extremely simple implementation, and only
 * supports the very basic log function. */
impl log::Log for SysLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        let msg = record.args().to_string();
        let level = match record.level() {
            log::Level::Error => libc::LOG_ERR,
            log::Level::Warn => libc::LOG_WARNING,
            log::Level::Info => libc::LOG_INFO,
            log::Level::Debug => libc::LOG_DEBUG,
            /* The highest libc log level is LOG_DEBUG */
            log::Level::Trace => libc::LOG_DEBUG,
        };
        unsafe {
            libc::syslog(level, msg.as_ptr() as *const libc::c_char);
        }
    }

    fn flush(&self) {}
}

fn append_log(
    app_name: &str,
    level: LevelFilter,
    target: &str,
    file_path: &str,
    file_size: u32,
    file_number: u32,
) {
    if target == "syslog" {
        let _ = log::set_boxed_logger(Box::new(SysLogger));
        log::set_max_level(level);
        return;
    }
    let config = build_log_config(app_name, level, target, file_path, file_size, file_number);
    let logger = log4rs::Logger::new(config);
    log::set_max_level(level);
    let _ = log::set_boxed_logger(Box::new(LogPlugin(logger)));
}

/// Init and set the sub unit manager's log
///
/// [`app_name`]: which app output the log
///
/// level:  maximum log level
///
/// target: log target
///
/// file_path: file path if the target is set to file
pub fn init_log_for_subum(
    app_name: &str,
    level: LevelFilter,
    target: &str,
    file_path: &str,
    file_size: u32,
    file_number: u32,
) {
    /* We should avoid calling init_log here, or we will get many "attempted
     * to set a logger after the logging system was already initialized" error
     * message. */
    append_log(app_name, level, target, file_path, file_size, file_number);
}

/// Init and set the log target to console
///
/// [`app_name`]: which app output the log
///
/// level: maximum log level
pub fn init_log_to_console(app_name: &str, level: LevelFilter) {
    init_log(app_name, level, "console", "", 0, 0);
}

/// Init and set the log target to file
///
/// [`app_name`]: which app output the log
///
/// level: maximum log level
///
/// file_path: log to which file
pub fn init_log_to_file(
    app_name: &str,
    level: LevelFilter,
    file_path: &str,
    file_size: u32,
    file_number: u32,
) {
    init_log(app_name, level, "file", file_path, file_size, file_number);
}

/// Init and set the logger
///
/// [`app_name`]: which app output the log
///
/// level:  maximum log level
///
/// target: log target
///
/// file_path: file path if the target is set to file
pub fn init_log(
    app_name: &str,
    level: LevelFilter,
    target: &str,
    file_path: &str,
    file_size: u32,
    file_number: u32,
) {
    if target == "syslog" {
        let _ = log::set_boxed_logger(Box::new(SysLogger));
        log::set_max_level(level);
        return;
    }
    let config = build_log_config(app_name, level, target, file_path, file_size, file_number);
    let r = log4rs::init_config(config);
    if let Err(e) = r {
        println!("{e}");
    }
}

fn build_log_config(
    app_name: &str,
    level: LevelFilter,
    target: &str,
    file_path: &str,
    file_size: u32,
    file_number: u32,
) -> Config {
    let mut target = target;
    /* If the file is configured to None, use console forcely. */
    if (file_path.is_empty() || file_size == 0 || file_number == 0) && target == "file" {
        println!(
            "LogTarget is configured to `file`, but configuration is invalid, changing the \
             LogTarget to `console`, file: {file_path}, file_size: {file_size}, file_number: \
             {file_number}"
        );
        target = "console";
    }
    let encoder = Box::new(PatternEncoder::new(LOG_PATTERN));
    let appender: Box<dyn Append> = match target {
        "console" => Box::new(
            ConsoleAppender::builder()
                .encoder(encoder)
                .target(Target::Stdout)
                .build(),
        ),
        "file" => {
            let pattern = file_path.to_string() + &".{}";
            let policy = Box::new(CompoundPolicy::new(
                Box::new(SizeTrigger::new(file_size as u64 * 1024)),
                Box::new(
                    FixedWindowRoller::builder()
                        .build(&pattern, file_number)
                        .unwrap(),
                ),
            ));
            Box::new(
                RollingFileAppender::builder()
                    .encoder(encoder)
                    .build(file_path, policy)
                    .unwrap(),
            )
        }
        _ => Box::new(
            ConsoleAppender::builder()
                .encoder(encoder)
                .target(Target::Stdout)
                .build(),
        ),
    };
    let logger = Logger::builder().build(app_name, level);
    let root = Root::builder().appender(target).build(level);
    Config::builder()
        .appender(Appender::builder().build(target, appender))
        .logger(logger)
        .build(root)
        .unwrap()
}

#[cfg(test)]

mod tests {
    use super::*;
    use log;
    #[test]
    fn test_init_log_to_console() {
        init_log_to_console("test", LevelFilter::Debug);
        // assert_eq!((), ());
        log::info!("test for logger info");
        log::error!("test for logger error");
        log::warn!("test for logger warn");
        log::debug!("test for logger debug");
        log::trace!("test for logger trace");
    }
}
