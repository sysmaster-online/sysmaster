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
use constants::LOG_FILE_PATH;
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
        atomic::{AtomicU32, Ordering},
        Arc, Mutex,
    },
};

pub use crate::inner::Level;
pub use crate::inner::{debug, error, info, trace, warn};

static mut LOG_LEVEL: AtomicU32 = AtomicU32::new(4);
static mut LOG_TARGET: AtomicU32 = AtomicU32::new(0);
static mut LOG_FILE_SIZE: AtomicU32 = AtomicU32::new(1024);
static mut LOG_FILE_NUMBER: AtomicU32 = AtomicU32::new(5);

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
    dgram: Arc<Mutex<UnixDatagram>>,
}

impl SysLogger {
    fn connect() -> Result<Self, std::io::Error> {
        let sock = UnixDatagram::unbound()?;
        sock.connect("/dev/log")?;
        Ok(Self {
            dgram: Arc::new(Mutex::new(sock)),
        })
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

        let dgram = self.dgram.lock().unwrap();

        if let Err(e) = dgram.send(msg.as_bytes()) {
            println!("Failed to log message: {}", e);
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
    file: Mutex<Option<File>>,

    open_when_needed: bool,
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
                    if !self.open_when_needed {
                        println!("Failed to open the log file.");
                        return;
                    }
                    let mut file = FileLogger::file_open(&self.file_path, self.file_mode);
                    write_msg_file(&mut file, module_path, record.args().to_string());
                    current_size = match file.metadata() {
                        Err(_) => return,
                        Ok(v) => v.len() as u32,
                    }
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
                    if !self.open_when_needed {
                        return;
                    }
                    let file = FileLogger::file_open(&self.file_path, self.file_mode);
                    if let Err(e) = file.set_len(0) {
                        println!("Failed to clear log file: {}", e);
                    }
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
                if !self.open_when_needed {
                    return;
                }
                let mut file = FileLogger::file_open(&self.file_path, self.file_mode);
                if let Err(e) = file.flush() {
                    println!("Failed to flush log file: {}", e);
                }
            }
        }
    }
}

