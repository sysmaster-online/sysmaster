use kernlog::KernelLog;

pub struct Logger;

impl Logger {
    pub fn init(loglevel: log::LevelFilter) {
        match KernelLog::with_level(loglevel) {
            Ok(klog) => {
                log::set_boxed_logger(Box::new(klog)).expect("Failed to set logger!");
                log::set_max_level(loglevel);
            }
            Err(e) => {
                env_logger::builder().filter_level(loglevel).init();
                log::error!("Unsupported log into /dev/kmsg: {}, log into console!", e);
            }
        }
    }
}
