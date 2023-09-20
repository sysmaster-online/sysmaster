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
use log::Log;
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    io::{Error, ErrorKind},
    os::unix::{
        net::UnixDatagram,
        prelude::{OpenOptionsExt, PermissionsExt},
    },
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Mutex,
    },
};

pub use crate::inner::Level;
pub use crate::inner::{debug, error, info, trace, warn};

static mut LOG_LEVEL: AtomicU8 = AtomicU8::new(4);
static mut OPEN_WHEN_NEEDED: AtomicBool = AtomicBool::new(false);

/// Logger instance should implement `ReInit` too.
pub trait ReInit: Log {
    /// Define how logger instance reinitializes.
    fn reinit(&self) {}
}

fn write_msg_common(writer: &mut impl Write, module: &str, msg: String) {
    let time: libc::time_t = unsafe { libc::time(std::ptr::null_mut()) };
    let now = unsafe { libc::localtime(&time) };
    let now_str = unsafe {
        format!(
            "{:0>4}-{:0>2}-{:0>2} {:0>2}:{:0>2}:{:0>2} ",
            (*now).tm_year + 1900, /* tm_year is years since 1900 */
            (*now).tm_mon + 1,     /* tm_mon is months since Jan: [0, 11] */
            (*now).tm_mday,
            (*now).tm_hour,
            (*now).tm_min,
            (*now).tm_sec
        )
    };

    /* 1. Write time */
    if let Err(e) = writer.write(now_str.as_bytes()) {
        println!("Failed to log time message: {}", e);
        return;
    }

    /* 2. Write module */
    if let Err(e) = writer.write((module.to_string() + " ").as_bytes()) {
        println!("Failed to log module message: {}", e);
        return;
    }

    /* 3. Write message */
    if let Err(e) = writer.write((msg + "\n").as_bytes()) {
        println!("Failed to log message: {}", e);
    }
}

fn write_msg_file(writer: &mut File, module: &str, msg: String) {
    write_msg_common(writer, module, msg);
}

struct SysLogger {
    dgram: Mutex<Option<UnixDatagram>>,
}

impl SysLogger {
    fn new() -> Result<Self, std::io::Error> {
        if get_open_when_needed() {
            Ok(Self {
                dgram: Mutex::new(None),
            })
        } else {
            let dgram = Self::connect()?;
            Ok(Self {
                dgram: Mutex::new(Some(dgram)),
            })
        }
    }

    fn connect() -> Result<UnixDatagram, std::io::Error> {
        let sock = UnixDatagram::unbound()?;
        sock.connect("/dev/log")?;
        Ok(sock)
    }
}

impl ReInit for SysLogger {
    fn reinit(&self) {
        if get_open_when_needed() {
            *self.dgram.lock().expect("failed to lock syslogger") = None;
            return;
        }

        let dgr = match UnixDatagram::unbound() {
            Ok(dgr) => dgr,
            Err(e) => {
                eprintln!("Failed to bound unix datagram: {}", e);
                return;
            }
        };

        if let Err(e) = dgr.connect("/dev/log") {
            eprintln!("Failed to connect /dev/log: {}", e);
            return;
        }

        *self.dgram.lock().expect("failed to lock syslogger") = Some(dgr);
    }
}

/* This is an extremely simple implementation, and only
 * supports the very basic log function. */
impl log::Log for SysLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        let mut msg = String::new();
        msg += match record.module_path() {
            None => "unknown",
            Some(v) => v,
        };
        msg += " ";
        msg += &record.args().to_string();

        if get_open_when_needed() {
            match Self::connect() {
                Ok(dgr) => {
                    if let Err(e) = dgr.send(msg.as_bytes()) {
                        eprintln!("Failed to send message to syslogger: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to connect syslogger: {}", e);
                }
            }
        } else {
            match self
                .dgram
                .lock()
                .expect("Failed to lock syslogger")
                .as_ref()
            {
                Some(dgr) => {
                    if let Err(e) = dgr.send(msg.as_bytes()) {
                        eprintln!("Failed to send message to syslogger: {}", e);
                    }
                }
                None => {
                    eprintln!("open_when_needed is unset but syslogger is invalid.");
                }
            }
        }
    }

    fn flush(&self) {}
}

