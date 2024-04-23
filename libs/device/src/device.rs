// Copyright (c) 2022 Huawei Technologies Co.,Ltd. All rights r&eserved.
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

//! struct Device
//!
use crate::utils::readlink_value;
use crate::{error::*, DeviceAction};
use basic::fs::{chmod, open_temporary, touch_file};
use basic::parse::{device_path_parse_devnum, parse_devnum, parse_ifindex};
use basic::string::fnmatch_or_empty;
use basic::uuid::{randomize, Uuid};
use libc::{
    dev_t, faccessat, gid_t, mode_t, uid_t, F_OK, S_IFBLK, S_IFCHR, S_IFDIR, S_IFLNK, S_IFMT,
    S_IRUSR, S_IWUSR,
};
use nix::dir::Dir;
use nix::errno::{self, Errno};
use nix::fcntl::{open, AtFlags, OFlag};
use nix::sys::stat::{self, fchmod, lstat, major, makedev, minor, stat, Mode};
use nix::unistd::{unlink, Gid, Uid};
use snafu::ResultExt;
use std::cell::{Ref, RefCell};
use std::collections::hash_set::Iter;
use std::collections::{HashMap, HashSet, VecDeque};
use std::ffi::CString;
use std::fs::{self, rename, OpenOptions, ReadDir};
use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use std::os::unix::prelude::FromRawFd;
use std::path::Path;
use std::rc::Rc;
use std::result::Result;

/// default directory to contain runtime temporary files
pub const DEFAULT_BASE_DIR: &str = "/run/devmaster";
/// database directory path
pub const DB_BASE_DIR: &str = "data";
/// tags directory path
pub const TAGS_BASE_DIR: &str = "tags";

/// Device
#[derive(Debug)]
pub struct Device {
    /// inotify handler
    pub watch_handle: RefCell<i32>,
    /// the parent device
    pub parent: RefCell<Option<Rc<Device>>>,
    /// ifindex
    pub ifindex: RefCell<u32>,
    /// device type
    pub devtype: RefCell<String>,
    /// device name, e.g., /dev/sda
    pub devname: RefCell<String>,
    /// device number
    pub devnum: RefCell<u64>,
    /// syspath with /sys/ as prefix, e.g., /sys/devices/pci0000:00/0000:00:10.0/host2/target2:0:1/2:0:1:0/block/sda
    pub syspath: RefCell<String>,
    /// relative path under /sys/, e.g., /devices/pci0000:00/0000:00:10.0/host2/target2:0:1/2:0:1:0/block/sda
    pub devpath: RefCell<String>,
    /// sysnum
    pub sysnum: RefCell<String>,
    /// sysname is the basename of syspath, e.g., sda
    pub sysname: RefCell<String>,
    /// device subsystem
    pub subsystem: RefCell<String>,
    /// only set when subsystem is 'drivers'
    pub driver_subsystem: RefCell<String>,
    /// device driver
    pub driver: RefCell<String>,
    /// device id
    pub device_id: RefCell<String>,
    /// device initialized usec
    pub usec_initialized: RefCell<u64>,
    /// device mode
    pub devmode: RefCell<Option<mode_t>>,
    /// device user id
    pub devuid: RefCell<Option<Uid>>,
    /// device group id
    pub devgid: RefCell<Option<Gid>>,
    // only set when device is passed through netlink
    /// uevent action
    pub action: RefCell<DeviceAction>,
    /// uevent seqnum, only if the device origins from uevent, the seqnum can be greater than zero
    pub seqnum: RefCell<u64>,
    // pub synth_uuid: u64,
    // pub partn: u32,
    /// device properties
    pub properties: RefCell<HashMap<String, String>>,
    /// the subset of properties that should be written to db
    pub properties_db: RefCell<HashMap<String, String>>,
    /// the string of properties
    pub properties_nulstr: RefCell<Vec<u8>>,
    /// the length of properties nulstr
    pub properties_nulstr_len: RefCell<usize>,
    /// cached sysattr values
    pub sysattr_values: RefCell<HashMap<String, String>>,
    /// names of sysattrs
    pub sysattrs: RefCell<HashSet<String>>,
    /// all tags
    pub all_tags: RefCell<HashSet<String>>,
    /// current tags
    pub current_tags: RefCell<HashSet<String>>,
    /// device links
    pub devlinks: RefCell<HashSet<String>>,
    /// device links priority
    pub devlink_priority: RefCell<i32>,
    /// block device sequence number, monothonically incremented by the kernel on create/attach
    pub diskseq: RefCell<u64>,
    /// database version
    pub database_version: RefCell<u32>,

    /// children
    pub children: RefCell<HashMap<String, Rc<Device>>>,
    /// children enumerated
    pub children_enumerated: RefCell<bool>,

    /// properties are outdated
    pub properties_buf_outdated: RefCell<bool>,
    /// devlinks in properties are outdated
    pub property_devlinks_outdated: RefCell<bool>,
    /// tags in properties are outdated
    pub property_tags_outdated: RefCell<bool>,
    /// whether the device is initialized by reading uevent file
    pub uevent_loaded: RefCell<bool>,
    /// whether the subsystem is initialized
    pub subsystem_set: RefCell<bool>,
    /// whether the parent is set
    pub parent_set: RefCell<bool>,
    /// whether the driver is set
    pub driver_set: RefCell<bool>,
    /// whether the database is loaded
    pub db_loaded: RefCell<bool>,
    /// whether the attributes are cached
    pub sysattrs_cached: RefCell<bool>,

    /// whether the device object is initialized
    pub is_initialized: RefCell<bool>,
    /// don not read more information from uevent/db
    pub sealed: RefCell<bool>,
    /// persist device db during switching root from initrd
    pub db_persist: RefCell<bool>,

    /// the base directory path to contain runtime temporary files
    pub base_path: RefCell<String>,
}

impl Default for Device {
    fn default() -> Self {
        Self::new()
    }
}

/// public methods
impl Device {
    /// create Device instance
    pub fn new() -> Device {
        Device {
            watch_handle: RefCell::new(-1),
            ifindex: RefCell::new(0),
            devtype: RefCell::new(String::new()),
            devname: RefCell::new(String::new()),
            devnum: RefCell::new(0),
            syspath: RefCell::new(String::new()),
            devpath: RefCell::new(String::new()),
            sysnum: RefCell::new(String::new()),
            sysname: RefCell::new(String::new()),
            subsystem: RefCell::new(String::new()),
            driver_subsystem: RefCell::new(String::new()),
            driver: RefCell::new(String::new()),
            device_id: RefCell::new(String::new()),
            usec_initialized: RefCell::new(0),
            devmode: RefCell::new(None),
            devuid: RefCell::new(None),
            devgid: RefCell::new(None),
            action: RefCell::new(DeviceAction::default()),
            seqnum: RefCell::new(0),
            properties: RefCell::new(HashMap::new()),
            properties_db: RefCell::new(HashMap::new()),
            properties_nulstr: RefCell::new(vec![]),
            properties_nulstr_len: RefCell::new(0),
            sysattr_values: RefCell::new(HashMap::new()),
            sysattrs: RefCell::new(HashSet::new()),
            all_tags: RefCell::new(HashSet::new()),
            current_tags: RefCell::new(HashSet::new()),
            devlinks: RefCell::new(HashSet::new()),
            properties_buf_outdated: RefCell::new(true),
            uevent_loaded: RefCell::new(false),
            subsystem_set: RefCell::new(false),
            diskseq: RefCell::new(0),
            parent: RefCell::new(None),
            parent_set: RefCell::new(false),
            driver_set: RefCell::new(false),
            property_devlinks_outdated: RefCell::new(true),
            property_tags_outdated: RefCell::new(true),
            is_initialized: RefCell::new(false),
            db_loaded: RefCell::new(false),
            sealed: RefCell::new(false),
            database_version: RefCell::new(0),
            devlink_priority: RefCell::new(0),
            db_persist: RefCell::new(false),
            children: RefCell::new(HashMap::new()),
            children_enumerated: RefCell::new(false),
            sysattrs_cached: RefCell::new(false),
            base_path: RefCell::new(DEFAULT_BASE_DIR.to_string()),
        }
    }

    /// change db prefix
    pub fn set_base_path(&self, prefix: &str) {
        self.base_path.replace(prefix.to_string());
    }

    /// create Device from buffer
    pub fn from_nulstr(nulstr: &[u8]) -> Result<Device, Error> {
        let device = Device::new();
        let s = String::from_utf8(nulstr.to_vec()).unwrap();
        let mut length = 0;
        let mut major = String::new();
        let mut minor = String::new();
        for line in s.split('\0') {
            let tokens = line.split('=').collect::<Vec<&str>>();
            if tokens.len() < 2 {
                break;
            }
            length = length + line.len() + 1;
            let (key, value) = (tokens[0], tokens[1]);
            match key {
                "MINOR" => minor = value.to_string(),
                "MAJOR" => major = value.to_string(),
                _ => device.amend_key_value(key, value)?,
            }
        }

        if !major.is_empty() {
            device.set_devnum(&major, &minor)?;
        }

        device.update_properties_bufs()?;

        device.verify()
    }

    /// Verify the legality of a device object from nulstr.
    fn verify(self) -> Result<Device, Error> {
        if self.devpath.borrow().is_empty()
            || self.subsystem.borrow().is_empty()
            || *self.action.borrow() == DeviceAction::Invalid
            || *self.seqnum.borrow() == 0
        {
            return Err(Error::Nix {
                msg: "Received invalid device object from uevent".to_string(),
                source: Errno::EINVAL,
            });
        }

        if &*self.subsystem.borrow() == "drivers" {
            self.set_drivers_subsystem()?;
        }

        self.sealed.replace(true);

        Ok(self)
    }

    /// create a Device instance from devname
    /// devname is the device path under /dev
    /// e.g. /dev/block/8:0
    /// e.g. /dev/char/7:0
    /// e.g. /dev/sda
    pub fn from_devname(devname: &str) -> Result<Device, Error> {
        if !devname.starts_with("/dev") {
            return Err(Error::Nix {
                msg: format!(
                    "from_devname failed: devname '{}' doesn't start with /dev",
                    devname
                ),
                source: Errno::EINVAL,
            });
        }

        let device = if let Ok((mode, devnum)) = device_path_parse_devnum(devname) {
            Device::from_mode_and_devnum(mode, devnum)?
        } else {
            match stat(Path::new(&devname)) {
                Ok(st) => Device::from_mode_and_devnum(st.st_mode, st.st_rdev)?,
                Err(e) => {
                    return Err(Error::Nix {
                        msg: format!("from_devname failed: cannot stat '{}'", devname),
                        source: {
                            if [Errno::ENODEV, Errno::ENXIO, Errno::ENOENT].contains(&e) {
                                // device is absent
                                Errno::ENODEV
                            } else {
                                e
                            }
                        },
                    });
                }
            }
        };

        Ok(device)
    }

    /// create a Device instance from syspath
    pub fn from_syspath(syspath: &str, strict: bool) -> Result<Device, Error> {
        if strict && !syspath.starts_with("/sys/") {
            return Err(Error::Nix {
                msg: format!(
                    "from_syspath failed: syspath '{}' doesn't start with /sys",
                    syspath
                ),
                source: nix::errno::Errno::EINVAL,
            });
        }

        let device = Device::default();
        device.set_syspath(syspath, true)?;

        Ok(device)
    }

    /// Create a Device instance from path.
    ///
    /// The path falls into two kinds: devname (/dev/...) and syspath (/sys/devices/...)
    pub fn from_path(path: &str) -> Result<Device, Error> {
        if path.starts_with("/dev") {
            return Device::from_devname(path);
        }

        Device::from_syspath(path, false)
    }

    /// create a Device instance from devnum
    pub fn from_devnum(device_type: char, devnum: dev_t) -> Result<Device, Error> {
        if device_type != 'b' && device_type != 'c' {
            return Err(Error::Nix {
                msg: format!("from_devnum failed: invalid device type '{}'", device_type),
                source: Errno::EINVAL,
            });
        }

        Self::from_mode_and_devnum(
            {
                if device_type == 'b' {
                    S_IFBLK
                } else {
                    S_IFCHR
                }
            },
            devnum,
        )
    }

    /// create a Device instance from ifindex
    pub fn from_ifindex(ifindex: u32) -> Result<Device, Error> {
        let mut buf = [0; 16];
        let buf_ptr = buf.as_mut_ptr() as *mut libc::c_char;
        unsafe {
            if libc::if_indextoname(ifindex, buf_ptr).is_null() {
                return Err(Error::Nix {
                    msg: format!("from_ifindex failed: if_indextoname '{}' failed", ifindex),
                    source: Errno::ENXIO,
                });
            }
        };

        let buf_trans: &[u8] = unsafe { std::slice::from_raw_parts(buf.as_ptr() as *const _, 16) };

        let ifname = String::from_utf8(buf_trans.to_vec()).context(FromUtf8 {
            msg: format!("invalid utf-8 string {:?}", buf_trans),
        })?;

        let syspath = format!("/sys/class/net/{}", ifname.trim_matches(char::from(0)));
        let dev = Self::from_syspath(&syspath, true)?;

        let i = dev
            .get_ifindex()
            .map_err(|e| e.replace_errno(Errno::ENOENT, Errno::ENXIO))?;

        if i != ifindex {
            return Err(Error::Nix {
                msg: "from_ifindex failed: ifindex inconsistent".to_string(),
                source: Errno::ENXIO,
            });
        }

        Ok(dev)
    }

