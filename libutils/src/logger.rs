use log::LevelFilter;
use log4rs::{
    append::console::{ConsoleAppender, Target},
    config::{Appender, Config, Logger, Root},
    encode::pattern::PatternEncoder,
};
use std::path::Path;
/// Init logger whith config yaml file.
///
/// [`path`] the config file path
/// example
/// see docs.rs/log4rs/1.0.0/#examples
///
pub fn init_log_with_file<P>(path: P)
where
    //where 字句限定 P的类型
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
pub fn init_log_with_console(app_name: &str, log_level: u32) {
    let mut pattern = String::new();
    pattern += "{d(%Y-%m-%d %H:%M:%S)} ";
    pattern += "{h({l}):<5} ";
    pattern += "{m}{n}";
    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(&pattern)))
        .target(Target::Stderr)
        .build();
    let logging_builder =
        Config::builder().appender(Appender::builder().build("console", Box::new(stdout)));
    let l_level;
    match log_level {
        0 => l_level = LevelFilter::Error,
        1 => l_level = LevelFilter::Warn,
        2 => l_level = LevelFilter::Info,
        3 => l_level = LevelFilter::Debug,
        4 => l_level = LevelFilter::Trace,
        _ => l_level = LevelFilter::Info,
    }

    let config = match Some(app_name) {
        Some(a_p) => logging_builder
            .logger(Logger::builder().build(a_p, l_level))
            .build(Root::builder().appender("console").build(l_level)),
        _ => logging_builder.build(Root::builder().appender("console").build(l_level)),
    }
    .unwrap();
    log4rs::init_config(config).unwrap();
}

#[cfg(test)]

mod tests {
    use super::*;
    use log;
    #[test]
    fn test_init_log_with_console() {
        init_log_with_console("test", 4);
        assert_eq!((), ());
        log::info!("test for logger info");
        log::error!("test for logger error");
        log::warn!("test for logger warn");
        log::debug!("test for logger debug");
        log::trace!("test for logger trace");
    }
}