struct ConsoleLogger;

impl ReInit for ConsoleLogger {}

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
    file_mode: u32,
    file_number: u32,
    max_size: u32,
    file: Mutex<Option<File>>,
}

impl ReInit for FileLogger {
    fn reinit(&self) {
        if get_open_when_needed() {
            *self.file.lock().expect("failed to lock filelogger") = None;
            return;
        }

        match Self::file_open(self.file_path.as_path(), self.file_mode) {
            Ok(file) => *self.file.lock().expect("failed to lock filelogger") = Some(file),
            Err(e) => {
                eprintln!(
                    "Failed to open log file '{}': {}",
                    self.file_path.display(),
                    e
                );
            }
        }
    }
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

            match &mut *file {
                Some(file) => {
                    write_msg_file(file, module_path, record.args().to_string());
                    current_size = match file.metadata() {
                        Err(_) => return,
                        Ok(v) => v.len() as u32,
                    };
                }
                None => {
                    if !get_open_when_needed() {
                        eprintln!("open_when_needed is unset but file logger is invalid.");
                        return;
                    }
                    match FileLogger::file_open(&self.file_path, self.file_mode) {
                        Ok(mut file) => {
                            write_msg_file(&mut file, module_path, record.args().to_string());
                            current_size = match file.metadata() {
                                Err(_) => return,
                                Ok(v) => v.len() as u32,
                            }
                        }
                        Err(e) => {
                            println!("Failed to open the log file:{}.", e);
                            return;
                        }
                    };
                }
            }
            /* file is automatically unlocked. */
        }
        if current_size > self.max_size {
            let file = match self.file.lock() {
                Err(_) => return,
                Ok(v) => v,
            };

            if let Err(e) = self.rotate() {
                println!("Failed to rotate log file: {}", e);
            }

            match &*file {
                Some(file) => {
                    if let Err(e) = file.set_len(0) {
                        println!("Failed to clear log file: {}", e);
                    }
                }
                None => {
                    if !get_open_when_needed() {
                        return;
                    }
                    match FileLogger::file_open(&self.file_path, self.file_mode) {
                        Ok(file) => {
                            if let Err(e) = file.set_len(0) {
                                println!("Failed to clear log file: {}", e);
                            }
                        }
                        Err(e) => {
                            println!("Failed to open the log file:{}.", e);
                        }
                    };
                }
            }
        }
    }

    fn flush(&self) {
        let mut file = match self.file.lock() {
            Err(_) => return,
            Ok(v) => v,
        };
        match &mut *file {
            Some(file) => {
                if let Err(e) = file.flush() {
                    println!("Failed to flush log file: {}", e);
                }
            }
            None => {
                if !get_open_when_needed() {
                    return;
                }
                match FileLogger::file_open(&self.file_path, self.file_mode) {
                    Ok(mut file) => {
                        if let Err(e) = file.flush() {
                            println!("Failed to flush log file: {}", e);
                        }
                    }
                    Err(e) => {
                        println!("Failed to open file for flushing log file: {}", e);
                    }
                };
            }
        }
    }
}