    /// Create a Device instance from subsystem and sysname.
    ///
    /// If subsystem is 'drivers', sysname should be like 'xxx:yyy'
    pub fn from_subsystem_sysname(subsystem: &str, sysname: &str) -> Result<Device, Error> {
        let sysname = sysname.replace('/', "!");
        if subsystem == "subsystem" {
            match Device::from_syspath(&format!("/sys/bus/{}", sysname), true) {
                Ok(d) => return Ok(d),
                Err(e) => {
                    if e.get_errno() != Errno::ENODEV {
                        return Err(Error::Nix {
                            msg: format!("from_subsystem_sysname failed: {}", e),
                            source: e.get_errno(),
                        });
                    }
                }
            }

            match Device::from_syspath(&format!("/sys/class/{}", sysname), true) {
                Ok(d) => return Ok(d),
                Err(e) => {
                    if e.get_errno() != Errno::ENODEV {
                        return Err(Error::Nix {
                            msg: format!("from_subsystem_sysname failed: {}", e),
                            source: e.get_errno(),
                        });
                    }
                }
            }
        } else if subsystem == "module" {
            match Device::from_syspath(&format!("/sys/module/{}", sysname), true) {
                Ok(d) => return Ok(d),
                Err(e) => {
                    if e.get_errno() != Errno::ENODEV {
                        return Err(Error::Nix {
                            msg: format!("from_subsystem_sysname failed: {}", e),
                            source: e.get_errno(),
                        });
                    }
                }
            }
        } else if subsystem == "drivers" {
            if let Some(idx) = sysname.find(':') {
                if idx < sysname.len() - 1 {
                    let subsys = sysname[0..idx].to_string();
                    let sep = sysname[idx + 1..].to_string();
                    let syspath = if sep == "drivers" {
                        format!("/sys/bus/{}/drivers", subsys)
                    } else {
                        format!("/sys/bus/{}/drivers/{}", subsys, sep)
                    };
                    match Device::from_syspath(&syspath, true) {
                        Ok(d) => return Ok(d),
                        Err(e) => {
                            if e.get_errno() != Errno::ENODEV {
                                return Err(Error::Nix {
                                    msg: format!("from_subsystem_sysname failed: {}", e),
                                    source: e.get_errno(),
                                });
                            }
                        }
                    }
                }
            }
        }

        let syspath = format!("/sys/bus/{}/devices/{}", subsystem, sysname);
        match Device::from_syspath(&syspath, true) {
            Ok(d) => return Ok(d),
            Err(e) => {
                if e.get_errno() != Errno::ENODEV {
                    return Err(Error::Nix {
                        msg: format!("from_subsystem_sysname failed: {}", e),
                        source: e.get_errno(),
                    });
                }
            }
        }

        let syspath = format!("/sys/class/{}/{}", subsystem, sysname);
        match Device::from_syspath(&syspath, true) {
            Ok(d) => return Ok(d),
            Err(e) => {
                if e.get_errno() != Errno::ENODEV {
                    return Err(Error::Nix {
                        msg: format!("from_subsystem_sysname failed: {}", e),
                        source: e.get_errno(),
                    });
                }
            }
        }

        let syspath = format!("/sys/firmware/{}/{}", subsystem, sysname);
        match Device::from_syspath(&syspath, true) {
            Ok(d) => return Ok(d),
            Err(e) => {
                if e.get_errno() != Errno::ENODEV {
                    return Err(Error::Nix {
                        msg: format!("from_subsystem_sysname failed: {}", e),
                        source: e.get_errno(),
                    });
                }
            }
        }

        Err(Error::Nix {
            msg: format!(
                "from_subsystem_sysname failed: subsystem {} or sysname {} is invalid",
                subsystem, sysname
            ),
            source: Errno::ENODEV,
        })
    }

    /// Set sysattr value.
    ///
    /// If the sysattr is not 'uevent', the value will be cached.
    pub fn set_sysattr_value(&self, sysattr: &str, value: Option<&str>) -> Result<(), Error> {
        if value.is_none() {
            self.remove_cached_sysattr_value(sysattr);
            return Ok(());
        }

        let sysattr_path = format!("{}/{}", self.syspath.borrow(), sysattr);

        let mut file = match OpenOptions::new().write(true).open(&sysattr_path) {
            Ok(f) => f,
            Err(e) => {
                return Err(Error::Nix {
                    msg: format!(
                        "set_sysattr_value failed: can't open sysattr '{}'",
                        sysattr_path
                    ),
                    source: Errno::from_i32(e.raw_os_error().unwrap_or_default()),
                })
            }
        };

        if let Err(e) = file.write(value.unwrap().as_bytes()) {
            self.remove_cached_sysattr_value(sysattr);
            return Err(Error::Nix {
                msg: format!(
                    "set_sysattr_value failed: can't write sysattr '{}'",
                    sysattr_path
                ),
                source: Errno::from_i32(e.raw_os_error().unwrap_or_default()),
            });
        };

        if sysattr == "uevent" {
            return Ok(());
        }

        self.cache_sysattr_value(sysattr, value.unwrap())?;

        Ok(())
    }

    /// create a Device instance from device id
    pub fn from_device_id(id: &str) -> Result<Device, Error> {
        if id.len() < 2 {
            return Err(Error::Nix {
                msg: format!("from_device_id failed: invalid id '{}'", id),
                source: Errno::EINVAL,
            });
        }

        match id.chars().next() {
            Some('b') | Some('c') => {
                let devnum = parse_devnum(&id[1..]).context(Basic {
                    msg: format!("from_device_id failed: parse_devnum '{}' failed", id),
                })?;

                Device::from_devnum(id.chars().next().unwrap(), devnum)
            }
            Some('n') => {
                let ifindex = parse_ifindex(&id[1..]).context(Basic {
                    msg: format!("from_device_id failed: parse_ifindex '{}' failed", id),
                })?;

                Device::from_ifindex(ifindex)
            }
            Some('+') => {
                let sep = match id.find(':') {
                    Some(idx) => {
                        if idx == id.len() - 1 {
                            return Err(Error::Nix {
                                msg: format!("from_device_id failed: invalid device id '{}'", id),
                                source: Errno::EINVAL,
                            });
                        }

                        idx
                    }
                    None => {
                        return Err(Error::Nix {
                            msg: format!("from_device_id failed: invalid device id '{}'", id),
                            source: Errno::EINVAL,
                        });
                    }
                };

                let subsystem = id[1..sep].to_string();
                let sysname = id[sep + 1..].to_string();
                Device::from_subsystem_sysname(&subsystem, &sysname)
            }
            _ => Err(Error::Nix {
                msg: format!("from_device_id failed: invalid id '{}'", id),
                source: Errno::EINVAL,
            }),
        }
    }

    /// trigger a fake device action, then kernel will report an uevent
    pub fn trigger(&self, action: DeviceAction) -> Result<(), Error> {
        self.set_sysattr_value("uevent", Some(&format!("{}", action)))
    }

    /// get the syspath of the device
    pub fn get_syspath(&self) -> Result<String, Error> {
        if self.syspath.borrow().is_empty() {
            return Err(Error::Nix {
                msg: "get_syspath failed: no syspath".to_string(),
                source: Errno::ENOENT,
            });
        }

        Ok(self.syspath.borrow().clone())
    }

    /// get the devpath of the device
    pub fn get_devpath(&self) -> Result<String, Error> {
        if self.devpath.borrow().is_empty() {
            return Err(Error::Nix {
                msg: "get_devpath failed: no devpath".to_string(),
                source: Errno::ENOENT,
            });
        }

        Ok(self.devpath.borrow().clone())
    }

    /// get the sysname of the device
    pub fn get_sysname(&self) -> Result<String, Error> {
        if self.sysname.borrow().is_empty() {
            self.set_sysname_and_sysnum()?;
        }

        Ok(self.sysname.borrow().clone())
    }

    /// get the parent of the device
    pub fn get_parent(&self) -> Result<Rc<Device>, Error> {
        if !*self.parent_set.borrow() {
            match Device::new_from_child(self) {
                Ok(parent) => {
                    let _ = self.parent.replace(Some(Rc::new(parent)));
                }
                Err(e) => {
                    // it is okay if no parent device is found,
                    if e.get_errno() != Errno::ENODEV {
                        return Err(Error::Nix {
                            msg: format!("get_parent failed: {}", e),
                            source: e.get_errno(),
                        });
                    }
                }
            };
            self.parent_set.replace(true);
        }

        if self.parent.borrow().is_none() {
            return Err(Error::Nix {
                msg: format!(
                    "get_parent failed: device '{}' has no parent",
                    self.devpath.borrow()
                ),
                source: Errno::ENOENT,
            });
        }

        return Ok(self.parent.borrow().clone().unwrap());
    }

    /// get the parent of the device
    pub fn get_parent_with_subsystem_devtype(
        &self,
        subsystem: &str,
        devtype: Option<&str>,
    ) -> Result<Rc<Device>, Error> {
        let mut parent = match self.get_parent() {
            Ok(parent) => parent,
            Err(e) => return Err(e),
        };

        loop {
            let parent_subsystem = parent.get_subsystem();

            if parent_subsystem.is_ok() && parent_subsystem.unwrap() == subsystem {
                if devtype.is_none() {
                    break;
                }

                let parent_devtype = parent.get_devtype();
                if parent_devtype.is_ok() && parent_devtype.unwrap() == devtype.unwrap() {
                    break;
                }
            }

            let tmp = parent.get_parent()?;
            parent = tmp;
        }

        Ok(parent)
    }

    /// get subsystem
    pub fn get_subsystem(&self) -> Result<String, Error> {
        if !*self.subsystem_set.borrow() {
            let subsystem_path = format!("{}/subsystem", self.syspath.borrow());
            let subsystem_path = Path::new(subsystem_path.as_str());

            // get the base name of absolute subsystem path
            // e.g. /sys/devices/pci0000:00/0000:00:10.0/host2/target2:0:1/2:0:1:0/block/sda/subsystem -> ../../../../../../../../class/block
            // get `block`
            let filename = if Path::exists(Path::new(subsystem_path)) {
                readlink_value(subsystem_path)?
            } else {
                "".to_string()
            };

            if !filename.is_empty() {
                self.set_subsystem(&filename);
            } else if self.devpath.borrow().starts_with("/module/") {
                self.set_subsystem("module");
            } else if self.devpath.borrow().contains("/drivers/")
                || self.devpath.borrow().contains("/drivers")
            {
                self.set_drivers_subsystem()?;
            } else if self.devpath.borrow().starts_with("/class/")
                || self.devpath.borrow().starts_with("/bus/")
            {
                self.set_subsystem("subsystem");
            } else {
                self.subsystem_set.replace(true);
            }
        };

        if !self.subsystem.borrow().is_empty() {
            Ok(self.subsystem.borrow().clone())
        } else {
            Err(Error::Nix {
                msg: "get_subsystem failed: no available subsystem".to_string(),
                source: Errno::ENOENT,
            })
        }
    }

    /// get the ifindex of device
    pub fn get_ifindex(&self) -> Result<u32, Error> {
        self.read_uevent_file()?;

        if *self.ifindex.borrow() == 0 {
            return Err(Error::Nix {
                msg: "get_ifindex failed: no ifindex".to_string(),
                source: Errno::ENOENT,
            });
        }

        Ok(*self.ifindex.borrow())
    }

    /// get the device type
    pub fn get_devtype(&self) -> Result<String, Error> {
        match self.read_uevent_file() {
            Ok(_) => {}
            Err(e) => {
                return Err(e);
            }
        }

        if self.devtype.borrow().is_empty() {
            return Err(Error::Nix {
                msg: "get_devtype failed: no available devname".to_string(),
                source: Errno::ENOENT,
            });
        }

        Ok(self.devtype.borrow().clone())
    }

    /// get devnum
    pub fn get_devnum(&self) -> Result<u64, Error> {
        self.read_uevent_file()?;

        if major(*self.devnum.borrow()) == 0 {
            return Err(Error::Nix {
                msg: "get_devnum failed: no devnum".to_string(),
                source: Errno::ENOENT,
            });
        }

        Ok(*self.devnum.borrow())
    }

    /// get driver
    pub fn get_driver(&self) -> Result<String, Error> {
        if !*self.driver_set.borrow() {
            let syspath = self.get_syspath()?;
            let driver_path_str = syspath + "/driver";
            let driver_path = Path::new(&driver_path_str);
            let driver = match readlink_value(driver_path) {
                Ok(filename) => filename,
                Err(e) => {
                    if e.get_errno() != Errno::ENOENT {
                        return Err(Error::Nix {
                            msg: format!("get_driver failed: {}", e),
                            source: e.get_errno(),
                        });
                    }

                    String::new()
                }
            };

            // if the device has no driver, clear it from internal property
            self.set_driver(&driver);
        }

        if self.driver.borrow().is_empty() {
            return Err(Error::Nix {
                msg: "get_driver failed: no driver".to_string(),
                source: Errno::ENOENT,
            });
        }

        Ok(self.driver.borrow().clone())
    }

    /// get device name
    pub fn get_devname(&self) -> Result<String, Error> {
        self.read_uevent_file()?;

        if self.devname.borrow().is_empty() {
            return Err(Error::Nix {
                msg: "get_devname failed: no available devname".to_string(),
                source: Errno::ENOENT,
            });
        }

        Ok(self.devname.borrow().clone())
    }

    /// get device sysnum
    pub fn get_sysnum(&self) -> Result<String, Error> {
        if self.sysname.borrow().is_empty() {
            self.set_sysname_and_sysnum()?;
        }

        if self.sysnum.borrow().is_empty() {
            return Err(Error::Nix {
                msg: "get_sysnum failed: no sysnum".to_string(),
                source: Errno::ENOENT,
            });
        }

        Ok(self.sysnum.borrow().clone())
    }

    /// get device action
    pub fn get_action(&self) -> Result<DeviceAction, Error> {
        if *self.action.borrow() == DeviceAction::Invalid {
            return Err(Error::Nix {
                msg: format!(
                    "get_action failed: '{}' does not have uevent action",
                    self.devpath.borrow()
                ),
                source: Errno::ENOENT,
            });
        }

        Ok(*self.action.borrow())
    }

    /// get device seqnum, if seqnum is greater than zero, return Ok, otherwise return Err
    pub fn get_seqnum(&self) -> Result<u64, Error> {
        if *self.seqnum.borrow() == 0 {
            return Err(Error::Nix {
                msg: "get_seqnum failed: no seqnum".to_string(),
                source: Errno::ENOENT,
            });
        }

        Ok(*self.seqnum.borrow())
    }

    /// get device diskseq
    pub fn get_diskseq(&self) -> Result<u64, Error> {
        self.read_uevent_file()?;

        if *self.diskseq.borrow() == 0 {
            return Err(Error::Nix {
                msg: format!(
                    "get_diskseq failed: '{}' does not have diskseq",
                    self.devpath.borrow()
                ),
                source: Errno::ENOENT,
            });
        }

        Ok(*self.diskseq.borrow())
    }

    /// get is initialized
    pub fn get_is_initialized(&self) -> Result<bool, Error> {
        match self.read_db() {
            Ok(_) => {}
            Err(e) => {
                if e.get_errno() == Errno::ENOENT {
                    return Ok(false);
                }

                return Err(Error::Nix {
                    msg: format!("get_is_initialized failed: {}", e),
                    source: e.get_errno(),
                });
            }
        }

        Ok(*self.is_initialized.borrow())
    }

    /// get initialized usec
    pub fn get_usec_initialized(&self) -> Result<u64, Error> {
        if !self.get_is_initialized()? {
            return Err(Error::Nix {
                msg: "get_usec_initialized failed: device is not initialized".to_string(),
                source: nix::Error::EBUSY,
            });
        }

        if *self.usec_initialized.borrow() == 0 {
            return Err(Error::Nix {
                msg: "device usec is not set".to_string(),
                source: nix::Error::ENODATA,
            });
        }

        Ok(*self.usec_initialized.borrow())
    }

    /// get usec since initialization
    pub fn get_usec_since_initialized(&self) -> Result<u64, Error> {
        todo!("require get_usec_initialized");
    }

    /// check whether the device has the tag
    pub fn has_tag(&self, tag: &str) -> Result<bool, Error> {
        let _ = self.read_db();

        Ok(self.all_tags.borrow().contains(tag))
    }

