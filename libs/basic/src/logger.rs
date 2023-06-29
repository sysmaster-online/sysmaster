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
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    os::unix::prelude::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::Mutex,
};

use constants::LOG_FILE_PATH;
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
use time::macros::offset;

use crate::Error;

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

fn write_msg_common(writer: &mut impl Write, module: &str, msg: String) {
    let now = match time::OffsetDateTime::now_local() {
        Ok(v) => v,
        Err(_) => time::OffsetDateTime::now_utc().to_offset(offset!(+8)),
    };

    let format = time::macros::format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
    let now = match now.format(&format) {
        Err(_) => "[unknown time]".to_string(),
        Ok(v) => v,
    };

    /* 1. Write time */
    if let Err(e) = writer.write(format!("{now} ").as_bytes()) {
        println!("Failed to log time message: {e}");
        return;
    }

    /* 2. Write module */
    if let Err(e) = writer.write((module.to_string() + " ").as_bytes()) {
        println!("Failed to log module message: {e}");
        return;
    }

    /* 3. Write message */
    if let Err(e) = writer.write((msg + "\n").as_bytes()) {
        println!("Failed to log message: {e}");
    }
}

fn write_msg_file(writer: &mut File, module: &str, msg: String) {
    write_msg_common(writer, module, msg);
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

struct ConsoleLogger;

impl log::Log for ConsoleLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        let mut stdout = std::io::stdout();
        let module_path = match record.module_path() {
            None => "unknown",
            Some(v) => v,
        };
        write_msg_common(&mut stdout, module_path, record.args().to_string());
    }

    fn flush(&self) {}
}

struct FileLogger {
    level: log::Level,
    file_path: PathBuf,
    #[allow(dead_code)]
    file_mode: u32,
    file_number: u32,
    max_size: u32,
    file: Mutex<File>,
}

impl log::Log for FileLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let current_size: u32;
        {
            let mut file = match self.file.lock() {
                Err(_) => return,
                Ok(v) => v,
            };

            let module_path = match record.module_path() {
                None => "unknown",
                Some(v) => v,
            };
            write_msg_file(&mut file, module_path, record.args().to_string());
            current_size = match file.metadata() {
                Err(_) => return,
                Ok(v) => v.len() as u32,
            };
            /* file is automatically unlocked. */
        }
        if current_size > self.max_size {
            let file = match self.file.lock() {
                Err(_) => return,
                Ok(v) => v,
            };
            if let Err(e) = self.rotate() {
                println!("Failed to rotate log file: {e}");
            }
            if let Err(e) = file.set_len(0) {
                println!("Failed to clear log file: {e}");
            }
        }
    }

    fn flush(&self) {
        let mut file = match self.file.lock() {
            Err(_) => return,
            Ok(v) => v,
        };
        if let Err(e) = file.flush() {
            println!("Failed to flush log file: {e}");
        }
    }
}

impl FileLogger {
    fn file_open(file_path: &PathBuf, file_mode: u32) -> File {
        /* Panic if we can't open a log file. */
        let dir = file_path.parent().unwrap();
        if !dir.exists() {
            fs::create_dir_all(dir).unwrap();
        }
        fs::set_permissions(dir, fs::Permissions::from_mode(0o700)).unwrap();
        OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .mode(file_mode)
            .open(file_path)
            .unwrap()
    }

    fn new(
        level: log::Level,
        file_path: PathBuf,
        file_mode: u32,
        max_size: u32,
        file_number: u32,
    ) -> Self {
        let file = Self::file_open(&file_path, file_mode);
        Self {
            level,
            file_path,
            file_mode,
            file_number,
            max_size: max_size * 1024,
            file: Mutex::new(file),
        }
    }

    fn mv_file_in_dir(src: &str, dst: Option<&str>, dir: &Path) {
        let src = dir.join(src);
        if dst.is_none() {
            if let Err(e) = fs::remove_file(src) {
                println!("Failed to remove old log file: {e}");
            }
            return;
        }
        let dst = dir.join(dst.unwrap()); /* safe here */
        if let Err(e) = fs::rename(src, dst) {
            println!("Failed to rotate log file: {e}");
        }
    }

    fn cp_file_in_dir(src: &str, dst: &str, dir: &Path) {
        let src = dir.join(src);
        let dst = dir.join(dst);
        if let Err(e) = fs::copy(src, &dst) {
            println!("Failed to create sysmaster.log.1: {e}");
        }
        if let Err(e) = fs::set_permissions(dst, fs::Permissions::from_mode(0o400)) {
            println!("Failed to set log file mode: {e}");
        }
    }