impl FileLogger {
    fn file_open(file_path: &Path, file_mode: u32) -> Result<File, Error> {
        let dir = file_path.parent().unwrap();
        if !dir.exists() {
            if let Err(e) = fs::create_dir_all(dir) {
                println!("Failed to create dir {}: {:?}", dir.to_string_lossy(), e);
                return Err(e);
            };
        }

        if let Err(e) = fs::set_permissions(dir, fs::Permissions::from_mode(0o700)) {
            println!(
                "Failed to set permissions of dir {}: {:?}",
                dir.to_string_lossy(),
                e
            );
            return Err(e);
        }

        match OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .mode(file_mode)
            .open(file_path)
        {
            Err(e) => {
                println!(
                    "Failed to open file {} for log: {:?}",
                    file_path.to_string_lossy(),
                    e
                );
                Err(e)
            }
            Ok(v) => Ok(v),
        }
    }

    fn new(
        level: log::Level,
        file_path: PathBuf,
        file_mode: u32,
        max_size: u32,
        file_number: u32,
    ) -> Result<Self, Error> {
        if get_open_when_needed() {
            return Ok(Self {
                level,
                file_path,
                file_mode,
                file_number,
                max_size: max_size * 1024,
                file: Mutex::new(None),
            });
        }

        let file = Self::file_open(&file_path, file_mode)?;

        Ok(Self {
            level,
            file_path,
            file_mode,
            file_number,
            max_size: max_size * 1024,
            file: Mutex::new(Some(file)),
        })
    }

    fn mv_file_in_dir(src: &str, dst: Option<&str>, dir: &Path) {
        let src = dir.join(src);
        if dst.is_none() {
            if let Err(e) = fs::remove_file(src) {
                println!("Failed to remove old log file: {}", e);
            }
            return;
        }
        let dst = dir.join(dst.unwrap()); /* safe here */
        if let Err(e) = fs::rename(src, dst) {
            println!("Failed to rotate log file: {}", e);
        }
    }

    fn cp_file_in_dir(src: &str, dst: &str, dir: &Path) {
        let src = dir.join(src);
        let dst = dir.join(dst);
        if let Err(e) = fs::copy(src, &dst) {
            println!("Failed to create sysmaster.log.1: {}", e);
        }
        if let Err(e) = fs::set_permissions(dst, fs::Permissions::from_mode(0o400)) {
            println!("Failed to set log file mode: {}", e);
        }
    }