    /// check whether the device has the current tag
    pub fn has_current_tag(&self, tag: &str) -> Result<bool, Error> {
        let _ = self.read_db();

        Ok(self.current_tags.borrow().contains(tag))
    }

    /// get the value of specific device property
    pub fn get_property_value(&self, key: &str) -> Result<String, Error> {
        self.properties_prepare()?;

        match self.properties.borrow().get(key) {
            Some(v) => Ok(v.clone()),
            None => Err(Error::Nix {
                msg: format!("get_property_value failed: no key '{}'", key),
                source: nix::errno::Errno::ENOENT,
            }),
        }
    }

    /// get the trigger uuid of the device
    pub fn get_trigger_uuid(&self) -> Result<Option<Uuid>, Error> {
        /* Retrieves the UUID attached to a uevent when triggering it from userspace via
         * trigger_with_uuid() or an equivalent interface. Returns ENOENT if the record is not
         * caused by a synthetic event and ENODATA if it was but no UUID was specified */
        let s = self.get_property_value("SYNTH_UUID")?;

        /* SYNTH_UUID=0 is set whenever a device is triggered by userspace without specifying a UUID */
        if s == "0" {
            return Err(Error::Nix {
                msg: format!(""),
                source: nix::errno::Errno::ENODATA,
            });
        }

        Ok(Uuid::from_string(&s))
    }

    /// get the value of specific device sysattr
    /// firstly check whether the sysattr is cached, otherwise lookup it from the sysfs and cache it
    pub fn get_sysattr_value(&self, sysattr: &str) -> Result<String, Error> {
        // check whether the sysattr is already cached
        match self.get_cached_sysattr_value(sysattr) {
            Ok(v) => {
                return Ok(v);
            }
            Err(e) => {
                if e.get_errno() != Errno::ESTALE {
                    return Err(Error::Nix {
                        msg: format!("get_sysattr_value failed: {}", e),
                        source: e.get_errno(),
                    });
                }
            }
        }

        let syspath = self.get_syspath()?;
        let sysattr_path = format!("{}/{}", syspath, sysattr);
        let value = match lstat(sysattr_path.as_str()) {
            Ok(stat) => {
                if stat.st_mode & S_IFMT == S_IFLNK {
                    if ["driver", "subsystem", "module"].contains(&sysattr) {
                        readlink_value(sysattr_path)?
                    } else {
                        return Err(Error::Nix {
                            msg: format!("get_sysattr_value failed: invalid sysattr '{}'", sysattr),
                            source: Errno::EINVAL,
                        });
                    }
                } else if stat.st_mode & S_IFMT == S_IFDIR {
                    return Err(Error::Nix {
                        msg: format!(
                            "get_sysattr_value failed: sysattr '{}' is a directory",
                            sysattr
                        ),
                        source: Errno::EISDIR,
                    });
                } else if stat.st_mode & S_IRUSR == 0 {
                    return Err(Error::Nix {
                        msg: format!(
                            "get_sysattr_value failed: no permission to read sysattr '{}'",
                            sysattr
                        ),
                        source: Errno::EPERM,
                    });
                } else {
                    // read full virtual file
                    let mut file = std::fs::OpenOptions::new()
                        .read(true)
                        .open(&sysattr_path)
                        .context(Io {
                            msg: format!(
                                "get_sysattr_value failed: can't open sysattr '{}'",
                                sysattr
                            ),
                        })?;
                    let mut value = String::new();
                    file.read_to_string(&mut value).context(Io {
                        msg: format!("get_sysattr_value failed: can't read sysattr '{}'", sysattr),
                    })?;
                    value.trim_end().to_string()
                }
            }

            Err(e) => {
                self.remove_cached_sysattr_value(sysattr);
                return Err(Error::Nix {
                    msg: format!("get_sysattr_value failed: can't lstat '{}'", sysattr_path),
                    source: e,
                });
            }
        };

        self.cache_sysattr_value(sysattr, &value)?;

        Ok(value)
    }

    /// trigger with uuid
    pub fn trigger_with_uuid(
        &self,
        action: DeviceAction,
        need_uuid: bool,
    ) -> Result<Option<Uuid>, Error> {
        if !need_uuid {
            self.trigger(action)?;
            return Ok(None);
        }

        let s = format!("{}", action);

        let id = randomize().context(Nix {
            msg: "Failed to randomize".to_string(),
        })?;

        let j = s + " " + &id.to_string();

        self.set_sysattr_value("uevent", Some(&j))?;

        Ok(Some(id))
    }