impl FileLogger {
    fn file_open(file_path: &Path, file_mode: u32) -> File {
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
        open_when_needed: bool,
    ) -> Self {
        if open_when_needed {
            return Self {
                level,
                file_path,
                file_mode,
                file_number,
                max_size: max_size * 1024,
                file: Mutex::new(None),
                open_when_needed: true,
            };
        }

        let file = Self::file_open(&file_path, file_mode);
        Self {
            level,
            file_path,
            file_mode,
            file_number,
            max_size: max_size * 1024,
            file: Mutex::new(Some(file)),
            open_when_needed: false,
        }
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
/// app_name: which app output the log
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
    level: Level,
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
/// app_name: which app output the log
///
/// level: maximum log level
pub fn init_log_to_console(app_name: &str, level: Level) {
    init_log(app_name, level, "console-syslog", 0, 0);
}

/// Init and set the log target to file
///
/// app_name: which app output the log
///
/// level: maximum log level
///
/// file_size: the maximum size of an active log file
///
/// file_number: the maximum number of rotated log files
pub fn init_log_to_file(app_name: &str, level: Level, file_size: u32, file_number: u32) {
    init_log(app_name, level, "file", file_size, file_number);
}

/// Init and set the logger
///
/// app_name: which app output the log
///
/// level:  maximum log level
///
/// target: log target
///
/// file_size: the maximum size of an active log file (valid when target == "file")
///
/// file_number: the maximum number of rotated log files (valid when target == "file")
pub fn init_log(_app_name: &str, level: Level, target: &str, file_size: u32, file_number: u32) {
    let level_num: u32 = match level {
        Level::Error => 0,
        Level::Warn => 1,
        Level::Info => 2,
        Level::Debug => 3,
        Level::Trace => 4,
    };

    let target_num: u32 = match target {
        "syslog" => 0,
        "console-syslog" => 1,
        "file" => 2,
        _ => 0,
    };

    let mut target = target;
    if target == "file" && (file_size == 0 || file_number == 0) {
        println!(
            "LogTarget is configured to `file`, but configuration is invalid, changing the \
             LogTarget to `syslog`, file_size: {}, file_number: {}",
            file_size, file_number
        );
        target = "syslog";
    }

    if target == "syslog" {
        match SysLogger::connect() {
            Ok(l) => {
                if let Err(e) = crate::inner::set_boxed_logger(Box::new(l)) {
                    eprint!("Failed to set logger: {:?}", e);
                }
            }
            Err(e) => {
                eprint!("Failed to connect to /dev/log: {:?}", e);
            }
        }
        crate::inner::set_max_level(level);

        unsafe {
            LOG_LEVEL.store(level_num, Ordering::SeqCst);
            LOG_TARGET.store(target_num, Ordering::SeqCst);
        }

        return;
    }

    if target == "console-syslog" {
        let mut logger = CombinedLogger::empty();
        logger.push(Box::new(ConsoleLogger));

        match SysLogger::connect() {
            Ok(l) => {
                logger.push(Box::new(l));
            }
            Err(e) => {
                eprint!("Failed to connect to /dev/log: {:?}", e);
            }
        }

        if let Err(e) = crate::inner::set_boxed_logger(Box::new(logger)) {
            eprintln!("Failed to set logger: {:?}", e);
        }
        crate::inner::set_max_level(level);

        unsafe {
            LOG_LEVEL.store(level_num, Ordering::SeqCst);
            LOG_TARGET.store(target_num, Ordering::SeqCst);
        }

        return;
    }

    if target == "file" {
        let _ = crate::inner::set_boxed_logger(Box::new(FileLogger::new(
            log::Level::Debug,
            PathBuf::from(LOG_FILE_PATH),
            0o600,
            file_size,
            file_number,
            false,
        )));
        crate::inner::set_max_level(level);

        unsafe {
            LOG_LEVEL.store(level_num, Ordering::SeqCst);
            LOG_TARGET.store(target_num, Ordering::SeqCst);
            LOG_FILE_NUMBER.store(file_number, Ordering::SeqCst);
            LOG_FILE_SIZE.store(file_size, Ordering::SeqCst);
        }
    }
}

/// Reinit the logger based on the former configuration
pub fn reinit(open_when_needed: bool) {
    let level = match unsafe { LOG_LEVEL.load(Ordering::SeqCst) } {
        0 => Level::Error,
        1 => Level::Warn,
        2 => Level::Info,
        3 => Level::Debug,
        4 => Level::Trace,
        _ => Level::Info,
    };

    let mut target: &str = match unsafe { LOG_TARGET.load(Ordering::SeqCst) } {
        0 => "syslog",
        1 => "console-syslog",
        2 => "file",
        _ => "syslog",
    };

    let file_size = unsafe { LOG_FILE_SIZE.load(Ordering::SeqCst) };
    let file_number = unsafe { LOG_FILE_NUMBER.load(Ordering::SeqCst) };

    if target == "file" && (file_size == 0 || file_number == 0) {
        println!(
            "LogTarget is configured to `file`, but configuration is invalid, changing the \
             LogTarget to `syslog`, file_size: {}, file_number: {}",
            file_size, file_number
        );
        target = "syslog";
    }

    if target == "syslog" {
        match SysLogger::connect() {
            Ok(l) => {
                if let Err(e) = crate::inner::set_boxed_logger(Box::new(l)) {
                    eprint!("Failed to set logger: {:?}", e);
                }
            }
            Err(e) => {
                eprint!("Failed to connect to /dev/log: {:?}", e);
            }
        }
        crate::inner::set_max_level(level);
        return;
    }

    if target == "console-syslog" {
        let mut logger = CombinedLogger::empty();
        logger.push(Box::new(ConsoleLogger));

        match SysLogger::connect() {
            Ok(l) => {
                logger.push(Box::new(l));
            }
            Err(e) => {
                eprint!("Failed to connect to /dev/log: {:?}", e);
            }
        }

        if let Err(e) = crate::inner::set_boxed_logger(Box::new(logger)) {
            eprintln!("Failed to set logger: {:?}", e);
        }
        crate::inner::set_max_level(level);
        return;
    }

    if target == "file" {
        let _ = crate::inner::set_boxed_logger(Box::new(FileLogger::new(
            log::Level::Debug,
            PathBuf::from(LOG_FILE_PATH),
            0o600,
            file_size,
            file_number,
            open_when_needed,
        )));
        crate::inner::set_max_level(level);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_init_log_to_console() {
        init_log_to_console("test", Level::Debug);
        crate::error!("hello, error!");
        crate::inner::set_max_level(Level::Info);
        crate::info!("hello, info!"); /* Won't print */
        crate::debug!("hello debug!");
        init_log("test", Level::Debug, "syslog", 0, 0);
        crate::debug!("hello debug2!"); /* Only print in the syslog */
        reinit(true);
        crate::debug!("hello debug3!"); /* Only print in the syslog */
    }
}
