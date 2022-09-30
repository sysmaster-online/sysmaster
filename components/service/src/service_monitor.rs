use super::service_config::ServiceConfig;
use std::cell::RefCell;
use std::rc::Rc;

pub(super) struct ServiceMonitor {
    data: RefCell<ServiceMonitorData>,
}

impl ServiceMonitor {
    pub(super) fn new(configr: &Rc<ServiceConfig>) -> ServiceMonitor {
        ServiceMonitor {
            data: RefCell::new(ServiceMonitorData::new(configr)),
        }
    }

    pub(super) fn start_action(&self) {
        self.data.borrow_mut().start_action()
    }
}

struct ServiceMonitorData {
    // associated objects
    config: Rc<ServiceConfig>,

    // owned objects
    watchdog_original_usec: u64,
    watchdog_override_usec: u64,
    watchdog_override_enable: bool,
}

// the declaration "pub(self)" is for identification only.
#[allow(dead_code)]
impl ServiceMonitorData {
    pub(self) fn new(configr: &Rc<ServiceConfig>) -> ServiceMonitorData {
        ServiceMonitorData {
            config: Rc::clone(configr),
            watchdog_original_usec: u64::MAX,
            watchdog_override_usec: 0,
            watchdog_override_enable: false,
        }
    }

    pub(self) fn start_action(&mut self) {
        if self
            .config
            .config_data()
            .borrow()
            .Service
            .WatchdogUSec
            .is_some()
        {
            if let Some(wd_sec) = self.config.config_data().borrow().Service.WatchdogUSec {
                self.watchdog_original_usec = wd_sec;
            }
        }
        self.watchdog_override_enable = false;
        self.watchdog_override_usec = u64::MAX;
    }
    /// software watchdog, if the watchdog not receive the READY=1 message within the timeout period, the kill the servcie.
    /// start the watchdog, compare the original and override timeout value, if it's invalid value then stop the watchdog.
    /// call recvmsg and read messages from the socket, and judge if it is the expected value, like READY=1.
    /// not implemented all function, depend on the timer and sd-event.
    fn start_watchdog(self) {
        // if watchdog_override_enable is enabled, the override the timeout with the watchdog_override_usec
        let watchdog_usec = if self.watchdog_override_enable {
            self.watchdog_override_usec
        } else {
            self.watchdog_original_usec
        };
        // if timeout is 0 then stop the watchdog
        if watchdog_usec == 0 || watchdog_usec == u64::MAX {
            self.stop_watchdog()
        }
        libwatchdog::register_timer();
        libwatchdog::event_source_set_enabled(true);
    }

    fn stop_watchdog(self) {
        libwatchdog::event_source_set_enabled(false);
    }
}