    /// open device
    pub fn open(&self, oflags: OFlag) -> Result<File, Error> {
        let devname = self
            .get_devname()
            .map_err(|e| e.replace_errno(Errno::ENOENT, Errno::ENOEXEC))?;
        let devnum = self
            .get_devnum()
            .map_err(|e| e.replace_errno(Errno::ENOENT, Errno::ENOEXEC))?;

        let subsystem = match self.get_subsystem() {
            Ok(s) => s,
            Err(e) => {
                if e.get_errno() != Errno::ENOENT {
                    return Err(Error::Nix {
                        msg: format!("open failed: {}", e),
                        source: e.get_errno(),
                    });
                }

                "".to_string()
            }
        };

        let file = match open(
            devname.as_str(),
            if oflags.intersects(OFlag::O_PATH) {
                oflags
            } else {
                OFlag::O_CLOEXEC | OFlag::O_NOFOLLOW | OFlag::O_PATH
            },
            stat::Mode::empty(),
        ) {
            Ok(fd) => unsafe { std::fs::File::from_raw_fd(fd) },
            Err(e) => {
                return Err(Error::Nix {
                    msg: format!("open failed: can't open '{}'", devname),
                    source: e,
                })
            }
        };

        let stat = nix::sys::stat::fstat(file.as_raw_fd()).context(Nix {
            msg: format!(
                "open failed: can't fstat fd {} for '{}'",
                file.as_raw_fd(),
                devname
            ),
        })?;

        if stat.st_rdev != devnum {
            return Err(Error::Nix {
                msg: format!(
                    "open failed: device number is inconsistent, 'st_rdev {}', 'devnum {}'",
                    stat.st_rdev, devnum
                ),
                source: Errno::ENXIO,
            });
        }

        if subsystem == "block" {
            if stat.st_mode & S_IFMT != S_IFBLK {
                // the device is not block
                return Err(Error::Nix {
                    msg: format!(
                        "open failed: subsystem is inconsistent, 'st_mode {}', 'subsystem {}'",
                        stat.st_mode, subsystem
                    ),
                    source: Errno::ENXIO,
                });
            }
        } else if stat.st_mode & S_IFMT != S_IFCHR {
            // the device is not char
            return Err(Error::Nix {
                msg: format!(
                    "open failed: subsystem is inconsistent, 'st_mode {}', 'subsystem {}'",
                    stat.st_mode, subsystem
                ),
                source: Errno::ENXIO,
            });
        }

        // if open flags has O_PATH, then we cannot check diskseq
        if oflags.intersects(OFlag::O_PATH) {
            return Ok(file);
        }

        let mut diskseq: u64 = 0;

        if self.get_is_initialized()? {
            match self.get_property_value("ID_IGNORE_DISKSEQ") {
                Ok(value) => {
                    if !value.parse::<bool>().context(ParseBool {
                        msg: format!("invalid value '{}'", value),
                    })? {
                        match self.get_diskseq() {
                            Ok(n) => diskseq = n,
                            Err(e) => {
                                if e.get_errno() != Errno::ENOENT {
                                    return Err(Error::Nix {
                                        msg: format!("open failed: {}", e),
                                        source: e.get_errno(),
                                    });
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    if e.get_errno() != Errno::ENOENT {
                        return Err(Error::Nix {
                            msg: format!("open failed: {}", e),
                            source: e.get_errno(),
                        });
                    }
                }
            }
        }

        let file2 = basic::fd::fd_reopen(file.as_raw_fd(), oflags).context(Basic {
            msg: format!("failed to open {}", file.as_raw_fd()),
        })?;

        if diskseq == 0 {
            return Ok(file2);
        }

        let q = basic::fd::fd_get_diskseq(file2.as_raw_fd()).context(Basic {
            msg: format!("failed to get diskseq on fd {}", file2.as_raw_fd()),
        })?;

        if q != diskseq {
            return Err(Error::Nix {
                msg: format!(
                    "open failed: diskseq is inconsistent, ioctl get {}, but diskseq is {}",
                    q, diskseq
                ),
                source: Errno::ENXIO,
            });
        }

        Ok(file2)
    }

    /// add property into device
    pub fn add_property(&self, key: &str, value: &str) -> Result<(), Error> {
        self.add_property_aux(key, value, false)?;

        if !key.starts_with('.') {
            self.add_property_aux(key, value, true)?;
        }

        Ok(())
    }

    /// shadow clone a device object and import properties from db
    pub fn clone_with_db(&self) -> Result<Device, Error> {
        let device = self.shallow_clone()?;
        device.read_db_internal(true)?;
        device.sealed.replace(true);
        Ok(device)
    }

    /// add tag to the device object
    pub fn add_tag(&self, tag: &str, both: bool) {
        if tag.trim().is_empty() {
            return;
        }

        self.all_tags.borrow_mut().insert(tag.trim().to_string());

        if both {
            self.current_tags
                .borrow_mut()
                .insert(tag.trim().to_string());
        }
        self.property_tags_outdated.replace(true);
    }

    /// add a set of tags, separated by ':'
    pub fn add_tags(&self, tags: &str, both: bool) {
        for tag in tags.split(':') {
            self.add_tag(tag, both);
        }
    }

    /// remove specific tag
    pub fn remove_tag(&self, tag: &str) {
        self.current_tags.borrow_mut().remove(tag);
        self.property_tags_outdated.replace(true);
    }

    /// cleanup all tags
    pub fn cleanup_tags(&self) {
        self.all_tags.borrow_mut().clear();
        self.current_tags.borrow_mut().clear();

        self.property_tags_outdated.replace(true);
    }

    /// set device db as persist
    pub fn set_db_persist(&self) {
        self.db_persist.replace(true);
    }

    /// set the priority of device symlink
    pub fn set_devlink_priority(&self, priority: i32) {
        self.devlink_priority.replace(priority);
    }

    /// get the priority of device symlink
    pub fn get_devlink_priority(&self) -> Result<i32, Error> {
        self.read_db()?;

        Ok(*self.devlink_priority.borrow())
    }

    /// Get the device id.
    ///
    /// Device id is used to identify database file in /run/devmaster/data/.
    ///
    /// The format is like:
    ///
    /// - character device:       c<major>:<minor>
    /// - block device:           b<major>:<minor>
    /// - network interface:      n<ifindex>
    /// - drivers:                +drivers:<driver subsystem>:<sysname>
    /// - other subsystems:       +<subsystem>:<sysname>
    pub fn get_device_id(&self) -> Result<String, Error> {
        if self.device_id.borrow().is_empty() {
            let subsystem = self.get_subsystem()?;

            let id: String;
            if let Ok(devnum) = self.get_devnum() {
                id = format!(
                    "{}{}:{}",
                    if subsystem == "block" { 'b' } else { 'c' },
                    major(devnum),
                    minor(devnum)
                );
            } else if let Ok(ifindex) = self.get_ifindex() {
                id = format!("n{}", ifindex);
            } else {
                let sysname = self.get_sysname()?;

                if subsystem == "drivers" {
                    id = format!("+drivers:{}:{}", self.driver_subsystem.borrow(), sysname);
                } else {
                    id = format!("+{}:{}", subsystem, sysname);
                }
            }
            self.device_id.replace(id);
        }

        Ok(self.device_id.borrow().clone())
    }

    /// cleanup devlinks
    pub fn cleanup_devlinks(&self) {
        self.devlinks.borrow_mut().clear();
        self.property_devlinks_outdated.replace(true);
    }

    /// add a set of devlinks to the device object
    pub fn add_devlinks(&self, devlinks: &str) -> Result<(), Error> {
        for link in devlinks.split_ascii_whitespace() {
            self.add_devlink(link)?;
        }

        Ok(())
    }

    /// add devlink records to the device object
    pub fn add_devlink(&self, devlink: &str) -> Result<(), Error> {
        if let Some(stripped) = devlink.strip_prefix("/dev") {
            if stripped.is_empty() {
                return Err(Error::Nix {
                    msg: "add_devlink failed: invalid devlink".to_string(),
                    source: nix::Error::EINVAL,
                });
            }
            self.devlinks.borrow_mut().insert(devlink.to_string());
        } else {
            if devlink.starts_with('/') {
                return Err(Error::Nix {
                    msg: "add_devlink failed: invalid devlink".to_string(),
                    source: nix::Error::EINVAL,
                });
            }
            self.devlinks
                .borrow_mut()
                .insert(format!("/dev/{}", devlink));
        }

        self.property_devlinks_outdated.replace(true);

        Ok(())
    }

    /// get uid of devnode
    pub fn get_devnode_uid(&self) -> Result<Uid, Error> {
        self.read_db()?;

        self.devuid.borrow().ok_or(Error::Nix {
            msg: "get_devnode_uid failed: devuid is not set".to_string(),
            source: errno::Errno::ENOENT,
        })
    }

    /// get gid of devnode
    pub fn get_devnode_gid(&self) -> Result<Gid, Error> {
        self.read_db()?;

        self.devgid.borrow().ok_or(Error::Nix {
            msg: "get_devnode_gid failed: devgid is not set".to_string(),
            source: errno::Errno::ENOENT,
        })
    }

    /// get mode of devnode
    pub fn get_devnode_mode(&self) -> Result<mode_t, Error> {
        self.read_db()?;

        self.devmode.borrow().ok_or(Error::Nix {
            msg: "get_devnode_mode failed: devmode is not set".to_string(),
            source: errno::Errno::ENOENT,
        })
    }

    /// check whether the device object contains a devlink
    pub fn has_devlink(&self, devlink: &str) -> bool {
        self.devlinks.borrow().contains(devlink)
    }

    /// set the initialized timestamp
    pub fn set_usec_initialized(&self, time: u64) {
        self.add_property_internal("USEC_INITIALIZED", &time.to_string())
            .unwrap();
        self.usec_initialized.replace(time);
    }

    #[inline]
    fn cleanup(db: &str, tmp_file: &str) {
        let _ = unlink(db);
        let _ = unlink(tmp_file);
    }

    /// update device database
    pub fn update_db(&self) -> Result<(), Error> {
        let has_info = self.has_info();

        let id = self.get_device_id()?;

        let db_path = format!("{}/{}/{}", self.base_path.borrow(), DB_BASE_DIR, id);

        if !has_info && *self.devnum.borrow() == 0 && *self.ifindex.borrow() == 0 {
            if let Err(e) = unlink(db_path.as_str()).context(Nix {
                msg: format!("update_db failed: can't unlink db '{}'", db_path),
            }) {
                if e.get_errno() != nix::errno::Errno::ENOENT {
                    return Err(e);
                }
            }

            return Ok(());
        }

        create_dir_all(&format!("{}/{}", self.base_path.borrow(), DB_BASE_DIR)).context(Io {
            msg: "failed to create db directory".to_string(),
        })?;

        if let Err(e) = chmod(
            &format!("{}/{}", self.base_path.borrow(), DB_BASE_DIR),
            0o750,
        ) {
            log::error!("Failed to set permission for /run/devmaster/data/: {}", e);
        }

        let (mut file, tmp_file) = open_temporary(&db_path).context(Basic {
            msg: "can't open temporary file".to_string(),
        })?;

        if let Err(e) = self.atomic_create_db(&mut file, tmp_file.as_str(), db_path.as_str()) {
            Self::cleanup(&db_path, &tmp_file);
            return Err(e);
        }

        Ok(())
    }

    fn atomic_create_db(
        &self,
        file: &mut File,
        tmp_file: &str,
        db_path: &str,
    ) -> Result<(), Error> {
        fchmod(
            file.as_raw_fd(),
            if *self.db_persist.borrow() {
                Mode::from_bits(0o1640).unwrap()
            } else {
                Mode::from_bits(0o640).unwrap()
            },
        )
        .context(Nix {
            msg: "update_db failed: can't change the mode of temporary file".to_string(),
        })?;

        if self.has_info() {
            if *self.devnum.borrow() > 0 {
                for link in self.devlinks.borrow().iter() {
                    file.write(format!("S:{}\n", link.strip_prefix("/dev/").unwrap()).as_bytes())
                        .context(Io {
                            msg: format!("update_db failed: can't write devlink '{}' to db", link),
                        })?;
                }

                if *self.devlink_priority.borrow() != 0 {
                    file.write(format!("L:{}\n", self.devlink_priority.borrow()).as_bytes())
                        .context(Io {
                            msg: format!(
                                "update_db failed: can't write devlink priority '{}' to db",
                                *self.devlink_priority.borrow()
                            ),
                        })?;
                }
            }

            if *self.usec_initialized.borrow() > 0 {
                file.write(format!("I:{}\n", self.usec_initialized.borrow()).as_bytes())
                    .context(Io {
                        msg: format!(
                            "update_db failed: can't write initial usec '{}' to db",
                            *self.usec_initialized.borrow()
                        ),
                    })?;
            }

            for (k, v) in self.properties_db.borrow().iter() {
                file.write(format!("E:{}={}\n", k, v).as_bytes())
                    .context(Io {
                        msg: format!(
                            "update_db failed: can't write property '{}'='{}' to db",
                            k, v
                        ),
                    })?;
            }

            for tag in self.all_tags.borrow().iter() {
                file.write(format!("G:{}\n", tag).as_bytes()).context(Io {
                    msg: "update_db failed: can't write tag '{}' to db".to_string(),
                })?;
            }

            for tag in self.current_tags.borrow().iter() {
                file.write(format!("Q:{}\n", tag).as_bytes()).context(Io {
                    msg: format!(
                        "update_db failed: failed to write current tag '{}' to db",
                        tag
                    ),
                })?;
            }
        }

        file.flush().context(Io {
            msg: "update_db failed: can't flush db".to_string(),
        })?;

        rename(tmp_file, &db_path).context(Io {
            msg: "update_db failed: can't rename temporary file".to_string(),
        })?;

        Ok(())
    }

    /// update persist device tag
    pub fn update_tag(&self, tag: &str, add: bool) -> Result<(), Error> {
        let id = self.get_device_id()?;

        let tag_path = format!(
            "{}/{}/{}/{}",
            self.base_path.borrow(),
            TAGS_BASE_DIR,
            tag,
            id
        );

        if add {
            touch_file(&tag_path, true, Some(0o444), None, None).context(Basic {
                msg: format!("can't touch file '{}'", tag_path),
            })?;

            if let Err(e) = chmod(
                &format!("{}/{}", self.base_path.borrow(), TAGS_BASE_DIR),
                0o750,
            ) {
                log::error!(
                    "Failed to set permission for {}: {}",
                    format!("{}/{}", self.base_path.borrow(), TAGS_BASE_DIR),
                    e
                );
            }

            if let Err(e) = chmod(
                &format!("{}/{}/{}", self.base_path.borrow(), TAGS_BASE_DIR, tag),
                0o750,
            ) {
                log::error!(
                    "Failed to set permission for {}: {}",
                    format!("{}/{}/{}", self.base_path.borrow(), TAGS_BASE_DIR, tag),
                    e
                );
            }

            return Ok(());
        }

        match unlink(tag_path.as_str()) {
            Ok(_) => {}
            Err(e) => {
                if e != nix::Error::ENOENT {
                    return Err(Error::Nix {
                        msg: format!("update_tag failed: can't unlink tag '{}'", tag_path),
                        source: e,
                    });
                }
            }
        }

        Ok(())
    }

    /// set the device object to initialized
    pub fn set_is_initialized(&self) {
        self.is_initialized.replace(true);
    }

    /// read database
    pub fn read_db(&self) -> Result<(), Error> {
        self.read_db_internal(false)
    }

    /// read database internally
    pub fn read_db_internal(&self, force: bool) -> Result<(), Error> {
        if *self.db_loaded.borrow() || (!force && *self.sealed.borrow()) {
            return Ok(());
        }

        let id = self.get_device_id()?;

        let path = format!("{}/{}/{}", self.base_path.borrow(), DB_BASE_DIR, id);

        self.read_db_internal_filename(&path)
    }

    /// get properties nulstr, if it is out of date, update it
    pub fn get_properties_nulstr(&self) -> Result<(Vec<u8>, usize), Error> {
        self.update_properties_bufs()?;

        Ok((
            self.properties_nulstr.borrow().clone(),
            *self.properties_nulstr_len.borrow(),
        ))
    }

    /// create a Device instance based on mode and devnum
    pub fn from_mode_and_devnum(mode: mode_t, devnum: dev_t) -> Result<Device, Error> {
        let t: &str = if (mode & S_IFMT) == S_IFCHR {
            "char"
        } else if (mode & S_IFMT) == S_IFBLK {
            "block"
        } else {
            return Err(Error::Nix {
                msg: "from_mode_and_devnum failed: invalid mode".to_string(),
                source: Errno::ENOTTY,
            });
        };

        if major(devnum) == 0 {
            return Err(Error::Nix {
                msg: "from_mode_and_devnum failed: invalid devnum".to_string(),
                source: Errno::ENODEV,
            });
        }

        let syspath = format!("/sys/dev/{}/{}:{}", t, major(devnum), minor(devnum));

        let device = Device::default();
        device.set_syspath(&syspath, true)?;

        // verify devnum
        let devnum_ret = device.get_devnum()?;
        if devnum_ret != devnum {
            return Err(Error::Nix {
                msg: "from_mode_and_devnum failed: inconsistent devnum".to_string(),
                source: Errno::EINVAL,
            });
        }

        // verify subsystem
        let subsystem_ret = device.get_subsystem()?;
        if (subsystem_ret == "block") != ((mode & S_IFMT) == S_IFBLK) {
            return Err(Error::Nix {
                msg: "from_mode_and_devnum failed: inconsistent subsystem".to_string(),
                source: Errno::EINVAL,
            });
        }

        Result::Ok(device)
    }

    /// generate device object based on the environment properties
    pub fn from_environment() -> Result<Device, Error> {
        let device = Device::new();
        let mut major = String::new();
        let mut minor = String::new();
        for (key, value) in std::env::vars() {
            match key.as_str() {
                "MINOR" => minor = value.to_string(),
                "MAJOR" => major = value.to_string(),
                _ => device.amend_key_value(&key, &value)?,
            }
        }

        if !major.is_empty() {
            device.set_devnum(&major, &minor)?;
        }

        device.update_properties_bufs()?;
        device.verify()
    }

    /// set the syspath of Device
    /// constraint: path should start with /sys
    pub fn set_syspath(&self, path: &str, verify: bool) -> Result<(), Error> {
        let p = if verify {
            let path = match fs::canonicalize(path) {
                Ok(pathbuf) => pathbuf,
                Err(e) => {
                    if let Some(libc::ENOENT) = e.raw_os_error() {
                        return Err(Error::Nix {
                            msg: format!("set_syspath failed: invalid syspath '{}'", path),
                            source: Errno::ENODEV,
                        });
                    }

                    return Err(Error::Nix {
                        msg: format!("set_syspath failed: can't canonicalize '{}'", path),
                        source: Errno::from_i32(e.raw_os_error().unwrap_or_default()),
                    });
                }
            };

            if !path.starts_with("/sys") {
                // todo: what if sysfs is mounted on somewhere else?
                // systemd has considered this situation
                return Err(Error::Nix {
                    msg: format!("set_syspath failed: '{:?}' does not start with /sys", path),
                    source: Errno::EINVAL,
                });
            }

            if path.starts_with("/sys/devices/") {
                if !path.is_dir() {
                    return Err(Error::Nix {
                        msg: format!("set_syspath failed: '{:?}' is not a directory", path),
                        source: Errno::ENODEV,
                    });
                }

                let uevent_path = path.join("uevent");
                if !uevent_path.exists() {
                    return Err(Error::Nix {
                        msg: format!("set_syspath failed: '{:?}' does not contain uevent", path),
                        source: Errno::ENODEV,
                    });
                }
            } else if !path.is_dir() {
                return Err(Error::Nix {
                    msg: format!("set_syspath failed: '{:?}' is not a directory", path),
                    source: Errno::ENODEV,
                });
            }

            // refuse going down into /sys/fs/cgroup/ or similar places
            // where things are not arranged as kobjects in kernel

            /* The path is validated before, thus can directly be unwrapped from os str. */
            path.as_os_str().to_str().unwrap().to_string()
        } else {
            if !path.starts_with("/sys/") {
                return Err(Error::Nix {
                    msg: format!("set_syspath failed: '{:?}' does not start with /sys", path),
                    source: Errno::EINVAL,
                });
            }

            path.to_string()
        };

        /* The syspath is already validated to start with /sys. */
        let devpath = p.strip_prefix("/sys").unwrap();

        if !devpath.starts_with('/') {
            return Err(Error::Nix {
                msg: format!(
                    "set_syspath failed: devpath '{}' alone is not a valid device path",
                    p
                ),
                source: Errno::ENODEV,
            });
        }

        /* The key 'DEVPATH' is not empty, definitely be ok. */
        self.add_property_internal("DEVPATH", devpath).unwrap();

        self.devpath.replace(devpath.to_string());
        self.syspath.replace(p);

        Ok(())
    }

    /// set the sysname and sysnum of device object
    pub fn set_sysname_and_sysnum(&self) -> Result<(), Error> {
        /* The devpath is validated to begin with '/' when setting syspath. */
        let idx = match self.devpath.borrow().rfind('/') {
            Some(idx) => idx,
            None => {
                return Err(Error::Nix {
                    msg: format!(
                        "set_sysname_and_sysnum failed: devpath '{}' is not a valid device path",
                        self.devpath.borrow()
                    ),
                    source: Errno::ENODEV,
                });
            }
        };
        let sysname = String::from(&self.devpath.borrow()[idx + 1..]);
        let sysname = sysname.replace('!', "/");

        let mut ridx = sysname.len();
        while ridx > 0 {
            match sysname.chars().nth(ridx - 1) {
                Some(c) => {
                    if c.is_ascii_digit() {
                        ridx -= 1;
                    } else {
                        break;
                    }
                }
                None => break,
            }
        }

        if ridx < sysname.len() {
            self.sysnum.replace(String::from(&sysname[ridx..]));
        }

        self.sysname.replace(sysname);
        Ok(())
    }

    /// add property internal, in other words, do not write to external db
    pub fn add_property_internal(&self, key: &str, value: &str) -> Result<(), Error> {
        self.add_property_aux(key, value, false)
    }

    /// add property,
    /// if flag db is true, write to self.properties_db,
    /// else write to self.properties, and set self.properties_buf_outdated to true for updating
    pub fn add_property_aux(&self, key: &str, value: &str, db: bool) -> Result<(), Error> {
        if key.is_empty() {
            return Err(Error::Nix {
                msg: "add_property_aux failed: empty key".to_string(),
                source: Errno::EINVAL,
            });
        }

        let reference = if db {
            &self.properties_db
        } else {
            &self.properties
        };

        if value.is_empty() {
            reference.borrow_mut().remove(key);
        } else {
            reference
                .borrow_mut()
                .insert(key.to_string(), value.to_string());
        }

        if !db {
            self.properties_buf_outdated.replace(true);
        }

        Ok(())
    }

    /// update properties buffer
    pub fn update_properties_bufs(&self) -> Result<(), Error> {
        if !*self.properties_buf_outdated.borrow() {
            return Ok(());
        }
        self.properties_nulstr.borrow_mut().clear();
        for (k, v) in self.properties.borrow().iter() {
            unsafe {
                self.properties_nulstr
                    .borrow_mut()
                    .append(k.clone().as_mut_vec());
                self.properties_nulstr.borrow_mut().append(&mut vec![b'=']);
                self.properties_nulstr
                    .borrow_mut()
                    .append(v.clone().as_mut_vec());
                self.properties_nulstr.borrow_mut().append(&mut vec![0]);
            }
        }

        self.properties_nulstr_len
            .replace(self.properties_nulstr.borrow().len());
        self.properties_buf_outdated.replace(false);
        Ok(())
    }

    /// set subsystem
    pub fn set_subsystem(&self, subsystem: &str) {
        self.add_property_internal("SUBSYSTEM", subsystem).unwrap();
        self.subsystem_set.replace(true);
        self.subsystem.replace(subsystem.to_string());
    }

    /// set drivers subsystem
    pub fn set_drivers_subsystem(&self) -> Result<(), Error> {
        let mut subsystem = String::new();
        let components = self
            .devpath
            .borrow()
            .split('/')
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        for (idx, com) in components.iter().enumerate() {
            if com == "drivers" {
                subsystem = components.get(idx - 1).unwrap().to_string();
                break;
            }
        }

        if subsystem.is_empty() {
            return Err(Error::Nix {
                msg: "set_drivers_subsystem failed: empty subsystem".to_string(),
                source: Errno::EINVAL,
            });
        }

        self.set_subsystem("drivers");
        self.driver_subsystem.replace(subsystem);

        Ok(())
    }

    /// read uevent file and filling device attributes
    pub fn read_uevent_file(&self) -> Result<(), Error> {
        if *self.uevent_loaded.borrow() || *self.sealed.borrow() {
            return Ok(());
        }

        let uevent_file = format!("{}/uevent", self.syspath.borrow());

        let mut file = match fs::OpenOptions::new().read(true).open(uevent_file) {
            Ok(f) => f,
            Err(e) => {
                let n = e.raw_os_error().unwrap_or_default();

                if [libc::EACCES, libc::ENODEV, libc::ENXIO, libc::ENOENT].contains(&n) {
                    // the uevent file may be write-only, or the device may be already removed or the device has no uevent file
                    return Ok(());
                }
                return Err(Error::Nix {
                    msg: "read_uevent_file failed: can't open uevent file".to_string(),
                    source: Errno::from_i32(n),
                });
            }
        };

        let mut buf = String::new();
        file.read_to_string(&mut buf).unwrap();

        let mut major = "";
        let mut minor = "";

        for line in buf.split('\n') {
            let tokens: Vec<&str> = line.split('=').collect();
            if tokens.len() < 2 {
                break;
            }

            let (key, value) = (tokens[0], tokens[1]);

            match key {
                "MAJOR" => {
                    major = value;
                }
                "MINOR" => {
                    minor = value;
                }
                _ => {
                    self.amend_key_value(key, value)?;
                }
            }
        }

        if !major.is_empty() {
            self.set_devnum(major, minor)?;
        }

        self.uevent_loaded.replace(true);

        Ok(())
    }

    /// set devtype
    pub fn set_devtype(&self, devtype: &str) {
        self.add_property_internal("DEVTYPE", devtype).unwrap();
        self.devtype.replace(devtype.to_string());
    }

    /// set ifindex
    pub fn set_ifindex(&self, ifindex: &str) -> Result<(), Error> {
        self.add_property_internal("IFINDEX", ifindex).unwrap();
        self.ifindex
            .replace(ifindex.parse::<u32>().context(ParseInt {
                msg: format!("invalid integer '{}'", ifindex),
            })?);
        Ok(())
    }

    /// set devname
    pub fn set_devname(&self, devname: &str) {
        let devname = if devname.starts_with('/') {
            devname.to_string()
        } else {
            format!("/dev/{}", devname)
        };

        self.add_property_internal("DEVNAME", &devname).unwrap();
        self.devname.replace(devname);
    }

    /// set devmode
    pub fn set_devmode(&self, devmode: &str) -> Result<(), Error> {
        let m = Some(mode_t::from_str_radix(devmode, 8).context(ParseInt {
            msg: format!("invalid octal mode '{}'", devmode),
        })?);

        self.devmode.replace(m);

        self.add_property_internal("DEVMODE", devmode).unwrap();

        Ok(())
    }

    /// set device uid
    pub fn set_devuid(&self, devuid: &str) -> Result<(), Error> {
        let uid = devuid.parse::<uid_t>().context(ParseInt {
            msg: format!("invalid uid '{}'", devuid),
        })?;

        self.devuid.replace(Some(Uid::from_raw(uid)));

        self.add_property_internal("DEVUID", devuid)?;

        Ok(())
    }

    /// set device gid
    pub fn set_devgid(&self, devgid: &str) -> Result<(), Error> {
        let gid = devgid.parse::<gid_t>().context(ParseInt {
            msg: format!("invalid gid '{}'", devgid),
        })?;

        self.devgid.replace(Some(Gid::from_raw(gid)));

        self.add_property_internal("DEVGID", devgid).unwrap();

        Ok(())
    }

    /// set devnum
    pub fn set_devnum(&self, major: &str, minor: &str) -> Result<(), Error> {
        let major_num: u64 = major.parse().context(ParseInt {
            msg: format!("invalid major number '{}'", major),
        })?;

        let minor_num: u64 = match minor.parse() {
            Ok(n) => n,
            Err(e) => {
                return Err(Error::Nix {
                    msg: format!("set_devnum failed: invalid minor number '{}': {}", minor, e),
                    source: Errno::EINVAL,
                });
            }
        };

        self.add_property_internal("MAJOR", major).unwrap();
        self.add_property_internal("MINOR", minor).unwrap();
        self.devnum.replace(makedev(major_num, minor_num));

        Ok(())
    }

    /// set diskseq
    pub fn set_diskseq(&self, diskseq: &str) -> Result<(), Error> {
        self.add_property_internal("DISKSEQ", diskseq).unwrap();

        let diskseq_num: u64 = diskseq.parse().context(ParseInt {
            msg: format!("invalid diskseq '{}'", diskseq),
        })?;

        self.diskseq.replace(diskseq_num);

        Ok(())
    }

    /// set action
    pub fn set_action(&self, action: DeviceAction) {
        self.add_property_internal("ACTION", &action.to_string())
            .unwrap();
        self.action.replace(action);
    }

    /// set action from string
    pub fn set_action_from_string(&self, action_s: &str) -> Result<(), Error> {
        let action = action_s.parse::<DeviceAction>()?;

        self.set_action(action);

        Ok(())
    }

    /// set seqnum from string
    pub fn set_seqnum_from_string(&self, seqnum_s: &str) -> Result<(), Error> {
        let seqnum: u64 = seqnum_s.parse().context(ParseInt {
            msg: format!("invalid seqnum '{}'", seqnum_s),
        })?;
        self.set_seqnum(seqnum);
        Ok(())
    }

    /// set seqnum
    pub fn set_seqnum(&self, seqnum: u64) {
        self.add_property_internal("SEQNUM", &seqnum.to_string())
            .unwrap();
        self.seqnum.replace(seqnum);
    }

    /// set driver
    pub fn set_driver(&self, driver: &str) {
        self.add_property_internal("DRIVER", driver).unwrap();
        self.driver_set.replace(true);
        self.driver.replace(driver.to_string());
    }

    /// cache sysattr value
    pub fn cache_sysattr_value(&self, sysattr: &str, value: &str) -> Result<(), Error> {
        if value.is_empty() {
            self.remove_cached_sysattr_value(sysattr);
        } else {
            self.sysattr_values
                .borrow_mut()
                .insert(sysattr.to_string(), value.to_string());
        }

        Ok(())
    }

    /// remove cached sysattr value
    pub fn remove_cached_sysattr_value(&self, sysattr: &str) {
        self.sysattr_values.borrow_mut().remove(sysattr);
    }

    /// get cached sysattr value
    pub fn get_cached_sysattr_value(&self, sysattr: &str) -> Result<String, Error> {
        if !self.sysattr_values.borrow().contains_key(sysattr) {
            return Err(Error::Nix {
                msg: format!("no cached sysattr '{}'", sysattr),
                source: Errno::ESTALE,
            });
        }

        self.sysattr_values
            .borrow()
            .get(sysattr)
            .cloned()
            .ok_or(Error::Nix {
                msg: format!("non-existing sysattr '{}'", sysattr),
                source: Errno::ENOENT,
            })
    }

    /// new from child
    pub fn new_from_child(device: &Device) -> Result<Device, Error> {
        let syspath = device.get_syspath()?;
        let syspath = Path::new(&syspath);

        let mut parent = syspath.parent();

        loop {
            match parent {
                Some(p) => {
                    if p == Path::new("/sys") {
                        return Err(Error::Nix {
                            msg: "no available parent device".to_string(),
                            source: Errno::ENODEV,
                        });
                    }

                    let path = p
                        .to_str()
                        .ok_or(Error::Nix {
                            msg: format!("invalid path '{:?}'", p),
                            source: Errno::ENODEV,
                        })?
                        .to_string();

                    match Device::from_syspath(&path, true) {
                        Ok(d) => return Ok(d),
                        Err(e) => {
                            if e.get_errno() != Errno::ENODEV {
                                return Err(Error::Nix {
                                    msg: format!("new_from_child failed: {}", e),
                                    source: e.get_errno(),
                                });
                            }
                        }
                    }
                }
                None => {
                    return Err(Error::Nix {
                        msg: "new_from_child failed: no available parent device".to_string(),
                        source: Errno::ENODEV,
                    });
                }
            }

            parent = parent.unwrap().parent();
        }
    }

    /// Get the child device if it exists.
    ///
    /// The parent will try to find the child in its cache firstly. If
    /// it already exists in the cache, directly return it. Otherwise
    /// the child device will be created and be cached.
    pub fn get_child(&self, child: &str) -> Result<Rc<Device>, Error> {
        if let Some(d) = self.children.borrow().get(child) {
            return Ok(d.clone());
        }

        let dev = Rc::new(Self::from_syspath(
            &format!("{}/{}", self.get_syspath()?, child),
            true,
        )?);

        self.children
            .borrow_mut()
            .insert(child.to_string(), dev.clone());

        Ok(dev)
    }

    /// prepare properties:
    /// 1. read from uevent file
    /// 2. read database
    /// 3. if self devlinks are outdated, add to internal property
    /// 4. if self tags are outdated ,add to internal property
    pub fn properties_prepare(&self) -> Result<(), Error> {
        self.read_uevent_file()?;

        self.read_db()?;

        let property_devlinks_outdated = *self.property_devlinks_outdated.borrow();
        if property_devlinks_outdated {
            let devlinks: String = {
                let devlinks = self.devlinks.borrow();
                let devlinks_vec = devlinks.iter().map(|s| s.as_str()).collect::<Vec<_>>();
                devlinks_vec.join(" ")
            };

            if !devlinks.is_empty() {
                self.add_property_internal("DEVLINKS", &devlinks).unwrap();

                self.property_devlinks_outdated.replace(false);
            }
        }

        let property_tags_outdated = *self.property_tags_outdated.borrow();
        if property_tags_outdated {
            let mut all_tags: String = {
                let all_tags = self.all_tags.borrow();
                all_tags.iter().map(|s| format!(":{}", s)).collect()
            };

            if !all_tags.is_empty() {
                all_tags.push(':');
                self.add_property_internal("TAGS", &all_tags).unwrap();
            }

            let mut current_tags: String = {
                let current_tags = self.current_tags.borrow();
                current_tags.iter().map(|s| format!(":{}", s)).collect()
            };

            if !current_tags.is_empty() {
                current_tags.push(':');
                self.add_property_internal("CURRENT_TAGS", &current_tags)
                    .unwrap();
            }

            self.property_tags_outdated.replace(false);
        }

        Ok(())
    }

    /// read database internally from specific file
    pub fn read_db_internal_filename(&self, filename: &str) -> Result<(), Error> {
        let mut file = match fs::OpenOptions::new().read(true).open(filename) {
            Ok(f) => f,
            Err(e) => {
                let n = e.raw_os_error().unwrap_or_default();
                if n == libc::ENOENT {
                    return Ok(());
                }
                return Err(Error::Nix {
                    msg: format!(
                        "read_db_internal_filename failed: can't open db '{}': {}",
                        filename, e
                    ),
                    source: Errno::from_i32(n),
                });
            }
        };

        let mut buf = String::new();
        file.read_to_string(&mut buf).context(Io {
            msg: format!("can't read db '{}'", filename),
        })?;

        self.is_initialized.replace(true);
        self.db_loaded.replace(true);

        for line in buf.split('\n') {
            if line.is_empty() {
                continue;
            }

            let key = &line[0..1];
            let value = &line[2..];

            self.handle_db_line(key, value)?;
        }

        Ok(())
    }

    /// handle database line
    pub fn handle_db_line(&self, key: &str, value: &str) -> Result<(), Error> {
        match key {
            "G" | "Q" => {
                self.add_tag(value, key == "Q");
            }
            "S" => {
                self.add_devlink(&format!("/dev/{}", value)).unwrap();
            }
            "E" => {
                let tokens: Vec<_> = value.split('=').collect();
                if tokens.len() != 2 {
                    return Err(Error::Nix {
                        msg: format!(
                            "handle_db_line failed: failed to parse property '{}'",
                            value
                        ),
                        source: Errno::EINVAL,
                    });
                }

                let (k, v) = (tokens[0], tokens[1]);

                self.add_property_internal(k, v)?;
            }
            "I" => {
                let time = value.parse::<u64>().context(ParseInt {
                    msg: format!("invalid usec integer '{}'", value),
                })?;

                self.set_usec_initialized(time);
            }
            "L" => {
                let priority = value.parse::<i32>().context(ParseInt {
                    msg: format!("invalid link priority integer '{}'", value),
                })?;

                self.devlink_priority.replace(priority);
            }
            "W" => {
                log::debug!("watch handle in database is deprecated.");
            }
            "V" => {
                let version = value.parse::<u32>().context(ParseInt {
                    msg: format!("invalid db version integer '{}'", value),
                })?;

                self.database_version.replace(version);
            }
            _ => {
                log::debug!("unknown key '{}' in database line, ignoring", key);
            }
        }

        Ok(())
    }

    /// shallow clone a device object
    pub fn shallow_clone(&self) -> Result<Device, Error> {
        let device = Self::default();

        device.set_base_path(self.base_path.borrow().as_str());

        let syspath = self.get_syspath()?;

        device.set_syspath(&syspath, false)?;

        /* Some devices, such as /sys/devices/platform, do not have subsystem. */
        if let Ok(subsystem) = self.get_subsystem() {
            device.set_subsystem(&subsystem);

            if subsystem == "drivers" {
                device
                    .driver_subsystem
                    .replace(self.driver_subsystem.borrow().clone());
            }
        }

        if let Ok(ifindex) = self.get_property_value("IFINDEX") {
            device.set_ifindex(&ifindex)?;
        }

        if let Ok(major) = self.get_property_value("MAJOR") {
            let minor = self.get_property_value("MINOR")?;
            device.set_devnum(&major, &minor)?;
        }

        device.read_uevent_file()?;

        Ok(device)
    }

    /// amend key and value to device object
    pub fn amend_key_value(&self, key: &str, value: &str) -> Result<(), Error> {
        match key {
            "DEVPATH" => self.set_syspath(&format!("/sys{}", value), false)?,
            "ACTION" => self.set_action_from_string(value)?,
            "SUBSYSTEM" => self.set_subsystem(value),
            "DEVTYPE" => self.set_devtype(value),
            "DEVNAME" => self.set_devname(value),
            "SEQNUM" => self.set_seqnum_from_string(value)?,
            "DRIVER" => self.set_driver(value),
            "IFINDEX" => self.set_ifindex(value)?,
            "USEC_INITIALIZED" => {
                self.set_usec_initialized(value.parse::<u64>().context(ParseInt {
                    msg: format!("invalid usec integer '{}'", value),
                })?);
            }
            "DEVMODE" => self.set_devmode(value)?,
            "DEVUID" => self.set_devuid(value)?,
            "DEVGID" => self.set_devgid(value)?,
            "DISKSEQ" => self.set_diskseq(value)?,
            "DEVLINKS" => self.add_devlinks(value)?,
            "TAGS" | "CURRENT_TAGS" => self.add_tags(value, key == "CURRENT_TAGS"),
            _ => self.add_property_internal(key, value)?,
        }

        Ok(())
    }

    #[inline]
    fn has_info(&self) -> bool {
        !self.devlinks.borrow().is_empty()
            || !self.properties_db.borrow().is_empty()
            || !self.all_tags.borrow().is_empty()
            || !self.current_tags.borrow().is_empty()
    }

    pub(crate) fn enumerate_children(&self) -> Result<bool, Error> {
        if *self.children_enumerated.borrow() {
            return Ok(false);
        }

        let stk = RefCell::new(VecDeque::<String>::new());

        self.enumerate_children_internal("", &stk)?;

        #[allow(clippy::while_let_loop)]
        loop {
            let subdir = match stk.borrow_mut().pop_front() {
                Some(s) => s,
                None => break,
            };

            self.enumerate_children_internal(&subdir, &stk)?;
        }

        let _ = self.children_enumerated.replace(true);

        Ok(true)
    }

    pub(crate) fn enumerate_children_internal(
        &self,
        subdir: &str,
        stk: &RefCell<VecDeque<String>>,
    ) -> Result<(), Error> {
        for entry in self.read_dir(subdir)? {
            let de = match entry {
                Ok(e) => e,
                Err(_) => {
                    continue;
                }
            };

            let de_name = match de.file_name().to_str() {
                Some(name) => {
                    if [".", ".."].contains(&name) {
                        continue;
                    }

                    name.to_string()
                }
                None => {
                    continue;
                }
            };

            match de.file_type() {
                Ok(t) => {
                    if !t.is_dir() && !t.is_symlink() {
                        continue;
                    }
                }
                Err(_) => {
                    continue;
                }
            }

            let path = if subdir.is_empty() {
                de_name
            } else {
                format!("{}/{}", subdir, de_name)
            };

            if let Err(e) = self.get_child(&path) {
                if e.is_errno(nix::Error::ENODEV) {
                    debug_assert!(de.file_type().is_ok());

                    /* Avoid infinite loop */
                    if de.file_type().unwrap().is_symlink() {
                        continue;
                    }

                    /*
                     * If the current sub-directory is not a device,
                     * push it to the statck and enumerate deeper until
                     * a child device is found.
                     */
                    stk.borrow_mut().push_back(path);
                } else {
                    return Err(e);
                }
            }
        }

        Ok(())
    }

    /// Read a directory and return 'ReadDir'
    pub fn read_dir(&self, subdir: &str) -> Result<ReadDir, Error> {
        let syspath = self.get_syspath()?;

        let dir = if syspath.is_empty() {
            syspath
        } else {
            format!("{}/{}", syspath, subdir)
        };

        std::fs::read_dir(&dir).context(Io {
            msg: format!("Failed to read directory '{}'", &dir),
        })
    }

    /// Open a subdirectory and return 'Dir'
    pub fn open_dir(&self, subdir: &str) -> Result<Dir, Error> {
        let syspath = self.get_syspath()?;

        let dir = if syspath.is_empty() {
            syspath
        } else {
            format!("{}/{}", syspath, subdir)
        };

        Dir::open(
            dir.as_str(),
            OFlag::O_DIRECTORY,
            Mode::from_bits_truncate(0o000),
        )
        .context(Nix {
            msg: format!("Failed to open directory '{}'", &dir),
        })
    }

    fn read_all_sysattrs(&self) -> Result<(), Error> {
        if *self.sysattrs_cached.borrow() {
            return Ok(());
        }

        let stk = RefCell::new(VecDeque::<String>::new());
        self.read_all_sysattrs_internal("", &stk)?;

        #[allow(clippy::while_let_loop)]
        loop {
            let subdir = match stk.borrow_mut().pop_front() {
                Some(v) => v,
                None => break,
            };

            self.read_all_sysattrs_internal(&subdir, &stk)?;
        }

        self.sysattrs_cached.replace(true);

        Ok(())
    }

    fn read_all_sysattrs_internal(
        &self,
        subdir: &str,
        stk: &RefCell<VecDeque<String>>,
    ) -> Result<(), Error> {
        let dir = match self.open_dir(subdir) {
            Ok(d) => d,
            Err(e) => {
                if e.is_errno(nix::Error::ENOENT) && !subdir.is_empty() {
                    return Ok(());
                }

                return Err(Error::Nix {
                    msg: format!("failed to read subdirectory '{}'", subdir),
                    source: e.get_errno(),
                });
            }
        };

        if !subdir.is_empty() {
            let uevent_str = match CString::new("uevent") {
                Ok(uevent_str) => uevent_str,
                Err(e) => {
                    return Err(Error::Nix {
                        msg: format!("failed to new CString({:?}) '{}'", "uevent", e),
                        source: nix::Error::EINVAL,
                    })
                }
            };
            if unsafe { faccessat(dir.as_raw_fd(), uevent_str.as_ptr(), F_OK, 0) } >= 0 {
                /* skip child device */
                return Ok(());
            }
            let error = nix::Error::from_i32(nix::errno::errno());
            if error != nix::Error::ENOENT {
                log::debug!(
                    "{}: failed to access {}/uevent, ignoring subdirectory {}: {}",
                    self.sysname.borrow(),
                    subdir,
                    subdir,
                    error
                );

                return Ok(());
            }
        }

        for de in self.read_dir(subdir)? {
            let de = match de {
                Ok(i) => i,
                Err(_) => continue,
            };

            match de.file_name().to_str() {
                Some(s) => {
                    if [".", ".."].contains(&s) {
                        continue;
                    }
                }
                None => {
                    continue;
                }
            }

            /* only handle symlinks, regular files, and directories */
            match de.file_type() {
                Ok(t) => {
                    if !t.is_dir() && !t.is_file() & !t.is_symlink() {
                        continue;
                    }
                }
                Err(_) => {
                    continue;
                }
            }

            let p = if !subdir.is_empty() {
                format!("{}/{}", subdir, de.file_name().to_str().unwrap())
            } else {
                de.file_name().to_str().unwrap().to_string()
            };

            if de.file_type().unwrap().is_dir() {
                stk.borrow_mut().push_back(p.clone());
                continue;
            }

            let stat = match nix::sys::stat::fstatat(
                dir.as_raw_fd(),
                de.file_name().to_str().unwrap(),
                AtFlags::AT_SYMLINK_NOFOLLOW,
            ) {
                Ok(s) => s,
                Err(_) => continue,
            };

            if (stat.st_mode & (S_IRUSR | S_IWUSR)) == 0 {
                continue;
            }

            /*
             * Some attributes are a symlink to other device, ignoring them.
             *
             * Only regard 'subsystem', 'driver' and 'module' as legal if
             * the attribute is a symlink.
             */
            if de.file_type().unwrap().is_symlink()
                && !["driver", "subsystem", "module"].contains(&p.as_str())
            {
                continue;
            }

            let _ = self.sysattrs.borrow_mut().insert(p);
        }

        Ok(())
    }

    /// check whether a device matches parent
    pub fn match_parent(
        &self,
        match_parent: &HashSet<String>,
        nomatch_parent: &HashSet<String>,
    ) -> bool {
        let syspath = match self.get_syspath() {
            Ok(syspath) => syspath,
            Err(_err) => return false,
        };

        for syspath_parent in nomatch_parent {
            if syspath.starts_with(syspath_parent) {
                return false;
            }
        }

        if match_parent.is_empty() {
            return true;
        }

        for syspath_parent in match_parent {
            if syspath.starts_with(syspath_parent) {
                return true;
            }
        }

        false
    }

    /// check whether the sysattrs of a device matches
    pub fn match_sysattr(
        &self,
        match_sysattr: &HashMap<String, String>,
        nomatch_sysattr: &HashMap<String, String>,
    ) -> bool {
        for (sysattr, patterns) in match_sysattr {
            if !self.match_sysattr_value(sysattr, patterns) {
                return false;
            }
        }

        for (sysattr, patterns) in nomatch_sysattr {
            if self.match_sysattr_value(sysattr, patterns) {
                return false;
            }
        }

        true
    }

    /// check whether the value of specific sysattr of a device matches
    fn match_sysattr_value(&self, sysattr: &str, patterns: &str) -> bool {
        let value = match self.get_sysattr_value(sysattr) {
            Ok(value) => value,
            Err(_) => return false,
        };

        if patterns.is_empty() {
            return true;
        }

        fnmatch_or_empty(patterns, &value, 0)
    }
}

/// iterator wrapper of hash set in refcell
pub struct HashSetRefWrapper<'a, T: 'a> {
    r: Ref<'a, HashSet<T>>,
}

impl<'a, 'b: 'a, T: 'a> IntoIterator for &'b HashSetRefWrapper<'a, T> {
    type IntoIter = Iter<'a, T>;
    type Item = &'a T;

    fn into_iter(self) -> Iter<'a, T> {
        self.r.iter()
    }
}

/// iterator wrapper of hash map in refcell
pub struct HashMapRefWrapper<'a, K: 'a, V: 'a> {
    r: Ref<'a, HashMap<K, V>>,
}

impl<'a, 'b: 'a, K: 'a, V: 'a> IntoIterator for &'b HashMapRefWrapper<'a, K, V> {
    type IntoIter = std::collections::hash_map::Iter<'a, K, V>;
    type Item = (&'a K, &'a V);

    fn into_iter(self) -> std::collections::hash_map::Iter<'a, K, V> {
        self.r.iter()
    }
}

