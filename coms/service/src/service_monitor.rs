/// software watchdog, if the watchdog not receive the READY=1 message within the timeout period, the kill the servcie.
/// start the watchdog, compare the original and override timeout value, if it's invalid value then stop the watchdog.
/// call recvmsg and read messages from the socket, and judge if it is the expected value, like READY=1.
///
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(super) struct ServiceMonitor {
    watchdog_original_usec: u64,
    watchdog_override_usec: u64,
    watchdog_override_enable: bool,
}

impl ServiceMonitor {
    pub(super) fn new() -> ServiceMonitor {
        ServiceMonitor {
            watchdog_original_usec: 0,
            watchdog_override_usec: 0,
            watchdog_override_enable: false,
        }
    }

    pub(super) fn override_watchdog_usec(&mut self, watchdog_usec: u64) {
        self.watchdog_override_enable = true;
        self.watchdog_override_usec = watchdog_usec;
    }

    pub(super) fn set_original_watchdog(&mut self, watchdog_usec: u64) {
        self.watchdog_original_usec = watchdog_usec;
    }

    pub(super) fn watchdog_usec(&self) -> u64 {
        // if watchdog_override_enable is enabled, the override the timeout with the watchdog_override_usec
        log::debug!(
            "override enable:{}, original sec: {}, override sec:{}",
            self.watchdog_override_enable,
            self.watchdog_original_usec,
            self.watchdog_override_usec
        );
        if self.watchdog_override_enable {
            self.watchdog_override_usec
        } else {
            self.watchdog_original_usec
        }
    }
}
