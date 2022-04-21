/// 硬件看门狗
use std::cmp::min;
use std::fs::{File, OpenOptions};
use std::io::{self, Error, Result};
use std::os::unix::io::{AsRawFd, RawFd};
use std::time::{Duration, Instant};

use nix::errno::Errno;
use nix::{ioctl_read, ioctl_readwrite};

/// 首先定义了一个trait，将看门狗的几个关键特性抽象出来
/// config表示配置一个看门狗的属性，主要是超时时间timeout并打开看门狗；close表示关闭一个看门狗；feed表示喂狗。
pub trait Watchdog {
    fn config(self, timeout: Option<Duration>) -> io::Result<()>;
    fn close(self) -> io::Result<i32>;
    fn feed(&mut self) -> io::Result<()>;
}

/// 然后定义了一个结构体来实现这个trait。硬件看门狗的实现要依赖相应的硬件。device表示设备的路径，默认为"/dev/watchdog0"，
/// file保存设备打开后的File结构体，timeout表示超时时间，last_feed表示上一次喂狗的时间
#[derive(Debug)]
pub struct HardwareWatchdog {
    device: String,
    file: Option<File>,
    timeout: Option<Duration>,
    last_feed: Option<Instant>,
}

/// 由于涉及硬件操作，所以大部分函数需要通过ioctl来实现功能，这里用例nix包提供的两个宏ioctl_read, ioctl_readwrite，分别是读取和读写看门狗硬件，常量值定义参考<linux/watchdog.h>文件
const WATCHDOG_IOCTL_BASE: u8 = b'W';
const WATCHDOG_SETOPTIONS: u8 = 4;
const WATCHDOG_KEEPALIVE: u8 = 5;
const WATCHDOG_SETTIMEOUT: u8 = 6;
const WATCHDOG_GETTIMEOUT: u8 = 7;
const WDIOS_DISABLECARD: i32 = 0x0001;
const WDIOS_ENABLECARD: i32 = 0x0002;

// 与C头文件中定义的宏函数等价
// ```c
// #define WDIOC_SETOPTIONS    _IOR(WATCHDOG_IOCTL_BASE, 4, int)
// ```
// 表示一个IOCTL的读操作，函数名称为watchdog_setoptions,传递的参数为WATCHDOG_IOCTL_BASE和4和1一个32位int
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

    /// 采用了函数式编程的map和map_err方法，将获取的超时时间返回
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

    /// 文件采用option表示，使用lazy加载的方式，如果访问时为空的话会自动打开对应的设备
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
    /// 配置一个看门狗
    fn config(mut self, timeout: Option<Duration>) -> io::Result<()> {
        // 如果已经打开了设备且超时时间合法，返回OK
        if self.file.is_some() && (self.timeout == timeout || timeout.is_none()) {
            return Ok(());
        }
        if let Some(time) = timeout {
            // timeout为0表示关闭看门狗
            if time.is_zero() {
                self.timeout = timeout;
                let _ = self.close();
                return Ok(());
            }
            let secs = min(time.as_secs() as i32, i32::MAX);
            match self.set_timeout(secs) {
                Ok(_) => {
                    self.timeout = Some(Duration::from_secs(secs as u64));
                    // 设置完超时时间后，打开看门狗并喂一次狗
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
                    // 出错的情况下将timeout置为空
                    self.timeout = None;
                }
            }
        }
        // 如果timeout为空，尝试从设备读取timeout值，是否设备默认值不支持修改
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

/// 不支持的错误列表，参考ERRNO_IS_NOT_SUPPORTED
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

    #[test]
    fn test_errno_is_not_supported() {
        assert!(errno_is_not_supported(Errno::ENOTTY));
    }

    #[test]
    fn test_feed() {
        let mut watchdog = HardwareWatchdog::default();
        let _ = watchdog.feed();
    }

    #[test]
    fn test_close() {
        let watchdog = HardwareWatchdog::default();
        let _ = watchdog.close();
    }

    #[test]
    fn test_config() {
        let watchdog = HardwareWatchdog::default();
        let _ = watchdog.config(Some(Duration::from_secs(10)));
    }

    #[test]
    fn test_set_device() {
        let mut watchdog = HardwareWatchdog::default();
        let _ = watchdog.set_device("/dev/watchdog0".to_string());
    }
}