impl Device {
    /// Return the tag iterator.
    ///
    /// The device object will try to load tags from db firstly.
    pub fn tag_iter(&self) -> HashSetRefWrapper<String> {
        let _ = self.read_db();

        HashSetRefWrapper {
            r: self.all_tags.borrow(),
        }
    }

    /// Return the current tag iterator.
    ///
    /// The device object will try to load tags from db firstly.
    pub fn current_tag_iter(&self) -> HashSetRefWrapper<String> {
        let _ = self.read_db();

        HashSetRefWrapper {
            r: self.current_tags.borrow(),
        }
    }

    /// Return the devlink iterator
    ///
    /// The device object will try to load devlinks from db firstly.
    pub fn devlink_iter(&self) -> HashSetRefWrapper<String> {
        let _ = self.read_db();

        HashSetRefWrapper {
            r: self.devlinks.borrow(),
        }
    }

    /// return the tag iterator
    pub fn property_iter(&self) -> HashMapRefWrapper<String, String> {
        if let Err(e) = self.properties_prepare() {
            log::debug!(
                "failed to prepare properties of '{}': {}",
                self.get_device_id()
                    .unwrap_or_else(|_| self.devpath.borrow().clone()),
                e
            )
        }

        HashMapRefWrapper {
            r: self.properties.borrow(),
        }
    }

