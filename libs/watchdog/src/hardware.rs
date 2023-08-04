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

//! Hardware watchdog
use std::cmp::min;
use std::fs::{File, OpenOptions};
use std::io::{self, Error, ErrorKind, Result};
use std::os::unix::io::{AsRawFd, RawFd};
use std::time::{Duration, Instant};

use nix::errno::Errno;
use nix::{ioctl_read, ioctl_readwrite};
/// First define a trait to abstract several key features
pub trait Watchdog {
    /// config() means to configure the properties of a watchdog, mainly set the timeout and turn on/off
    fn config(&mut self, timeout: Option<Duration>) -> io::Result<()>;
    /// means to close
    fn close(&mut self) -> io::Result<i32>;
    /// feed the dog.
    fn feed(&mut self) -> io::Result<()>;
}

/// Then define a structure to implement this trait. The implementation of the hardware watchdog depends on the corresponding hardware.
#[derive(Debug)]
pub struct HardwareWatchdog {
    /// represents the path of the device, the default is "/dev/watchdog0"
    device: String,
    /// saves the File structure after the device is opened
    file: Option<File>,
    /// timeout time
    timeout: Option<Duration>,
    /// the last time the dog was fed
    last_feed: Option<Instant>,
    /// status
    open: bool,
}

/// Since it involves hardware operations, most functions need to be implemented through ioctl.
/// Here, the two macros ioctl_read and ioctl_readwrite provided by the nix package are used to read and write watchdog, respectively.
/// For the definition of constant values, refer to <linux /watchdog.h> file
const WATCHDOG_IOCTL_BASE: u8 = b'W';
const WATCHDOG_SETOPTIONS: u8 = 4;
const WATCHDOG_KEEPALIVE: u8 = 5;
const WATCHDOG_SETTIMEOUT: u8 = 6;
const WATCHDOG_GETTIMEOUT: u8 = 7;
const WDIOS_DISABLECARD: i32 = 0x0001;
const WDIOS_ENABLECARD: i32 = 0x0002;
const WATCHDOG_PATH: &str = "/dev/watchdog0";

ioctl_read!(
    /// watchdog setoptions
    watchdog_setoptions,
    WATCHDOG_IOCTL_BASE,
    WATCHDOG_SETOPTIONS,
    i32
);

ioctl_read!(
    /// watchdog keepalive
    watchdog_keepalive,
    WATCHDOG_IOCTL_BASE,
    WATCHDOG_KEEPALIVE,
    i32
);

ioctl_readwrite!(
    /// watchdog settimeout
    watchdog_settimeout,
    WATCHDOG_IOCTL_BASE,
    WATCHDOG_SETTIMEOUT,
    i32
);

ioctl_read!(
    /// watchdog gettimeout
    watchdog_gettimeout,
    WATCHDOG_IOCTL_BASE,
    WATCHDOG_GETTIMEOUT,
    i32
);

impl HardwareWatchdog {
    fn set_options(&mut self, enable: bool) -> io::Result<i32> {
        let mut flag = if enable {
            WDIOS_ENABLECARD
        } else {
            WDIOS_DISABLECARD
        };
        unsafe { watchdog_setoptions(self.fd()?, &mut flag as *mut i32).map_err(Error::from) }
    }

    fn set_timeout(&mut self, mut secs: i32) -> Result<i32> {
        unsafe { watchdog_settimeout(self.fd()?, &mut secs as *mut i32).map_err(Error::from) }
    }

    // The map and map_err methods are used to obtained timeout seconds
    fn obtain_timeout(&mut self) -> io::Result<i32> {
        let mut sec = 0;
        unsafe {
            watchdog_gettimeout(self.fd()?, &mut sec as *mut i32)
                .map(|_| sec)
                .map_err(Error::from)
        }
    }

    fn keepalive(&mut self) -> io::Result<i32> {
        let mut c = 0;
        unsafe { watchdog_keepalive(self.fd()?, &mut c as *mut i32).map_err(Error::from) }
    }

    // The file is loaded in a lazy way.
    // If empty, the corresponding device will be automatically opened.
    fn fd(&mut self) -> io::Result<RawFd> {
        if self.file.is_none() {
            self.file = Some(OpenOptions::new().write(true).open(self.device.clone())?)
        }
        Ok(self.file.as_ref().unwrap().as_raw_fd())
    }

    #[allow(dead_code)]
    fn set_device(&mut self, device: String) {
        if self.device != device {
            self.device = device;
            self.file = None
        }
    }
}

