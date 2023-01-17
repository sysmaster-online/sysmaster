//! utils of libdevmaster
//!
use std::io::{self, Write};
use std::time::SystemTime;

/// set the global log level
const LOG_LEVEL: LogLevel = LogLevel::Info;

/// log level
#[derive(PartialEq)]
enum LogLevel {
    Info,
    Debug,
}

/// prefix of every log
pub fn log_prefix() -> String {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => {
            format!("[{}] devmaster: ", n.as_secs())
        }
        Err(_) => {
            panic!("SystemTime before UNIX EPOCH!");
        }
    }
}

/// log debug message
pub fn log_debug(msg: String) {
    if LOG_LEVEL != LogLevel::Debug {
        return;
    }

    io::stdout()
        .write_all(format!("{}{msg}", log_prefix()).as_bytes())
        .expect("Failed to write to stdout");
    io::stdout().flush().expect("Failed to flush stdout");
}

/// log info message
pub fn log_info(msg: String) {
    io::stdout()
        .write_all(format!("{}{msg}", log_prefix()).as_bytes())
        .expect("Failed to write to stdout");
    io::stdout().flush().expect("Failed to flush stdout");
}

/// log error message
pub fn log_error(msg: String) {
    io::stderr()
        .write_all(format!("{}{msg}", log_prefix()).as_bytes())
        .expect("Failed to write to stderr");
    io::stderr().flush().expect("Failed to flush stderr");
}

/// Error kinds of devmaster
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Error kind for worker manager
    #[error("Worker Manager: {}", msg)]
    WorkerManagerError {
        ///
        msg: &'static str,
    },

    /// Error kind for job queue
    #[error("Job Queue: {}", msg)]
    JobQueueError {
        ///
        msg: &'static str,
    },

    /// Error kind for control manager
    #[error("Control Manager: {}", msg)]
    ControlManagerError {
        ///
        msg: &'static str,
    },
}