    /// return the child iterator
    pub fn child_iter(&self) -> HashMapRefWrapper<String, Rc<Device>> {
        if let Err(e) = self.enumerate_children() {
            log::debug!(
                "failed to enumerate children of '{}': {}",
                self.get_device_id()
                    .unwrap_or_else(|_| self.devpath.borrow().clone()),
                e
            )
        }

        HashMapRefWrapper {
            r: self.children.borrow(),
        }
    }

    /// return the sysattr iterator
    pub fn sysattr_iter(&self) -> HashSetRefWrapper<String> {
        if !*self.sysattrs_cached.borrow() {
            if let Err(e) = self.read_all_sysattrs() {
                log::debug!(
                    "{}: failed to read all sysattrs: {}",
                    self.get_sysname()
                        .unwrap_or_else(|_| self.devpath.borrow().clone()),
                    e
                );
            }
        }

        HashSetRefWrapper {
            r: self.sysattrs.borrow(),
        }
    }
}

impl PartialEq for Device {
    fn eq(&self, other: &Self) -> bool {
        self.get_syspath().unwrap_or_default() == other.get_syspath().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        device::*,
        device_enumerator::{DeviceEnumerationType, DeviceEnumerator},
    };
    use basic::IN_SET;
    use libc::S_IFBLK;
    use std::fs::OpenOptions;
    use std::panic::catch_unwind;

    #[cfg(feature = "loopdev")]
    use crate::utils::LoopDev;

    fn compare(dev1: &Device, dev2: &Device) -> bool {
        let syspath_1 = dev1.get_syspath().unwrap();
        let syspath_2 = dev2.get_syspath().unwrap();
        syspath_1 == syspath_2
    }

