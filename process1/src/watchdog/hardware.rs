use std::cmp::min;
use std::fs::{File, OpenOptions};
use std::io::{self, Error, Result};
use std::os::unix::io::{AsRawFd, RawFd};
use std::time::{Duration, Instant};

use nix::errno::Errno;
use nix::{ioctl_read, ioctl_readwrite};

pub trait Watchdog {
    fn config(self, timeout: Option<Duration>) -> io::Result<()>;
    fn close(self) -> io::Result<i32>;
    fn feed(&mut self) -> io::Result<()>;
}

#[derive(Debug)]
pub struct HardwareWatchdog {
    device: String,
    file: Option<File>,
    timeout: Option<Duration>,
    last_feed: Option<Instant>,
}

const WATCHDOG_IOCTL_BASE: u8 = b'W';
const WATCHDOG_SETOPTIONS: u8 = 4;
const WATCHDOG_KEEPALIVE: u8 = 5;
const WATCHDOG_SETTIMEOUT: u8 = 6;
const WATCHDOG_GETTIMEOUT: u8 = 7;
const WDIOS_DISABLECARD: i32 = 0x0001;
const WDIOS_ENABLECARD: i32 = 0x0002;
ioctl_read!(
    watchdog_setoptions,
    WATCHDOG_IOCTL_BASE,
    WATCHDOG_SETOPTIONS,
    i32
);
ioctl_read!(
    watchdog_keepalive,
    WATCHDOG_IOCTL_BASE,
    WATCHDOG_KEEPALIVE,
    i32
);
ioctl_readwrite!(
    watchdog_settimeout,
    WATCHDOG_IOCTL_BASE,
    WATCHDOG_SETTIMEOUT,
    i32
);
ioctl_read!(
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

    fn fd(&mut self) -> io::Result<RawFd> {
        if self.file.is_none() {
            self.file = Some(OpenOptions::new().write(true).open(self.device.clone())?)
        }
        Ok(self.file.as_ref().unwrap().as_raw_fd())
    }

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
            device: "/dev/watchdog0".to_string(),
            file: None,
            timeout: Some(Duration::default()),
            last_feed: None,
        }
    }
}

impl Watchdog for HardwareWatchdog {
    fn config(mut self, timeout: Option<Duration>) -> io::Result<()> {
        if self.file.is_some() && (self.timeout == timeout || timeout.is_none()) {
            return Ok(());
        }
        if let Some(time) = timeout {
            if time.is_zero() {
                self.timeout = timeout;
                let _ = self.close();
                return Ok(());
            }
            let secs = min(time.as_secs() as i32, i32::MAX);
            match self.set_timeout(secs) {
                Ok(_) => {
                    self.timeout = Some(Duration::from_secs(secs as u64));
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
                    self.timeout = None;
                }
            }
        }
        if self.timeout.is_none() {
            match self.obtain_timeout() {
                Ok(secs) => self.timeout = Some(Duration::from_secs(secs as u64)),
                Err(err) => {
                    self.timeout = Some(Duration::default());
                    return Err(err);
                }
            }
        }
        Ok(())
    }

    fn close(mut self) -> io::Result<i32> {
        self.timeout = Some(Duration::default());
        self.set_options(false).map_err(Error::from)
    }

    fn feed(&mut self) -> io::Result<()> {
        if let Some(time) = self.timeout {
            if time.is_zero() {
                return Ok(());
            }
        }
        self.keepalive()?;
        self.last_feed = Some(Instant::now());
        Ok(())
    }
}

fn errno_is_not_supported(errno: Errno) -> bool {
    for e in vec![
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

    #[test]
    fn test_errno_is_not_supported() {
        assert_eq!(errno_is_not_supported(Errno::ENOTTY), true);
    }

    #[test]
    fn test_feed() {
        let mut watchdog = HardwareWatchdog::default();
        let _ = watchdog.feed();
    }

    #[test]
    fn test_close() {
        let mut watchdog = HardwareWatchdog::default();
        let _ = watchdog.close();
    }

    #[test]
    fn test_config() {
        let mut watchdog = HardwareWatchdog::default();
        let _ = watchdog.config(Some(Duration::from_secs(10)));
    }

    #[test]
    fn test_set_device() {
        let mut watchdog = HardwareWatchdog::default();
        let _ = watchdog.set_device("/dev/watchdog0".to_string());
    }
}