    fn rotate(&self) -> Result<(), Error> {
        let dir = match self.file_path.parent() {
            None => {
                return Err(Error::from(ErrorKind::NotFound));
            }
            Some(v) => v,
        };
        let file_name = match self.file_path.file_name() {
            None => {
                return Err(Error::from(ErrorKind::InvalidData));
            }
            Some(v) => v.to_string_lossy().to_string(),
        };
        let file_name_dot = String::from(&file_name) + ".";

        /* Walk through the parent directory, save the suffix rotate number in num_list */
        let mut num_list: Vec<usize> = Vec::new();
        let read_dir = match dir.read_dir() {
            Err(e) => return Err(e),
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

/// Collect different kinds of loggers together that implements `ReInit` trait.
///
/// Include: SysLogger, ConsoleLogger, FileLogger
struct CombinedLogger {
    loggers: Vec<Box<dyn ReInit>>,
}

impl ReInit for CombinedLogger {
    fn reinit(&self) {
        for logger in self.loggers.iter() {
            logger.as_ref().reinit()
        }
    }
}

impl Log for CombinedLogger {
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
    fn new() -> Self {
        Self {
            loggers: Vec::new(),
        }
    }

    fn push(&mut self, logger: Box<dyn ReInit>) {
        self.loggers.push(logger)
    }

    fn is_empty(&self) -> bool {
        self.loggers.is_empty()
    }
}

/// Initialize the global static logger instance.
/// Available log `targets` include `file`, `syslog`, `console`.
/// Arguments of `file_*` and `open_when_needed` only take effect on `file` target.
///
/// Repeated targets take effect only once.
///
/// # Arguments
///
/// * `name` - The application name that initializes the logger. Just used for debugging.
/// * `level` - Log message level.
/// * `targets` - A set of log targets.
/// * `file_path` - The log file path.
/// * `file_size` - Limit of the log file size. If the size of log file exceeds the limit, latter logs will override previous messages.
/// * `file_number` - The log file number.
/// * `open_when_needed` - If true, open the logger file just when logging messages.
pub fn init_log(
    name: &str,
    level: Level,
    targets: Vec<&str>,
    file_path: &str,
    file_size: u32,
    file_number: u32,
    open_when_needed: bool,
) {
    let level_num: u8 = match level {
        Level::Error => 1,
        Level::Warn => 2,
        Level::Info => 3,
        Level::Debug => 4,
        Level::Trace => 5,
    };
    crate::inner::set_max_level(level);

    let mut combined_loggers = CombinedLogger::new();

    for target in targets {
        let logger = match target {
            "console" => Box::new(ConsoleLogger) as Box<dyn ReInit>,
            "syslog" => match SysLogger::new() {
                Ok(logger) => Box::new(logger) as Box<dyn ReInit>,
                Err(e) => {
                    eprintln!("{} failed to create syslogger: {:?}", name, e);
                    continue;
                }
            },
            "file" => {
                match FileLogger::new(
                    log::Level::Debug,
                    PathBuf::from(file_path),
                    0o600,
                    file_size,
                    file_number,
                ) {
                    Ok(logger) => Box::new(logger) as Box<dyn ReInit>,
                    Err(e) => {
                        eprint!(
                            "{} failed to create '{}' file logger: {:?}",
                            name, file_path, e
                        );
                        continue;
                    }
                }
            }
            _ => {
                eprintln!("{}: log target '{}' is strange, ignoring.", name, target);
                continue;
            }
        };

        combined_loggers.push(logger);
    }

    if combined_loggers.is_empty() {
        eprintln!("{}: no available log targets.", name);
    }

    if let Err(e) = crate::inner::set_boxed_logger(Box::new(combined_loggers)) {
        eprintln!("{}: failed to set global logger: {:?}", name, e);
        return;
    }

    set_log_level(level_num);
    set_open_when_needed(open_when_needed);
}

/// Reinit the logger based on the previous configuration
pub fn reinit() {
    super::inner::reinit();
}

/// Initialize console and syslog logger.
pub fn init_log_to_console(name: &str, level: Level) {
    init_log(name, level, vec!["console"], "", 0, 0, false);
}

/// Initialize console and syslog logger.
pub fn init_log_to_console_syslog(name: &str, level: Level) {
    init_log(name, level, vec!["console", "syslog"], "", 0, 0, false);
}

/// Initialize console and syslog logger.
pub fn init_log_to_file(
    name: &str,
    level: Level,
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

/// Set the `OPEN_WHEN_NEEDED` flag.
pub fn set_open_when_needed(val: bool) {
    unsafe {
        OPEN_WHEN_NEEDED.store(val, Ordering::Release);
    }
}

/// Get the `OPEN_WHEN_NEEDED` flag.
pub fn get_open_when_needed() -> bool {
    unsafe { OPEN_WHEN_NEEDED.load(Ordering::Acquire) }
}

/// Set the `LOG_LEVEL`.
pub fn set_log_level(level: u8) {
    unsafe {
        LOG_LEVEL.store(level, Ordering::Release);
    }
}

/// Get the `LOG_LEVEL`.
pub fn get_log_level() -> u8 {
    unsafe { LOG_LEVEL.load(Ordering::Acquire) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_log_to_console() {
        init_log("test", Level::Debug, vec!["console"], "", 0, 0, false);
        crate::error!("hello, error!");
        crate::inner::set_max_level(Level::Info);
        crate::info!("hello, info!"); /* Won't print */
        crate::debug!("hello debug!");
        init_log("test", Level::Debug, vec!["syslog"], "", 0, 0, false);
        crate::debug!("hello debug2!"); /* Only print in the syslog */
        reinit();
        crate::debug!("hello debug3!"); /* Only print in the syslog */
    }
}