    /// test a single device
    fn test_device_one(device: Rc<Device>) {
        let syspath = device.get_syspath().unwrap();
        assert!(syspath.starts_with("/sys"));
        let sysname = device.get_sysname().unwrap();

        // test Device::from_syspath()
        let device_new = Device::from_syspath(&syspath, true).unwrap();
        let syspath_new = device_new.get_syspath().unwrap();
        assert_eq!(syspath, syspath_new);

        // test Device::from_path()
        let device_new = Device::from_path(&syspath).unwrap();
        let syspath_new = device_new.get_syspath().unwrap();
        assert_eq!(syspath, syspath_new);

        // test Device::from_ifindex()
        match device.get_ifindex() {
            Ok(ifindex) => match Device::from_ifindex(ifindex) {
                Ok(dev) => {
                    assert_eq!(syspath, dev.get_syspath().unwrap());
                }
                Err(e) => {
                    assert_eq!(e.get_errno(), Errno::ENODEV);
                }
            },
            Err(e) => {
                assert_eq!(e.get_errno(), Errno::ENOENT);
            }
        }

        let mut is_block = false;

        // test Device::from_subsystem_sysname
        match device.get_subsystem() {
            Ok(subsystem) => {
                if !subsystem.is_empty() && subsystem != "gpio" {
                    is_block = subsystem == "block";
                    let name = if subsystem == "drivers" {
                        format!("{}:{}", device.driver_subsystem.borrow(), sysname)
                    } else {
                        sysname
                    };

                    match Device::from_subsystem_sysname(&subsystem, &name) {
                        Ok(dev) => {
                            assert_eq!(syspath, dev.get_syspath().unwrap());
                        }
                        Err(e) => {
                            assert_eq!(e.get_errno(), Errno::ENODEV);
                        }
                    }

                    let device_id = device.get_device_id().unwrap();
                    match Device::from_device_id(&device_id) {
                        Ok(dev) => {
                            assert_eq!(device_id, dev.get_device_id().unwrap());
                            assert_eq!(syspath, dev.get_syspath().unwrap());
                        }
                        Err(e) => {
                            assert_eq!(e.get_errno(), Errno::ENODEV);
                        }
                    }

                    if device.get_is_initialized().unwrap() {
                        match device.get_usec_initialized() {
                            Ok(usec) => {
                                assert!(usec > 0);
                            }
                            Err(e) => {
                                assert_eq!(e.get_errno(), nix::Error::ENODATA);
                            }
                        }
                    }

                    match device.get_property_value("ID_NET_DRIVER") {
                        Ok(_) => {}
                        Err(e) => {
                            assert_eq!(e.get_errno(), Errno::ENOENT);
                        }
                    }

                    let _ = device.get_parent_with_subsystem_devtype("usb", Some("usb_interface"));
                }
            }
            Err(e) => {
                assert_eq!(e.get_errno(), Errno::ENOENT);
            }
        };

        match device.get_devname() {
            Ok(devname) => {
                match Device::from_devname(&devname) {
                    Ok(device_new) => {
                        let syspath_new = device_new.get_syspath().unwrap();
                        assert_eq!(syspath, syspath_new);
                    }
                    Err(e) => {
                        assert!(
                            [Errno::ENODEV, Errno::EACCES, Errno::EPERM].contains(&e.get_errno())
                        );
                    }
                };

                match Device::from_path(&devname) {
                    Ok(device_new) => {
                        let syspath_new = device_new.get_syspath().unwrap();
                        assert_eq!(syspath, syspath_new);

                        // todo: device_open
                        match device.open(
                            OFlag::O_CLOEXEC
                                | OFlag::O_NONBLOCK
                                | if is_block {
                                    OFlag::O_RDONLY
                                } else {
                                    OFlag::O_NOCTTY | OFlag::O_PATH
                                },
                        ) {
                            Ok(f) => {
                                assert!(f.as_raw_fd() >= 0)
                            }
                            Err(e) => {
                                assert!(basic::error::errno_is_privilege(e.get_errno()));
                            }
                        }

                        let dev2 = Device::from_path(&device.get_syspath().unwrap()).unwrap();
                        assert!(compare(&device_new, &dev2));
                    }
                    Err(e) => {
                        assert!(
                            [Errno::ENODEV, Errno::EACCES, Errno::EPERM].contains(&e.get_errno())
                        );
                    }
                };
            }
            Err(e) => {
                assert_eq!(e.get_errno(), Errno::ENOENT);
            }
        }

        match device.get_devnum() {
            Ok(devnum) => {
                let device_new = Device::from_devnum(
                    {
                        if is_block {
                            'b'
                        } else {
                            'c'
                        }
                    },
                    devnum,
                )
                .unwrap();
                let syspath_new = device_new.get_syspath().unwrap();
                assert_eq!(syspath, syspath_new);

                let devname = format!(
                    "/dev/{}/{}:{}",
                    {
                        if is_block {
                            "block"
                        } else {
                            "char"
                        }
                    },
                    major(devnum),
                    minor(devnum)
                );
                let device_new = Device::from_devname(&devname).unwrap();
                let syspath_new = device_new.get_syspath().unwrap();
                assert_eq!(syspath, syspath_new);

                let device_new = Device::from_path(&devname).unwrap();
                let syspath_new = device_new.get_syspath().unwrap();
                assert_eq!(syspath, syspath_new);
            }
            Err(e) => {
                assert_eq!(e.get_errno(), Errno::ENOENT);
            }
        }

        device.get_devpath().unwrap();

        match device.get_devtype() {
            Ok(_) => {}
            Err(e) => {
                assert_eq!(e.get_errno(), Errno::ENOENT);
            }
        };

        match device.get_driver() {
            Ok(_) => {}
            Err(e) => {
                assert_eq!(e.get_errno(), Errno::ENOENT);
            }
        };

        match device.get_sysnum() {
            Ok(sysnum) => {
                sysnum.parse::<u64>().unwrap();
            }
            Err(e) => {
                assert_eq!(e.get_errno(), Errno::ENOENT);
            }
        }

        match device.get_sysattr_value("nsid") {
            Ok(value) => {
                println!("{}", value);
                value.trim().parse::<u32>().unwrap();
            }
            Err(e) => {
                assert!([Errno::EACCES, Errno::EPERM, Errno::ENOENT, Errno::EINVAL]
                    .contains(&e.get_errno()));
            }
        }

        if let Err(e) = device.get_diskseq() {
            assert_eq!(e.get_errno(), errno::Errno::ENOENT);
        }

        if let Err(e) = device.get_seqnum() {
            assert_eq!(e.get_errno(), errno::Errno::ENOENT);
        }

        if let Err(e) = device.get_trigger_uuid() {
            assert_eq!(e.get_errno(), nix::errno::Errno::ENOENT);
        }

        match device.get_devname() {
            Ok(devname) => {
                let st = nix::sys::stat::stat(devname.as_str()).unwrap();
                let uid = st.st_uid;
                let gid = st.st_gid;
                let mode = st.st_mode;

                match device.get_devnode_uid() {
                    Ok(dev_uid) => {
                        assert_eq!(uid, dev_uid.as_raw());
                    }
                    Err(e) => {
                        assert!(IN_SET!(e.get_errno(), nix::errno::Errno::ENOENT));
                    }
                }

                match device.get_devnode_gid() {
                    Ok(dev_gid) => {
                        assert_eq!(gid, dev_gid.as_raw());
                    }
                    Err(e) => {
                        assert!(IN_SET!(e.get_errno(), nix::errno::Errno::ENOENT));
                    }
                }

                match device.get_devnode_mode() {
                    Ok(dev_mode) => {
                        assert_eq!(mode & 0o777, dev_mode);
                    }
                    Err(e) => {
                        assert!(IN_SET!(e.get_errno(), nix::errno::Errno::ENOENT));
                    }
                }
            }
            Err(e) => {
                assert!(IN_SET!(e.get_errno(), nix::errno::Errno::ENOENT));
            }
        }

        if let Err(e) = device.get_action() {
            assert_eq!(e.get_errno(), nix::errno::Errno::ENOENT);
        }

        let _shadow = device.shallow_clone().unwrap();
        let _db = device.clone_with_db().unwrap();

        /* Test set and get devlink priority. */
        device.set_devlink_priority(10);
        assert_eq!(10, device.get_devlink_priority().unwrap());

        /* Test add devlinks */
        device.add_devlinks("/dev/test /dev/test1").unwrap();

        assert_eq!(
            device.add_devlink("/root/test").unwrap_err().get_errno(),
            nix::Error::EINVAL
        );
        assert_eq!(
            device.add_devlink("/dev").unwrap_err().get_errno(),
            nix::Error::EINVAL
        );

        /* Test other add_* methods. */
        device.add_property("A", "AA").unwrap();
        device.add_property("B", "BB").unwrap();
        device.add_tags("A:B:C", true);

        device.update_db().unwrap();

        /* Test enumerating child devices */
        for (subdir, child) in &device.child_iter() {
            let canoicalized_path =
                std::fs::canonicalize(&format!("{}/{}", &syspath, subdir)).unwrap();
            assert_eq!(
                canoicalized_path.to_str().unwrap(),
                &child.get_syspath().unwrap()
            );
        }

        /* Test iterate all attributes */
        for sysattr in &device.sysattr_iter() {
            let p = format!("{}/{}", syspath, sysattr);
            let path = Path::new(&p);
            assert!(path.exists());

            let st = nix::sys::stat::stat(path).unwrap();
            if st.st_mode & S_IWUSR == 0 {
                assert!(device
                    .set_sysattr_value(sysattr.as_str(), Some(""))
                    .is_err());
            }

            let _value = device.get_sysattr_value(sysattr.as_str());
        }

        /* Test iterators */
        for tag in &device.tag_iter() {
            assert!(device.has_tag(tag.as_str()).unwrap());
        }

        for tag in &device.current_tag_iter() {
            assert!(device.has_current_tag(tag.as_str()).unwrap());
        }

        for devlink in &device.devlink_iter() {
            assert!(device.has_devlink(devlink.as_str()));
        }

        for (k, v) in &device.property_iter() {
            assert_eq!(&device.get_property_value(k.as_str()).unwrap(), v);
        }

        device.cleanup_devlinks();
        device.cleanup_tags();

        let db = device.get_device_id().unwrap();
        let _ = unlink(format!("/tmp/devmaster/data/{}", db).as_str());

        /* Test open device node. */
        let _ = device.open(OFlag::O_RDONLY);
        let _ = device.open(OFlag::O_WRONLY);
        let _ = device.open(OFlag::O_RDWR);
        let _ = device.open(OFlag::O_EXCL);

        /* Cover exceptional code branches. */
        let _ = device.tag_iter();
        let _ = device.current_tag_iter();
        let _ = device.devlink_iter();
        let _ = device.property_iter();
    }

    #[test]
    fn test_devices_all() {
        let mut enumerator = DeviceEnumerator::new();
        enumerator.set_enumerator_type(DeviceEnumerationType::All);
        for device in enumerator.iter() {
            device.set_base_path("/tmp/devmaster");
            test_device_one(device);
        }
    }

    /// test whether Device::from_mode_and_devnum can create Device instance normally
    #[test]
    fn test_from_mode_and_devnum() {
        #[inline]
        fn inner_test(dev: &mut Device) -> Result<(), Error> {
            let devnum = dev.get_devnum()?;
            let mode = S_IFBLK;
            let new_dev = Device::from_mode_and_devnum(mode, devnum)?;

            assert_eq!(dev.get_syspath()?, new_dev.get_syspath()?);
            assert_eq!(dev.get_devpath()?, new_dev.get_devpath()?);
            assert_eq!(dev.get_devname()?, new_dev.get_devname()?);
            assert_eq!(dev.get_sysname()?, new_dev.get_sysname()?);
            assert_eq!(dev.get_subsystem()?, new_dev.get_subsystem()?);
            assert_eq!(dev.get_devnum()?, new_dev.get_devnum()?);

            Ok(())
        }

        if let Err(e) =
            LoopDev::inner_process("/tmp/test_from_mode_and_devnum", 1024 * 10, inner_test)
        {
            assert!(e.is_errno(nix::Error::EACCES) || e.is_errno(nix::Error::EBUSY));
        }
    }

    /// test whether Device::from_devname can create Device instance normally
    #[test]
    fn test_from_devname() {
        #[inline]
        fn inner_test(dev: &mut Device) -> Result<(), Error> {
            let devname = dev.get_devname()?;
            let new_dev = Device::from_devname(&devname).unwrap();

            assert_eq!(dev.get_syspath()?, new_dev.get_syspath()?);
            assert_eq!(dev.get_devpath()?, new_dev.get_devpath()?);
            assert_eq!(dev.get_devname()?, new_dev.get_devname()?);
            assert_eq!(dev.get_sysname()?, new_dev.get_sysname()?);
            assert_eq!(dev.get_subsystem()?, new_dev.get_subsystem()?);
            assert_eq!(dev.get_devnum()?, new_dev.get_devnum()?);

            Ok(())
        }

        if let Err(e) = LoopDev::inner_process("/tmp/test_from_devname", 1024 * 10, inner_test) {
            assert!(e.is_errno(nix::Error::EACCES) || e.is_errno(nix::Error::EBUSY));
        }
    }

    /// test whether Device::set_sysattr_value can work normally
    #[test]
    fn test_set_sysattr_value() {
        #[inline]
        fn inner_test(dev: &mut Device) -> Result<(), Error> {
            dev.set_sysattr_value("uevent", Some("change"))
        }

        if let Err(e) = LoopDev::inner_process("/tmp/test_set_sysattr_value", 1024 * 10, inner_test)
        {
            assert!(e.is_errno(nix::Error::EACCES) || e.is_errno(nix::Error::EBUSY));
        }
    }

    /// test device tag iterator
    #[test]
    fn test_device_tag_iterator() {
        #[inline]
        fn inner_test(dev: &mut Device) -> Result<(), Error> {
            dev.add_tag("test_tag", true);

            let mut all_tags = HashSet::new();

            for tag in &dev.tag_iter() {
                all_tags.insert(tag.clone());
            }

            assert!(all_tags.contains("test_tag"));
            Ok(())
        }

        if let Err(e) =
            LoopDev::inner_process("/tmp/test_device_tag_iterator", 1024 * 10, inner_test)
        {
            assert!(e.is_errno(nix::Error::EACCES) || e.is_errno(nix::Error::EBUSY));
        }
    }

    /// test device property iterator
    #[test]
    fn test_device_property_iterator() {
        #[inline]
        fn inner_test(dev: &mut Device) -> Result<(), Error> {
            dev.add_property("A", "B")?;

            let dev_clone = dev.shallow_clone()?;

            let devnum = dev.get_devnum()?;
            let minor = minor(devnum);
            let major = major(devnum);
            let devpath = dev.get_devpath()?;
            let devname = dev.get_devname()?;
            let devtype = dev.get_devtype()?;

            for (k, v) in dev_clone.properties.borrow().iter() {
                match k.as_str() {
                    "SUBSYSTEM" => {
                        assert_eq!(v, "block");
                    }
                    "MINOR" => {
                        assert_eq!(v, &minor.to_string());
                    }
                    "MAJOR" => {
                        assert_eq!(v, &major.to_string());
                    }
                    "DEVPATH" => {
                        assert_eq!(v, &devpath);
                    }
                    "DEVNAME" => {
                        assert_eq!(v, &devname);
                    }
                    "DEVTYPE" => {
                        assert_eq!(v, &devtype);
                    }
                    _ => {}
                }
            }

            /* Method 'property_iter' would prepare the properties of the device object
             * by reading the uevent file in kernel device tree.
             * According to different kernel versions, the content in uevent file may be
             * different, which may introduce new properties than the following ones.
             * Thus when iterating the properties collected by method 'property_iter',
             * tolerate new properties.
             */
            for (k, v) in &dev_clone.property_iter() {
                match k.as_str() {
                    "SUBSYSTEM" => {
                        assert_eq!(v, "block");
                    }
                    "MINOR" => {
                        assert_eq!(v, &minor.to_string());
                    }
                    "MAJOR" => {
                        assert_eq!(v, &major.to_string());
                    }
                    "DEVPATH" => {
                        assert_eq!(v, &devpath);
                    }
                    "DEVNAME" => {
                        assert_eq!(v, &devname);
                    }
                    "DEVTYPE" => {
                        assert_eq!(v, &devtype);
                    }
                    _ => {}
                }
            }

            Ok(())
        }

        if let Err(e) =
            LoopDev::inner_process("/tmp/test_device_property_iterator", 1024 * 10, inner_test)
        {
            assert!(e.is_errno(nix::Error::EACCES) || e.is_errno(nix::Error::EBUSY));
        }
    }

    #[test]
    fn test_update_db() {
        #[inline]
        fn inner_test(dev: &mut Device) -> Result<(), Error> {
            dev.add_devlinks("test1 test2")?;
            dev.add_tags("tag1:tag2", true);
            dev.add_property("key", "value")?;
            dev.set_devlink_priority(10);
            dev.set_usec_initialized(1000);

            dev.update_db()?;

            let db_path = format!("/tmp/devmaster/{}/{}", DB_BASE_DIR, dev.get_device_id()?);

            unlink(db_path.as_str()).unwrap();

            Ok(())
        }

        if let Err(e) = LoopDev::inner_process("/tmp/test_update_db", 1024 * 10, inner_test) {
            assert!(e.is_errno(nix::Error::EACCES) || e.is_errno(nix::Error::EBUSY));
        }
    }

    #[test]
    fn test_update_tag() {
        #[inline]
        fn inner_test(dev: &mut Device) -> Result<(), Error> {
            dev.set_usec_initialized(1000);
            dev.add_tags("test_update_tag:test_update_tag2", true);
            dev.remove_tag("test_update_tag");
            dev.update_db()?;
            dev.update_tag("test_update_tag", true)?;
            dev.update_tag("test_update_tag2", true)?;
            let tag_path = format!(
                "/tmp/devmaster/tags/test_update_tag/{}",
                dev.get_device_id()?
            );
            assert!(Path::new(tag_path.as_str()).exists());

            dev.update_tag("test_update_tag", false)?;
            assert!(!Path::new(tag_path.as_str()).exists());

            let _ = dev.get_usec_initialized().unwrap();

            assert!(dev.has_tag("test_update_tag").unwrap());
            assert!(dev.has_tag("test_update_tag2").unwrap());
            assert!(!dev.has_current_tag("test_update_tag").unwrap());
            assert!(dev.has_current_tag("test_update_tag2").unwrap());

            dev.cleanup_tags();

            fs::remove_dir_all("/tmp/devmaster").unwrap();

            Ok(())
        }

        if let Err(e) = LoopDev::inner_process("/tmp/test_update_tag", 1024 * 10, inner_test) {
            assert!(
                e.is_errno(nix::Error::EACCES)
                    || e.is_errno(nix::Error::EBUSY)
                    || e.is_errno(nix::Error::EAGAIN)
            );
        }
    }

