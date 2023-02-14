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
use log::LevelFilter;
use log4rs::{
    append::console::{ConsoleAppender, Target},
    config::{Appender, Config, Logger, Root},
    encode::pattern::PatternEncoder,
};
use std::path::Path;

struct LoggerPlugin(log4rs::Logger);

impl log::Log for LoggerPlugin {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.0.enabled(metadata)
    }

    fn log(&self, record: &log::Record) {
        self.0.log(record);
    }

    fn flush(&self) {
        self.0.flush();
    }
}

fn set_logger(logger: log4rs::Logger) {
    log::set_max_level(LevelFilter::Trace);
    let _ = log::set_boxed_logger(Box::new(LoggerPlugin(logger)));
}

///
pub fn init_log_with_default(app_name: &str, log_level: LevelFilter) {
    let config = build_log_config(app_name, log_level);
    let logger = log4rs::Logger::new(config);
    set_logger(logger);
}

/// Init logger with config yaml file.
///
/// [`path`] the config file path
/// example
/// see docs.rs/log4rs/1.0.0/#examples
///
pub fn init_log_with_file<P>(path: P)
where
    //where
    P: AsRef<Path>,
{
    log4rs::init_file(path, Default::default()).expect("logging init");
}

/// Init logger output the log to console.
///
/// [`app_name`] which app output the log
/// log level set the output log level
/// [0] Error
/// [1] Warn
/// [2] Info.
/// [3] Debug
/// [4]  Trace
/// [others] Info
///
pub fn init_log_with_console(app_name: &str, log_level: LevelFilter) {
    let config = build_log_config(app_name, log_level);
    let log_init_result = log4rs::init_config(config);
    if let Err(e) = log_init_result {
        println!("{e}");
    }
}

fn build_log_config(app_name: &str, level: LevelFilter) -> Config {
    let mut pattern = String::new();
    pattern += "{d(%Y-%m-%d %H:%M:%S)} ";
    pattern += "{h({l}):<5} ";
    pattern += "{M} {m}{n}";
    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(&pattern)))
        .target(Target::Stderr)
        .build();
    let logging_builder =
        Config::builder().appender(Appender::builder().build("console", Box::new(stdout)));

    match Some(app_name) {
        Some(a_p) => logging_builder
            .logger(Logger::builder().build(a_p, level))
            .build(Root::builder().appender("console").build(level)),
        _ => logging_builder.build(Root::builder().appender("console").build(level)),
    }
    .unwrap()
}

#[cfg(test)]

mod tests {
    use super::*;
    use log;
    #[test]
    fn test_init_log_with_console() {
        init_log_with_console("test", LevelFilter::Debug);
        // assert_eq!((), ());
        log::info!("test for logger info");
        log::error!("test for logger error");
        log::warn!("test for logger warn");
        log::debug!("test for logger debug");
        log::trace!("test for logger trace");
    }
}