    fn rotate(&self) -> Result<(), Error> {
        let dir = match self.file_path.parent() {
            None => {
                return Err(Error::Other {
                    msg: "Cannot determine the parent directory of log file".to_string(),
                })
            }
            Some(v) => v,
        };
        let file_name = match self.file_path.file_name() {
            None => {
                return Err(Error::Other {
                    msg: "Cannot determine the file name of log file".to_string(),
                })
            }
            Some(v) => v.to_string_lossy().to_string(),
        };
        let file_name_dot = String::from(&file_name) + ".";

        /* Walk through the parent directory, save the suffix rotate number in num_list */
        let mut num_list: Vec<usize> = Vec::new();
        let read_dir = match dir.read_dir() {
            Err(e) => return Err(Error::Io { source: e }),
            Ok(v) => v,
        };
        for de in read_dir {
            let de = match de {
                Err(_) => continue,
                Ok(v) => v,
            };

            let file_type = match de.file_type() {
                Err(_) => continue,
                Ok(v) => v,
            };
            if !file_type.is_file() {
                continue;
            }

            let de_file_name = de.file_name().to_string_lossy().to_string();
            let rotated_num = de_file_name.trim_start_matches(&file_name_dot);
            let rotated_num = match rotated_num.parse::<usize>() {
                Err(_) => {
                    continue;
                }
                Ok(v) => v,
            };
            num_list.push(rotated_num);
        }

        num_list.sort_unstable();

        /* 1. delete surplus rotated file */
        /* We only keep (file_number - 1) rotated files, because we will generate a new one later. */
        while num_list.len() > (self.file_number - 1) as usize {
            let num = num_list.pop().unwrap(); /* safe here */
            let src = String::from(&file_name_dot) + &num.to_string();
            Self::mv_file_in_dir(&src, None, dir);
        }

        /* 2. {sysmaster.log.1, sysmaster.log.2, ...} => {sysmaster.log.2, sysmaster.log.3, ...} */
        while let Some(num) = num_list.pop() {
            let src = String::from(&file_name_dot) + &num.to_string();
            let dst = String::from(&file_name_dot) + &(num + 1).to_string();
            Self::mv_file_in_dir(&src, Some(&dst), dir);
        }

        /* 3. **copy** sysmaster.log => sysmaster.log.1 */
        let src = String::from(&file_name);
        let dst = String::from(&file_name_dot) + "1";
        Self::cp_file_in_dir(&src, &dst, dir);
        Ok(())
    }
}

struct CombinedLogger {
    loggers: Vec<Box<dyn log::Log>>,
}

impl log::Log for CombinedLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        for logger in &self.loggers {
            logger.log(record);
        }
    }

    fn flush(&self) {}
}

impl CombinedLogger {
    fn empty() -> Self {
        Self {
            loggers: Vec::new(),
        }
    }

    fn push(&mut self, logger: Box<dyn Log>) {
        self.loggers.push(logger)
    }
}

/// Init and set the sub unit manager's log
///
/// [`app_name`]: which app output the log
///
/// level:  maximum log level
///
/// target: log target
///
/// file_size: the maximum size of an active log file (valid when target == "file")
///
/// file_number: the maximum number of rotated log files (valid when target == "file")
pub fn init_log_for_subum(
    app_name: &str,
    level: LevelFilter,
    target: &str,
    file_size: u32,
    file_number: u32,
) {
    /* We should avoid calling init_log here, or we will get many "attempted
     * to set a logger after the logging system was already initialized" error
     * message. */
    init_log(app_name, level, target, file_size, file_number);
}

/// Init and set the log target to console
///
/// [`app_name`]: which app output the log
///
/// level: maximum log level
pub fn init_log_to_console(app_name: &str, level: LevelFilter) {
    init_log(app_name, level, "console-syslog", 0, 0);
}

/// Init and set the log target to file
///
/// [`app_name`]: which app output the log
///
/// level: maximum log level
///
/// file_size: the maximum size of an active log file
///
/// file_number: the maximum number of rotated log files
pub fn init_log_to_file(app_name: &str, level: LevelFilter, file_size: u32, file_number: u32) {
    init_log(app_name, level, "file", file_size, file_number);
}

/// Init and set the logger
///
/// [`app_name`]: which app output the log
///
/// level:  maximum log level
///
/// target: log target
///
/// file_size: the maximum size of an active log file (valid when target == "file")
///
/// file_number: the maximum number of rotated log files (valid when target == "file")
pub fn init_log(
    _app_name: &str,
    level: LevelFilter,
    target: &str,
    file_size: u32,
    file_number: u32,
) {
    let mut target = target;
    if target == "file" && (file_size == 0 || file_number == 0) {
        println!(
            "LogTarget is configured to `file`, but configuration is invalid, changing the \
             LogTarget to `syslog`, file_size: {file_size}, file_number: {file_number}"
        );
        target = "syslog";
    }

    if target == "syslog" {
        let _ = log::set_boxed_logger(Box::new(SysLogger));
        log::set_max_level(level);
        return;
    }
    if target == "console-syslog" {
        let mut logger = CombinedLogger::empty();
        logger.push(Box::new(ConsoleLogger));
        logger.push(Box::new(SysLogger));
        let _ = log::set_boxed_logger(Box::new(logger));
        log::set_max_level(level);
        return;
    }
    if target == "file" {
        let _ = log::set_boxed_logger(Box::new(FileLogger::new(
            log::Level::Debug,
            PathBuf::from(LOG_FILE_PATH),
            0o600,
            file_size,
            file_number,
        )));
        log::set_max_level(level);
    }
}

#[allow(unused)]
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
    if (file_path.is_empty() || file_size == 0 || file_number == 0) && target == "rolling_file" {
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
        "rolling_file" => {
            let pattern = file_path.to_string() + ".{}";
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