    #[test]
    fn test_read_all_sysattrs() {
        #[inline]
        fn inner_test(dev: &mut Device) -> Result<(), Error> {
            dev.read_all_sysattrs().unwrap();

            for sysattr in &dev.sysattr_iter() {
                if let Err(e) = dev.get_sysattr_value(sysattr) {
                    assert!(!IN_SET!(e.get_errno(), Errno::EPERM, Errno::EINVAL));
                }
            }

            Ok(())
        }

        if let Err(e) = LoopDev::inner_process("/tmp/test_read_all_sysattrs", 1024 * 10, inner_test)
        {
            assert!(e.is_errno(nix::Error::EACCES) || e.is_errno(nix::Error::EBUSY));
        }
    }

    #[test]
    fn test_enumerate_children() {
        #[inline]
        fn inner_test(dev: &mut Device) -> Result<(), Error> {
            dev.enumerate_children().unwrap();

            for _ in &dev.child_iter() {}

            Ok(())
        }

        if let Err(e) =
            LoopDev::inner_process("/tmp/test_enumerate_children", 1024 * 10, inner_test)
        {
            assert!(e.is_errno(nix::Error::EACCES) || e.is_errno(nix::Error::EBUSY));
        }
    }

    #[test]
    fn test_shallow_clone() {
        /* Enumerator merely collect devices with valid subsystems,
         * while get_parent method may not, e.g., /sys/devices/platform.
         */
        let mut e = DeviceEnumerator::new();
        e.set_enumerator_type(DeviceEnumerationType::All);

        for mut dev in e.iter() {
            let dev_clone = dev.shallow_clone().unwrap();
            dev_clone.get_syspath().unwrap();
            dev_clone.get_subsystem().unwrap();

            loop {
                let ret = dev.get_parent();

                if let Ok(parent) = ret {
                    parent.get_syspath().unwrap();
                    if let Err(e) = parent.get_subsystem() {
                        assert_eq!(e.get_errno(), Errno::ENOENT);
                    }
                    dev = parent;
                    continue;
                }

                break;
            }
        }
    }

    #[test]
    fn test_add_devlink() {
        #[inline]
        fn inner_test(dev: &mut Device) -> Result<(), Error> {
            let s1 = dev.get_syspath().unwrap();

            let dev_clone = dev.shallow_clone().unwrap();

            assert_eq!(s1, dev_clone.get_syspath().unwrap());

            Ok(())
        }

        if let Err(e) = LoopDev::inner_process("/tmp/test_add_devlink", 1024 * 10, inner_test) {
            assert!(e.is_errno(nix::Error::EACCES) || e.is_errno(nix::Error::EBUSY));
        }
    }

    #[test]
    fn test_from() {
        #[inline]
        fn inner_test(dev: &mut Device) -> Result<(), Error> {
            let syspath = dev.get_syspath().unwrap();
            let devnum = dev.get_devnum().unwrap();
            let id = dev.get_device_id().unwrap();
            let devname = dev.get_devname().unwrap();

            dev.set_action_from_string("change").unwrap();
            dev.set_seqnum_from_string("1000").unwrap();

            let (nulstr, _) = dev.get_properties_nulstr().unwrap();

            let dev_new = Device::from_syspath(&syspath, true).unwrap();
            assert_eq!(dev, &dev_new);

            let dev_new = Device::from_device_id(&id).unwrap();
            assert_eq!(dev, &dev_new);

            let dev_new = Device::from_nulstr(nulstr.as_slice()).unwrap();
            assert_eq!(dev, &dev_new);

            let dev_new = Device::from_devnum('b', devnum).unwrap();
            assert_eq!(dev, &dev_new);

            let dev_new = Device::from_devname(&devname).unwrap();
            assert_eq!(dev, &dev_new);

            let dev_new_1 = Device::from_path(&syspath).unwrap();
            let dev_new_2 = Device::from_path(&devname).unwrap();
            assert_eq!(dev_new_1, dev_new_2);

            assert_eq!(
                Device::from_devname(&syspath).unwrap_err().get_errno(),
                Errno::EINVAL
            );

            Ok(())
        }

        if let Err(e) = LoopDev::inner_process("/tmp/test_from", 1024 * 10, inner_test) {
            assert!(e.is_errno(nix::Error::EACCES) || e.is_errno(nix::Error::EBUSY));
        }
    }

    #[test]
    fn test_set_syspath_error() {
        let device = Device::new();

        assert!(device.set_syspath("", true).is_err());
        assert!(device.set_syspath(".././///../.", true).is_err());
        assert!(device.set_syspath("/not/exist", true).is_err());
        assert!(device.set_syspath("/dev/hello", true).is_err());
        assert!(device.set_syspath("/sys/devices/none", true).is_err());
        assert!(device.set_syspath("/sys/none", true).is_err());
        assert_eq!(
            device.set_syspath("/sys/", true).unwrap_err().get_errno(),
            nix::Error::ENODEV
        );

        assert_eq!(
            device
                .set_syspath("/dev/hello", false)
                .unwrap_err()
                .get_errno(),
            nix::Error::EINVAL
        );
        assert!(device.set_syspath("/sys/", false).is_ok());
        assert!(device.set_syspath("/sys", false).is_err());
    }

    #[test]
    fn test_from_ifindex_error() {
        assert!(Device::from_ifindex(10000).is_err());
    }

    #[test]
    fn test_set_seqnum_from_string() {
        let device = Device::new();
        device.set_seqnum_from_string("1000").unwrap();

        assert!(device.set_seqnum_from_string("xxxx").is_err());
    }

    #[test]
    fn test_set_db_persist() {
        let device = Device::new();
        device.set_db_persist();
    }

    #[test]
    fn test_from_db() {
        /* Legal db content. */
        {
            let content = "S:disk/by-path/pci-0000:00:10.0-scsi-0:0:0:0-part1
I:1698916066
E:ID_PART_ENTRY_OFFSET=2048
G:devmaster
Q:devmaster
V:100
";
            touch_file("/tmp/tmp_db", false, Some(0o777), None, None).unwrap();
            let mut f = OpenOptions::new().write(true).open("/tmp/tmp_db").unwrap();
            f.write_all(content.as_bytes()).unwrap();
            let device = Device::new();
            device.read_db_internal_filename("/tmp/tmp_db").unwrap();
        }

        /* Strange db entry would be ignored. */
        {
            let content = "error
";
            let mut f = OpenOptions::new().write(true).open("/tmp/tmp_db").unwrap();
            f.write_all(content.as_bytes()).unwrap();
            let device = Device::new();
            device.read_db_internal_filename("/tmp/tmp_db").unwrap();
        }

        /* Illegal db entry value would throw error. */
        {
            let content = "I:invalid
";
            let mut f = OpenOptions::new().write(true).open("/tmp/tmp_db").unwrap();
            f.write_all(content.as_bytes()).unwrap();
            let device = Device::new();
            assert!(device.read_db_internal_filename("/tmp/tmp_db").is_err());
        }

        /* DB should be readable. */
        {
            touch_file("/tmp/tmp_db_writeonly", false, Some(0o222), None, None).unwrap();
            let device = Device::new();
            assert!(device.read_db_internal_filename("/tmp/tmp_db").is_err());
        }

        /* Test different kinds of illegal db entry. */
        {
            let device = Device::new();
            assert!(device
                .amend_key_value("USEC_INITIALIZED", "invalid")
                .is_err());
            assert!(device.handle_db_line("E", "ID_TEST==invalid").is_err());
            assert!(device.handle_db_line("E", "=invalid").is_err());
            assert!(device.handle_db_line("I", "invalid").is_err());
            assert!(device.handle_db_line("L", "invalid").is_err());
            assert!(device.handle_db_line("W", "").is_ok());
            assert!(device.handle_db_line("V", "invalid").is_err());
        }

        unlink("/tmp/tmp_db").unwrap();
        unlink("/tmp/tmp_db_writeonly").unwrap();
    }

    #[test]
    fn test_set_is_initialized() {
        let device = Device::from_subsystem_sysname("net", "lo").unwrap();
        device.set_is_initialized();
        if device
            .trigger_with_uuid(DeviceAction::Change, false)
            .is_ok()
        {
            device
                .trigger_with_uuid(DeviceAction::Change, true)
                .unwrap();
            device.trigger(DeviceAction::Change).unwrap();
        }
    }

    #[test]
    fn test_get_usec_since_initialized() {
        assert!(catch_unwind(|| {
            let dev = Device::new();
            dev.get_usec_since_initialized().unwrap();
        })
        .is_err());
    }

    #[test]
    fn test_set() {
        let device = Device::from_subsystem_sysname("net", "lo").unwrap();
        device.set_devuid("1").unwrap();
        device.set_devgid("1").unwrap();
        device.set_devmode("666").unwrap();
        device.set_diskseq("1").unwrap();
        device.set_action_from_string("change").unwrap();

        if device.set_sysattr_value("ifalias", Some("test")).is_ok() {
            assert_eq!(&device.get_cached_sysattr_value("ifalias").unwrap(), "test");
        }

        assert_eq!(&device.get_property_value("DEVUID").unwrap(), "1");
        assert_eq!(&device.get_property_value("DEVGID").unwrap(), "1");
        assert_eq!(&device.get_property_value("DEVMODE").unwrap(), "666");
        assert_eq!(&device.get_property_value("DISKSEQ").unwrap(), "1");
        assert_eq!(&device.get_property_value("ACTION").unwrap(), "change");

        assert!(device.set_devuid("invalid").is_err());
        assert!(device.set_devgid("invalid").is_err());
        assert!(device.set_devmode("invalid").is_err());
        assert!(device.set_diskseq("invalid").is_err());
        assert!(device.set_action_from_string("invalid").is_err());
        assert!(device.set_sysattr_value("nonexist", Some("test")).is_err());

        assert!(device.set_sysattr_value("nonexist", None).is_ok());
        assert!(device.set_sysattr_value("ifalias", None).is_ok());
    }

    #[test]
    fn test_from_device_id() {
        assert!(Device::from_device_id("invalid").is_err());
        assert!(Device::from_device_id("b").is_err());
        assert!(Device::from_device_id("+drivers").is_err());
        assert!(Device::from_device_id("+drivers:").is_err());
        assert!(Device::from_device_id("+drivers::usb").is_err());

        let dev = Device::from_device_id("+drivers:usb:usb").unwrap();
        println!("{}", dev.get_device_id().unwrap());

        let dev = Device::from_syspath("/sys/bus/usb/drivers/usb", true).unwrap();
        println!("{}", dev.get_device_id().unwrap());

        let _ = unlink("/tmp/devmaster/data/+drivers:usb:usb");
        dev.set_base_path("/tmp/devmaster");
        assert!(!dev.update_db().is_err());
        dev.add_property("hello", "world").unwrap();
        dev.update_db().unwrap();
        assert!(Path::new("/tmp/devmaster/data/+drivers:usb:usb").exists());
    }

    #[test]
    fn test_get_err() {
        let device = Device::new();
        assert!(device.get_syspath().is_err());
        assert!(device.get_devpath().is_err());
        assert!(device.get_parent().is_err());
        assert!(device.get_devtype().is_err());
        assert!(!device.get_is_initialized().unwrap());
    }

    #[test]
    fn test_cleanup() {
        let _ = touch_file("/tmp/devmaster/a", false, None, None, None);
        let _ = touch_file("/tmp/devmaster/b", false, None, None, None);
        Device::cleanup("/tmp/devmaster/a", "/tmp/devmaster/b");
    }

    #[test]
    fn test_fmt() {
        let device = Device::from_subsystem_sysname("net", "lo").unwrap();
        println!("{:?}", device);
    }

    #[test]
    fn test_set_syspath_no_verify() {
        let device = Device::new();
        device.set_syspath("/sys/test", false).unwrap();

        assert!(device.set_sysname_and_sysnum().is_ok());
    }

    #[test]
    fn test_partial_eq_trait() {
        let dev1 = Device::from_syspath("/sys/class/net/lo", true).unwrap();
        let dev2 = Device::from_subsystem_sysname("net", "lo").unwrap();

        assert!(dev1 == dev2);
    }

    #[test]
    fn test_from_devnum_err() {
        assert!(Device::from_devnum('x', 100).is_err());
    }

    #[test]
    fn test_match_sysattr() {
        let mut ert = DeviceEnumerator::new();
        let dev = Device::from_subsystem_sysname("net", "lo").unwrap();

        ert.add_match_sysattr("ifindex", "1", true).unwrap();
        ert.add_match_sysattr("address", "aa:aa:aa:aa:aa:aa", false)
            .unwrap();

        assert!(dev.match_sysattr(&ert.match_sysattr.borrow(), &ert.not_match_sysattr.borrow()));

        assert!(dev.match_sysattr(&ert.match_sysattr.borrow(), &HashMap::new()));

        let mut nomatch_sysattr = HashMap::new();
        nomatch_sysattr.insert("ifindex".to_string(), "1".to_string());
        assert!(!dev.match_sysattr(&HashMap::new(), &nomatch_sysattr));

        assert!(dev.match_sysattr(&HashMap::new(), &HashMap::new()));
    }

    #[test]
    fn test_match_parent() {
        let mut ert = DeviceEnumerator::new();
        let dev = Device::from_subsystem_sysname("net", "lo").unwrap();
        ert.add_match_parent(&dev).unwrap();
        assert!(dev.match_parent(&ert.match_parent.borrow(), &HashSet::new()));

        let mut nomatch_parent = HashSet::new();
        nomatch_parent.insert("/sys/devices/virtual/net/lo".to_string());
        assert!(!dev.match_parent(&HashSet::new(), &nomatch_parent));
    }

    #[test]
    fn test_from_environment() {
        /* When generating device object from nulstr or environment properties,
         * the following four properties are required:
         *
         * SUBSYSTEM, DEVPATH, SEQNUM, ACTION
         */
        std::env::set_var("SUBSYSTEM", "net");
        let _ = Device::from_environment().unwrap_err();
        std::env::set_var("DEVPATH", "/devices/virtual/net/lo");
        let _ = Device::from_environment().unwrap_err();
        std::env::set_var("SEQNUM", "100");
        let _ = Device::from_environment().unwrap_err();
        std::env::set_var("ACTION", "add");
        let _ = Device::from_environment().unwrap();
    }
}