impl Default for HardwareWatchdog {
    fn default() -> Self {
        HardwareWatchdog {
            device: WATCHDOG_PATH.to_string(),
            file: None,
            timeout: Some(Duration::default()),
            last_feed: None,
            open: false,
        }
    }
}

impl Drop for HardwareWatchdog {
    fn drop(&mut self) {
        if let Err(e) = self.close() {
            println!("HardwareWatchdog drop close err: {}", e);
        }
    }
}

impl Watchdog for HardwareWatchdog {
    /// config watchdog
    fn config(&mut self, timeout: Option<Duration>) -> io::Result<()> {
        if self.open && self.timeout == timeout {
            return Ok(());
        }

        if let Some(time) = timeout {
            if time.is_zero() {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Not Support Zero Timeout",
                ));
            }
            let secs = min(time.as_secs() as i32, i32::MAX);
            match self.set_timeout(secs) {
                Ok(_) => {
                    self.timeout = Some(Duration::from_secs(secs as u64));
                    // After setting timeout, turn on the watchdog and feed the dog one time.
                    self.set_options(true)?;
                    self.feed()?;
                }
                Err(err) => {
                    if let Some(e) = err.raw_os_error() {
                        if !errno_is_not_supported(Errno::from_i32(e)) {
                            return Err(err);
                        }
                    } else {
                        return Err(err);
                    }
                    // Set timeout to null in case of error
                    self.timeout = None;
                }
            }
        }
        // If timeout is empty, try to read the value from the device,
        if self.timeout.is_none() {
            match self.obtain_timeout() {
                Ok(secs) => self.timeout = Some(Duration::from_secs(secs as u64)),
                Err(err) => {
                    self.timeout = Some(Duration::default());
                    return Err(err);
                }
            }
        }
        self.open = true;
        Ok(())
    }

    fn close(&mut self) -> io::Result<i32> {
        if !self.open {
            return Ok(0);
        }
        self.open = false;
        self.timeout = Some(Duration::default());
        self.set_options(false).map_err(Error::from)
    }

    fn feed(&mut self) -> io::Result<()> {
        if !self.open {
            return Err(Error::new(ErrorKind::Other, "Not Config Or Closed"));
        }

        self.keepalive()?;
        self.last_feed = Some(Instant::now());
        Ok(())
    }
}

/// ERRNO_IS_NOT_SUPPORTED
fn errno_is_not_supported(errno: Errno) -> bool {
    for e in [
        Errno::EOPNOTSUPP,
        Errno::ENOTTY,
        Errno::ENOSYS,
        Errno::EAFNOSUPPORT,
        Errno::EPFNOSUPPORT,
        Errno::EPROTONOSUPPORT,
        Errno::ESOCKTNOSUPPORT,
    ] {
        if e == errno {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn has_watchdog0() -> bool {
        Path::new(WATCHDOG_PATH).exists()
    }

    #[test]
    fn test_errno_is_not_supported() {
        assert!(errno_is_not_supported(Errno::ENOTTY));
    }

    #[test]
    fn test_feed() {
        if !has_watchdog0() {
            return;
        }
        if !nix::unistd::getuid().is_root() {
            println!("Unprivileged users cannot config watchdog, skipping.");
            return;
        }
        let mut watchdog = HardwareWatchdog::default();
        watchdog.config(Some(Duration::from_secs(10))).unwrap();
        watchdog.feed().unwrap();
    }

    #[test]
    fn test_close() {
        if !has_watchdog0() {
            return;
        }
        if !nix::unistd::getuid().is_root() {
            println!("Unprivileged users cannot config watchdog, skipping.");
            return;
        }
        let mut watchdog = HardwareWatchdog::default();
        watchdog.config(Some(Duration::from_secs(10))).unwrap();
        watchdog.close().unwrap();
    }

    #[test]
    fn test_config() {
        if !has_watchdog0() {
            return;
        }
        if !nix::unistd::getuid().is_root() {
            println!("Unprivileged users cannot config watchdog, skipping.");
            return;
        }
        let mut watchdog = HardwareWatchdog::default();
        watchdog.config(Some(Duration::from_secs(10))).unwrap();
        watchdog.config(None).unwrap();
    }

    #[test]
    fn test_set_device() {
        if !has_watchdog0() {
            return;
        }
        let mut watchdog = HardwareWatchdog::default();
        watchdog.set_device("/dev/watchdog0".to_string());
    }
}
