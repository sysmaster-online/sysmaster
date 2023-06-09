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

//! struct Device
//!
use crate::err_wrapper;
use crate::utils::readlink_value;
use crate::{error::*, DeviceAction};
use basic::fs_util::{open_temporary, touch_file};
use basic::parse_util::{device_path_parse_devnum, parse_devnum, parse_ifindex};
use libc::{dev_t, gid_t, mode_t, uid_t, S_IFBLK, S_IFCHR, S_IFDIR, S_IFLNK, S_IFMT, S_IRUSR};
use nix::errno::{self, Errno};
use nix::fcntl::{open, OFlag};
use nix::sys::stat::{self, fchmod, lstat, major, makedev, minor, stat, Mode};
use nix::unistd::{unlink, Gid, Uid};
use std::cell::{Ref, RefCell};
use std::collections::hash_set::Iter;
use std::collections::{HashMap, HashSet};
use std::fs::{self, rename, OpenOptions};
use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use std::os::unix::prelude::FromRawFd;
use std::path::Path;
use std::rc::Rc;
use std::result::Result;

/// database directory path
pub const DB_BASE_DIR: &str = "/run/devmaster/data/";
/// tags directory path
pub const TAGS_BASE_DIR: &str = "/run/devmaster/tags/";

/// Device
#[derive(Debug, Clone)]
pub struct Device {
    /// inotify handler
    pub watch_handle: RefCell<i32>,
    /// the parent device
    pub parent: RefCell<Option<Rc<RefCell<Device>>>>,
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

    /// whether the device object is initialized
    pub is_initialized: RefCell<bool>,
    /// don not read more information from uevent/db
    pub sealed: RefCell<bool>,
    /// persist device db during switching root from initrd
    pub db_persist: RefCell<bool>,
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
        }
    }

    /// create Device from buffer
    pub fn from_nulstr(nulstr: &[u8]) -> Result<Device, Error> {
        let device = Device::new();
        let s = std::str::from_utf8(nulstr).unwrap();
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

        Ok(device)
    }

    /// create a Device instance from devname
    /// devname is the device path under /dev
    /// e.g. /dev/block/8:0
    /// e.g. /dev/char/7:0
    /// e.g. /dev/sda
    pub fn from_devname(devname: &str) -> Result<Device, Error> {
        if !devname.starts_with("/dev") {
            return Err(Error::Nix {
                msg: format!("from_devname failed: devname '{devname}' doesn't start with /dev"),
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
                        msg: format!("from_devname failed: cannot stat '{devname}'"),
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

    /// create a Device instance from path
    /// path falls into two kinds: devname (/dev/...) and syspath (/sys/devices/...)
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

        let ifname = String::from_utf8(buf_trans.to_vec()).map_err(|e| Error::Nix {
            msg: format!("from_ifindex failed: from_utf8 {:?} ({})", buf_trans, e),
            source: Errno::EINVAL,
        })?;

        let syspath = format!("/sys/class/net/{}", ifname.trim_matches(char::from(0)));
        let dev = Self::from_syspath(&syspath, true).map_err(|e| Error::Nix {
            msg: format!("from_ifindex failed: {}", e),
            source: e.get_errno(),
        })?;

        let i = match dev.get_ifindex() {
            Ok(i) => i,
            Err(e) => {
                if e.get_errno() == Errno::ENOENT {
                    return Err(Error::Nix {
                        msg: format!("from_ifindex failed: {}", e),
                        source: Errno::ENXIO,
                    });
                }

                return Err(Error::Nix {
                    msg: format!("from_ifindex failed: {}", e),
                    source: e.get_errno(),
                });
            }
        };

        if i != ifindex {
            return Err(Error::Nix {
                msg: "from_ifindex failed: ifindex inconsistent".to_string(),
                source: Errno::ENXIO,
            });
        }

        Ok(dev)
    }

    /// create a Device instance from subsystem and sysname
    /// if subsystem is 'drivers', sysname should be like 'xxx:yyy'
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

    /// set sysattr value
    pub fn set_sysattr_value(&self, sysattr: &str, value: Option<&str>) -> Result<(), Error> {
        if value.is_none() {
            self.remove_cached_sysattr_value(sysattr)?;
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
            self.remove_cached_sysattr_value(sysattr)?;
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
                let devnum = parse_devnum(&id[1..]).map_err(|_| Error::Nix {
                    msg: format!("from_device_id failed: parse_devnum '{}' failed", id),
                    source: Errno::EINVAL,
                })?;

                return Device::from_devnum(id.chars().next().unwrap(), devnum).map_err(|e| {
                    Error::Nix {
                        msg: format!("from_device_id failed: {}", e),
                        source: e.get_errno(),
                    }
                });
            }
            Some('n') => {
                let ifindex = parse_ifindex(&id[1..]).map_err(|_| Error::Nix {
                    msg: format!("from_device_id failed: parse_ifindex '{}' failed", id),
                    source: Errno::EINVAL,
                })?;

                Device::from_ifindex(ifindex).map_err(|e| Error::Nix {
                    msg: format!("from_device_id failed: {}", e),
                    source: e.get_errno(),
                })
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
                Device::from_subsystem_sysname(&subsystem, &sysname).map_err(|e| Error::Nix {
                    msg: format!("from_device_id failed: {}", e),
                    source: e.get_errno(),
                })
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
            err_wrapper!(self.set_sysname_and_sysnum(), "get_sysname")?;
        }

        Ok(self.sysname.borrow().clone())
    }

    /// get the parent of the device
    pub fn get_parent(&self) -> Result<Rc<RefCell<Device>>, Error> {
        if !*self.parent_set.borrow() {
            match Device::new_from_child(self) {
                Ok(parent) => {
                    let _ = self.parent.replace(Some(Rc::new(RefCell::new(parent))));
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
    ) -> Result<Rc<RefCell<Device>>, Error> {
        let mut parent = match self.get_parent() {
            Ok(parent) => parent,
            Err(e) => return Err(e),
        };

        loop {
            let parent_subsystem = parent.borrow_mut().get_subsystem();

            if parent_subsystem.is_ok() && parent_subsystem.unwrap() == subsystem {
                if devtype.is_none() {
                    break;
                }

                let parent_devtype = parent.borrow_mut().get_devtype();
                if parent_devtype.is_ok() && parent_devtype.unwrap() == devtype.unwrap() {
                    break;
                }
            }

            let tmp = parent.borrow_mut().get_parent()?;
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
                readlink_value(subsystem_path).map_err(|e| Error::Nix {
                    msg: format!("get_subsystem failed: {}", e),
                    source: e.get_errno(),
                })?
            } else {
                "".to_string()
            };

            if !filename.is_empty() {
                self.set_subsystem(&filename)?;
            } else if self.devpath.borrow().starts_with("/module/") {
                self.set_subsystem("module")?;
            } else if self.devpath.borrow().contains("/drivers/")
                || self.devpath.borrow().contains("/drivers")
            {
                self.set_drivers_subsystem()?;
            } else if self.devpath.borrow().starts_with("/class/")
                || self.devpath.borrow().starts_with("/bus/")
            {
                self.set_subsystem("subsystem")?;
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
        self.read_uevent_file().map_err(|e| Error::Nix {
            msg: format!("get_ifindex failed: {}", e),
            source: e.get_errno(),
        })?;

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
        match self.read_uevent_file() {
            Ok(_) => {}
            Err(e) => {
                return Err(e);
            }
        }

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
            self.set_driver(&driver).map_err(|e| Error::Nix {
                msg: format!("get_driver failed: {}", e),
                source: e.get_errno(),
            })?;
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
        match self.read_uevent_file() {
            Ok(_) => {}
            Err(e) => {
                return Err(e);
            }
        }

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
            self.set_sysname_and_sysnum().map_err(|e| Error::Nix {
                msg: format!("get_sysnum failed: {}", e),
                source: e.get_errno(),
            })?;
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
        self.read_uevent_file().map_err(|e| Error::Nix {
            msg: format!("get_diskseq failed: {}", e),
            source: e.get_errno(),
        })?;

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
        // match self.read_db
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
        self.read_db().map_err(|e| Error::Nix {
            msg: format!("has_tag failed: {}", e),
            source: e.get_errno(),
        })?;

        Ok(self.all_tags.borrow().contains(tag))
    }

    /// check whether the device has the current tag
    pub fn has_current_tag(&self, tag: &str) -> Result<bool, Error> {
        self.read_db().map_err(|e| Error::Nix {
            msg: format!("has_tag failed: {}", e),
            source: e.get_errno(),
        })?;

        Ok(self.current_tags.borrow().contains(tag))
    }

    /// get the value of specific device property
    pub fn get_property_value(&self, key: &str) -> Result<String, Error> {
        self.properties_prepare().map_err(|e| Error::Nix {
            msg: format!("get_property_value failed: {}", e),
            source: e.get_errno(),
        })?;

        match self.properties.borrow().get(key) {
            Some(v) => Ok(v.clone()),
            None => Err(Error::Nix {
                msg: format!("get_property_value failed: no key '{}'", key),
                source: nix::errno::Errno::ENOENT,
            }),
        }
    }

    /// get the trigger uuid of the device
    pub fn get_trigger_uuid(&self) -> Result<[u8; 8], Error> {
        todo!()
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
                        .open(sysattr_path.clone())
                        .map_err(|e| Error::Nix {
                            msg: format!(
                                "get_sysattr_value failed: can't open sysattr '{}': {}",
                                sysattr_path, e
                            ),
                            source: Errno::from_i32(e.raw_os_error().unwrap_or_default()),
                        })?;
                    let mut value = String::new();
                    file.read_to_string(&mut value).map_err(|e| Error::Nix {
                        msg: format!(
                            "get_sysattr_value failed: can't read sysattr '{}': {}",
                            sysattr_path, e
                        ),
                        source: Errno::from_i32(e.raw_os_error().unwrap_or_default()),
                    })?;
                    value.trim_end().to_string()
                }
            }

            Err(e) => {
                self.remove_cached_sysattr_value(sysattr).unwrap();
                return Err(Error::Nix {
                    msg: format!("get_sysattr_value failed: can't lstat '{}'", sysattr_path),
                    source: e,
                });
            }
        };

        self.cache_sysattr_value(sysattr, &value)
            .map_err(|e| Error::Nix {
                msg: format!("get_sysattr_value failed: {}", e),
                source: e.get_errno(),
            })?;

        Ok(value)
    }

    /// trigger with uuid
    pub fn trigger_with_uuid(&self, _action: DeviceAction) -> Result<[u8; 8], Error> {
        todo!()
    }

    /// open device
    pub fn open(&self, oflags: OFlag) -> Result<File, Error> {
        let devname = self.get_devname().map_err(|e| {
            if e.get_errno() == Errno::ENOENT {
                Error::Nix {
                    msg: format!("open failed: {}", e),
                    source: Errno::ENOEXEC,
                }
            } else {
                Error::Nix {
                    msg: format!("open failed: {}", e),
                    source: e.get_errno(),
                }
            }
        })?;

        let devnum = self.get_devnum().map_err(|e| {
            if e.get_errno() == Errno::ENOENT {
                Error::Nix {
                    msg: format!("open failed: {}", e),
                    source: Errno::ENOEXEC,
                }
            } else {
                Error::Nix {
                    msg: format!("open failed: {}", e),
                    source: e.get_errno(),
                }
            }
        })?;

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

        let stat = match nix::sys::stat::fstat(file.as_raw_fd()) {
            Ok(s) => s,
            Err(e) => {
                return Err(Error::Nix {
                    msg: format!(
                        "open failed: can't fstat fd {} for '{}'",
                        file.as_raw_fd(),
                        devname
                    ),
                    source: e,
                })
            }
        };

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

        if self.get_is_initialized().map_err(|e| Error::Nix {
            msg: format!("open failed: {}", e),
            source: e.get_errno(),
        })? {
            match self.get_property_value("ID_IGNORE_DISKSEQ") {
                Ok(value) => {
                    if !value.parse::<bool>().map_err(|e| Error::Nix {
                        msg: format!(
                            "open failed: failed to parse value '{}' to boolean: {}",
                            value, e
                        ),
                        source: Errno::EINVAL,
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

        let file2 =
            basic::fd_util::fd_reopen(file.as_raw_fd(), oflags).map_err(|e| Error::Nix {
                msg: format!("open failed: {}", e),
                source: match e {
                    basic::Error::Nix { source } => source,
                    _ => Errno::EINVAL,
                },
            })?;

        if diskseq == 0 {
            return Ok(file2);
        }

        let q = basic::fd_util::fd_get_diskseq(file2.as_raw_fd()).map_err(|e| Error::Nix {
            msg: format!(
                "open failed: failed to get diskseq on fd {}",
                file2.as_raw_fd()
            ),
            source: match e {
                basic::Error::Nix { source } => source,
                _ => Errno::EINVAL,
            },
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
        let device = self.shallow_clone().map_err(|e| Error::Nix {
            msg: format!("clone_with_db failed: {}", e),
            source: e.get_errno(),
        })?;

        device.read_db().map_err(|e| Error::Nix {
            msg: format!("clone_with_db failed: {}", e),
            source: e.get_errno(),
        })?;

        device.sealed.replace(true);

        Ok(device)
    }

    /// add tag to the device object
    pub fn add_tag(&self, tag: &str, both: bool) -> Result<(), Error> {
        self.all_tags.borrow_mut().insert(tag.to_string());

        if both {
            self.current_tags.borrow_mut().insert(tag.to_string());
        }
        self.property_tags_outdated.replace(true);
        Ok(())
    }

    /// add a set of tags, separated by ':'
    pub fn add_tags(&self, tags: &str, both: bool) -> Result<(), Error> {
        for tag in tags.split(':') {
            self.add_tag(tag, both)?;
        }

        Ok(())
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

    /// get the device id
    /// device id is used to identify database file in /run/devmaster/data/
    pub fn get_device_id(&self) -> Result<String, Error> {
        if self.device_id.borrow().is_empty() {
            let subsystem = self.get_subsystem().map_err(|e| Error::Nix {
                msg: format!("get_device_id failed: {}", e),
                source: e.get_errno(),
            })?;

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
                let sysname = self.get_sysname().map_err(|e| Error::Nix {
                    msg: format!("get_device_id failed: {}", e),
                    source: e.get_errno(),
                })?;

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
    pub fn set_usec_initialized(&self, time: u64) -> Result<(), Error> {
        self.add_property_internal("USEC_INITIALIZED", &time.to_string())?;
        self.usec_initialized.replace(time);
        Ok(())
    }

    /// update device database
    pub fn update_db(&self) -> Result<(), Error> {
        #[inline]
        fn cleanup(db: &str, tmp_file: &str) {
            let _ = unlink(db);
            let _ = unlink(tmp_file);
        }

        let has_info = self.has_info();

        let id = self.get_device_id()?;

        let db_path = format!("{}{}", DB_BASE_DIR, id);

        if !has_info && *self.devnum.borrow() == 0 && *self.ifindex.borrow() == 0 {
            unlink(db_path.as_str()).map_err(|e| Error::Nix {
                msg: format!("update_db failed: can't unlink db '{}'", db_path),
                source: e,
            })?;

            return Ok(());
        }

        create_dir_all(DB_BASE_DIR).map_err(|e| Error::Nix {
            msg: "update_db failed: can't create db directory".to_string(),
            source: e
                .raw_os_error()
                .map_or_else(|| nix::Error::EIO, nix::Error::from_i32),
        })?;

        let (mut file, tmp_file) = open_temporary(&db_path).map_err(|e| {
            let errno = match e {
                basic::error::Error::Nix { source } => source,
                _ => nix::Error::EINVAL,
            };
            Error::Nix {
                msg: "update_db failed: can't open temporary file".to_string(),
                source: errno,
            }
        })?;

        fchmod(
            file.as_raw_fd(),
            if *self.db_persist.borrow() {
                Mode::from_bits(0o1644).unwrap()
            } else {
                Mode::from_bits(0o644).unwrap()
            },
        )
        .map_err(|e| {
            cleanup(&db_path, &tmp_file);
            Error::Nix {
                msg: "update_db failed: can't change the mode of temporary file".to_string(),
                source: e,
            }
        })?;

        if has_info {
            if *self.devnum.borrow() > 0 {
                for link in self.devlinks.borrow().iter() {
                    file.write(format!("S:{}\n", link.strip_prefix("/dev/").unwrap()).as_bytes())
                        .map_err(|e| {
                            cleanup(&db_path, &tmp_file);
                            Error::Nix {
                                msg: format!(
                                    "update_db failed: can't write devlink '{}' to db",
                                    link
                                ),
                                source: e
                                    .raw_os_error()
                                    .map(nix::Error::from_i32)
                                    .unwrap_or(nix::Error::EIO),
                            }
                        })?;
                }

                if *self.devlink_priority.borrow() != 0 {
                    file.write(format!("L:{}\n", self.devlink_priority.borrow()).as_bytes())
                        .map_err(|e| {
                            cleanup(&db_path, &tmp_file);
                            Error::Nix {
                                msg: format!(
                                    "update_db failed: can't write devlink priority '{}' to db",
                                    *self.devlink_priority.borrow()
                                ),
                                source: e
                                    .raw_os_error()
                                    .map(nix::Error::from_i32)
                                    .unwrap_or(nix::Error::EIO),
                            }
                        })?;
                }
            }

            if *self.usec_initialized.borrow() > 0 {
                file.write(format!("I:{}\n", self.usec_initialized.borrow()).as_bytes())
                    .map_err(|e| {
                        cleanup(&db_path, &tmp_file);
                        Error::Nix {
                            msg: format!(
                                "update_db failed: can't write initial usec '{}' to db",
                                *self.usec_initialized.borrow()
                            ),
                            source: e
                                .raw_os_error()
                                .map(nix::Error::from_i32)
                                .unwrap_or(nix::Error::EIO),
                        }
                    })?;
            }

            for (k, v) in self.properties_db.borrow().iter() {
                file.write(format!("E:{}={}\n", k, v).as_bytes())
                    .map_err(|e| {
                        cleanup(&db_path, &tmp_file);
                        Error::Nix {
                            msg: format!(
                                "update_db failed: can't write property '{}'='{}' to db",
                                k, v
                            ),
                            source: e
                                .raw_os_error()
                                .map(nix::Error::from_i32)
                                .unwrap_or(nix::Error::EIO),
                        }
                    })?;
            }

            for tag in self.all_tags.borrow().iter() {
                file.write(format!("G:{}\n", tag).as_bytes()).map_err(|e| {
                    cleanup(&db_path, &tmp_file);
                    Error::Nix {
                        msg: "update_db failed: can't write tag '{}' to db".to_string(),
                        source: e
                            .raw_os_error()
                            .map(nix::Error::from_i32)
                            .unwrap_or(nix::Error::EIO),
                    }
                })?;
            }

            for tag in self.current_tags.borrow().iter() {
                file.write(format!("Q:{}\n", tag).as_bytes()).map_err(|e| {
                    cleanup(&db_path, &tmp_file);
                    Error::Nix {
                        msg: format!(
                            "update_db failed: failed to write current tag '{}' to db",
                            tag
                        ),
                        source: e
                            .raw_os_error()
                            .map(nix::Error::from_i32)
                            .unwrap_or(nix::Error::EIO),
                    }
                })?;
            }
        }

        file.flush().map_err(|e| {
            cleanup(&db_path, &tmp_file);
            Error::Nix {
                msg: "update_db failed: can't flush db".to_string(),
                source: e
                    .raw_os_error()
                    .map(nix::Error::from_i32)
                    .unwrap_or(nix::Error::EIO),
            }
        })?;

        rename(&tmp_file, &db_path).map_err(|e| {
            cleanup(&db_path, &tmp_file);
            Error::Nix {
                msg: "update_db failed: can't rename temporary file".to_string(),
                source: e
                    .raw_os_error()
                    .map(nix::Error::from_i32)
                    .unwrap_or(nix::Error::EIO),
            }
        })?;

        Ok(())
    }

    /// update persist device tag
    pub fn update_tag(&self, tag: &str, add: bool) -> Result<(), Error> {
        let id = self.get_device_id()?;

        let tag_path = format!("{}{}/{}", TAGS_BASE_DIR, tag, id);

        if add {
            touch_file(&tag_path, true, Some(0o444), None, None).map_err(|e| Error::Nix {
                msg: format!("tag_persist failed: can't touch file '{}': {}", tag_path, e),
                source: nix::Error::EINVAL,
            })?;

            return Ok(());
        }

        match unlink(tag_path.as_str()) {
            Ok(_) => {}
            Err(e) => {
                if e != nix::Error::ENOENT {
                    return Err(Error::Nix {
                        msg: "update_tag failed: can't unlink db".to_string(),
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
        self.read_db_internal(false).map_err(|e| Error::Nix {
            msg: format!("read_db failed: {}", e),
            source: e.get_errno(),
        })
    }

    /// read database internally
    pub fn read_db_internal(&self, force: bool) -> Result<(), Error> {
        if *self.db_loaded.borrow() || (!force && *self.sealed.borrow()) {
            return Ok(());
        }

        let id = self.get_device_id().map_err(|e| Error::Nix {
            msg: format!("read_db_internal failed: {}", e),
            source: e.get_errno(),
        })?;

        let path = format!("{}{}", DB_BASE_DIR, id);

        self.read_db_internal_filename(&path)
            .map_err(|e| Error::Nix {
                msg: format!("read_db_internal failed: {}", e),
                source: e.get_errno(),
            })
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
        let subsystem_ret = device.get_subsystem().map_err(|e| Error::Nix {
            msg: format!("from_mode_and_devnum failed: {}", e),
            source: e.get_errno(),
        })?;
        if (subsystem_ret == "block") != ((mode & S_IFMT) == S_IFBLK) {
            return Err(Error::Nix {
                msg: "from_mode_and_devnum failed: inconsistent subsystem".to_string(),
                source: Errno::EINVAL,
            });
        }

        Result::Ok(device)
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

            match path.as_os_str().to_str() {
                Some(s) => s.to_string(),
                None => {
                    return Err(Error::Nix {
                        msg: format!("set_syspath failed: '{:?}' can not change to string", path),
                        source: Errno::EINVAL,
                    });
                }
            }
        } else {
            if !path.starts_with("/sys/") {
                return Err(Error::Nix {
                    msg: format!("set_syspath failed: '{:?}' does not start with /sys", path),
                    source: Errno::EINVAL,
                });
            }

            path.to_string()
        };

        let devpath = match p.strip_prefix("/sys") {
            Some(p) => p,
            None => {
                return Err(Error::Nix {
                    msg: format!("set_syspath failed: '{}' does not start with /sys", p),
                    source: Errno::EINVAL,
                });
            }
        };

        if !devpath.starts_with('/') {
            return Err(Error::Nix {
                msg: format!(
                    "set_syspath failed: devpath '{}' alone is not a valid device path",
                    p
                ),
                source: Errno::ENODEV,
            });
        }

        match self.add_property_internal("DEVPATH", devpath) {
            Ok(_) => {}
            Err(e) => {
                return Err(Error::Nix {
                    msg: format!("set_syspath failed: {}", e),
                    source: Errno::ENODEV,
                })
            }
        }
        self.devpath.replace(devpath.to_string());
        self.syspath.replace(p);

        Ok(())
    }

    /// set the sysname and sysnum of device object
    pub fn set_sysname_and_sysnum(&self) -> Result<(), Error> {
        let sysname = match self.devpath.borrow().rfind('/') {
            Some(i) => String::from(&self.devpath.borrow()[i + 1..]),
            None => {
                return Err(Error::Nix {
                    msg: format!(
                        "set_sysname_and_sysnum failed: invalid devpath '{}'",
                        self.devpath.borrow()
                    ),
                    source: Errno::EINVAL,
                });
            }
        };

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
    pub fn set_subsystem(&self, subsystem: &str) -> Result<(), Error> {
        self.add_property_internal("SUBSYSTEM", subsystem)?;
        self.subsystem_set.replace(true);
        self.subsystem.replace(subsystem.to_string());
        Ok(())
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

        self.set_subsystem("drivers")?;
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
            Err(e) => match e.raw_os_error() {
                Some(n) => {
                    if [libc::EACCES, libc::ENODEV, libc::ENXIO, libc::ENOENT].contains(&n) {
                        // the uevent file may be write-only, or the device may be already removed or the device has no uevent file
                        return Ok(());
                    }
                    return Err(Error::Nix {
                        msg: "read_uevent_file failed: can't open uevent file".to_string(),
                        source: Errno::from_i32(n),
                    });
                }
                None => {
                    return Err(Error::Nix {
                        msg: "read_uevent_file failed: can't open uevent file".to_string(),
                        source: Errno::EINVAL,
                    });
                }
            },
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
    pub fn set_devtype(&self, devtype: &str) -> Result<(), Error> {
        self.add_property_internal("DEVTYPE", devtype)?;
        self.devtype.replace(devtype.to_string());
        Ok(())
    }

    /// set ifindex
    pub fn set_ifindex(&self, ifindex: &str) -> Result<(), Error> {
        self.add_property_internal("IFINDEX", ifindex)?;
        self.ifindex.replace(match ifindex.parse::<u32>() {
            Ok(idx) => idx,
            Err(e) => {
                return Err(Error::Nix {
                    msg: format!("set_ifindex failed: {}", e),
                    source: Errno::EINVAL,
                });
            }
        });
        Ok(())
    }

    /// set devname
    pub fn set_devname(&self, devname: &str) -> Result<(), Error> {
        let devname = if devname.starts_with('/') {
            devname.to_string()
        } else {
            format!("/dev/{}", devname)
        };

        self.add_property_internal("DEVNAME", &devname)?;
        self.devname.replace(devname);
        Ok(())
    }

    /// set devmode
    pub fn set_devmode(&self, devmode: &str) -> Result<(), Error> {
        let m = Some(mode_t::from_str_radix(devmode, 8).map_err(|e| Error::Nix {
            msg: format!(
                "set_devmode failed: can't change '{}' to mode: {}",
                devmode, e
            ),
            source: Errno::EINVAL,
        })?);

        self.devmode.replace(m);

        self.add_property_internal("DEVMODE", devmode)?;

        Ok(())
    }

    /// set device uid
    pub fn set_devuid(&self, devuid: &str) -> Result<(), Error> {
        let uid = devuid.parse::<uid_t>().map_err(|e| Error::Nix {
            msg: format!("set_devuid failed: can't change '{}' to uid: {}", devuid, e),
            source: Errno::EINVAL,
        })?;

        self.devuid.replace(Some(Uid::from_raw(uid)));

        self.add_property_internal("DEVUID", devuid)?;

        Ok(())
    }

    /// set device gid
    pub fn set_devgid(&self, devgid: &str) -> Result<(), Error> {
        let gid = devgid.parse::<gid_t>().map_err(|e| Error::Nix {
            msg: format!("set_devgid failed: can't change '{}' to gid: {}", devgid, e),
            source: Errno::EINVAL,
        })?;

        self.devgid.replace(Some(Gid::from_raw(gid)));

        self.add_property_internal("DEVGID", devgid)?;

        Ok(())
    }

    /// set devnum
    pub fn set_devnum(&self, major: &str, minor: &str) -> Result<(), Error> {
        let major_num: u64 = match major.parse() {
            Ok(n) => n,
            Err(e) => {
                return Err(Error::Nix {
                    msg: format!("set_devnum failed: invalid major number '{}': {}", major, e),
                    source: Errno::EINVAL,
                });
            }
        };
        let minor_num: u64 = match minor.parse() {
            Ok(n) => n,
            Err(e) => {
                return Err(Error::Nix {
                    msg: format!("set_devnum failed: invalid minor number '{}': {}", minor, e),
                    source: Errno::EINVAL,
                });
            }
        };

        self.add_property_internal("MAJOR", major)?;
        self.add_property_internal("MINOR", minor)?;
        self.devnum.replace(makedev(major_num, minor_num));

        Ok(())
    }

    /// set diskseq
    pub fn set_diskseq(&self, diskseq: &str) -> Result<(), Error> {
        self.add_property_internal("DISKSEQ", diskseq)?;

        let diskseq_num: u64 = match diskseq.parse() {
            Ok(n) => n,
            Err(e) => {
                return Err(Error::Nix {
                    msg: format!("set_diskseq failed: invalid diskseq '{}': {}", diskseq, e),
                    source: Errno::EINVAL,
                });
            }
        };

        self.diskseq.replace(diskseq_num);

        Ok(())
    }

    /// set action
    pub fn set_action(&self, action: DeviceAction) -> Result<(), Error> {
        self.add_property_internal("ACTION", &action.to_string())?;
        self.action.replace(action);
        Ok(())
    }

    /// set action from string
    pub fn set_action_from_string(&self, action_s: &str) -> Result<(), Error> {
        let action = match action_s.parse::<DeviceAction>() {
            Ok(a) => a,
            Err(_) => {
                return Err(Error::Nix {
                    msg: format!(
                        "set_action_from_string failed: invalid action '{}'",
                        action_s
                    ),
                    source: Errno::EINVAL,
                });
            }
        };

        self.set_action(action)
    }

    /// set seqnum from string
    pub fn set_seqnum_from_string(&self, seqnum_s: &str) -> Result<(), Error> {
        let seqnum: u64 = match seqnum_s.parse() {
            Ok(n) => n,
            Err(_) => {
                return Err(Error::Nix {
                    msg: format!(
                        "set_seqnum_from_string failed: invalid seqnum '{}'",
                        seqnum_s
                    ),
                    source: Errno::EINVAL,
                });
            }
        };

        self.set_seqnum(seqnum)
    }

    /// set seqnum
    pub fn set_seqnum(&self, seqnum: u64) -> Result<(), Error> {
        self.add_property_internal("SEQNUM", &seqnum.to_string())?;
        self.seqnum.replace(seqnum);
        Ok(())
    }

    /// set driver
    pub fn set_driver(&self, driver: &str) -> Result<(), Error> {
        self.add_property_internal("DRIVER", driver)?;
        self.driver_set.replace(true);
        self.driver.replace(driver.to_string());
        Ok(())
    }

    /// cache sysattr value
    pub fn cache_sysattr_value(&self, sysattr: &str, value: &str) -> Result<(), Error> {
        if value.is_empty() {
            self.remove_cached_sysattr_value(sysattr)?;
        } else {
            self.sysattr_values
                .borrow_mut()
                .insert(sysattr.to_string(), value.to_string());
        }

        Ok(())
    }

    /// remove cached sysattr value
    pub fn remove_cached_sysattr_value(&self, sysattr: &str) -> Result<(), Error> {
        self.sysattr_values.borrow_mut().remove(sysattr);

        Ok(())
    }

    /// get cached sysattr value
    pub fn get_cached_sysattr_value(&self, sysattr: &str) -> Result<String, Error> {
        if !self.sysattr_values.borrow().contains_key(sysattr) {
            return Err(Error::Nix {
                msg: format!(
                    "get_cached_sysattr_value failed: no cached sysattr '{}'",
                    sysattr
                ),
                source: Errno::ESTALE,
            });
        }

        match self.sysattr_values.borrow().get(sysattr) {
            Some(value) => Ok(value.clone()),
            None => Err(Error::Nix {
                msg: format!(
                    "get_cached_sysattr_value failed: non-existing sysattr '{}'",
                    sysattr
                ),
                source: Errno::ENOENT,
            }),
        }
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
                            msg: "new_from_child failed: no available parent device until /sys"
                                .to_string(),
                            source: Errno::ENODEV,
                        });
                    }

                    let path = p
                        .to_str()
                        .ok_or(Error::Nix {
                            msg: format!("new_from_child failed: invalid path '{:?}'", p),
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

    /// prepare properties:
    /// 1. read from uevent file
    /// 2. read database
    /// 3. if self devlinks are outdated, add to internal property
    /// 4. if self tags are outdated ,add to internal property
    pub fn properties_prepare(&self) -> Result<(), Error> {
        self.read_uevent_file().map_err(|e| Error::Nix {
            msg: format!("properties_prepare failed: {}", e),
            source: e.get_errno(),
        })?;

        self.read_db().map_err(|e| Error::Nix {
            msg: format!("properties_prepare failed: {}", e),
            source: e.get_errno(),
        })?;

        let property_devlinks_outdated = *self.property_devlinks_outdated.borrow();
        if property_devlinks_outdated {
            let devlinks: String = {
                let devlinks = self.devlinks.borrow();
                let devlinks_vec = devlinks.iter().map(|s| s.as_str()).collect::<Vec<_>>();
                devlinks_vec.join(" ")
            };

            if !devlinks.is_empty() {
                self.add_property_internal("DEVLINKS", &devlinks)
                    .map_err(|e| Error::Nix {
                        msg: format!("properties_prepare failed: {}", e),
                        source: e.get_errno(),
                    })?;

                self.property_devlinks_outdated.replace(false);
            }
        }

        let property_tags_outdated = *self.property_tags_outdated.borrow();
        if property_tags_outdated {
            let all_tags: String = {
                let all_tags = self.all_tags.borrow();
                let tags_vec = all_tags.iter().map(|s| s.as_str()).collect::<Vec<_>>();
                tags_vec.join(":")
            };

            if !all_tags.is_empty() {
                self.add_property_internal("TAGS", &all_tags)
                    .map_err(|e| Error::Nix {
                        msg: format!("properties_prepare failed: {}", e),
                        source: e.get_errno(),
                    })?;
            }

            let current_tags: String = {
                let current_tags = self.current_tags.borrow();
                let tags_vec = current_tags.iter().map(|s| s.as_str()).collect::<Vec<_>>();
                tags_vec.join(":")
            };

            if !current_tags.is_empty() {
                self.add_property_internal("CURRENT_TAGS", &current_tags)
                    .map_err(|e| Error::Nix {
                        msg: format!("properties_prepare failed: {}", e),
                        source: e.get_errno(),
                    })?;
            }

            self.property_tags_outdated.replace(false);
        }

        Ok(())
    }

    /// read database internally from specific file
    pub fn read_db_internal_filename(&self, filename: &str) -> Result<(), Error> {
        let mut file = match fs::OpenOptions::new().read(true).open(filename) {
            Ok(f) => f,
            Err(e) => match e.raw_os_error() {
                Some(n) => {
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
                None => {
                    return Err(Error::Nix {
                        msg: format!(
                            "read_db_internal_filename failed: can't open db '{}': {}",
                            filename, e
                        ),
                        source: Errno::EINVAL,
                    });
                }
            },
        };

        let mut buf = String::new();
        file.read_to_string(&mut buf).map_err(|e| Error::Nix {
            msg: format!(
                "read_db_internal_filename failed: can't read db '{}': {}",
                filename, e
            ),
            source: e
                .raw_os_error()
                .map(nix::Error::from_i32)
                .unwrap_or(nix::Error::EIO),
        })?;

        self.is_initialized.replace(true);
        self.db_loaded.replace(true);

        for line in buf.split('\n') {
            if line.is_empty() {
                continue;
            }

            let key = &line[0..1];
            let value = &line[2..];

            self.handle_db_line(key, value).map_err(|e| Error::Nix {
                msg: format!("read_db_internal_filename failed: {}", e),
                source: e.get_errno(),
            })?;
        }

        Ok(())
    }

    /// handle database line
    pub fn handle_db_line(&self, key: &str, value: &str) -> Result<(), Error> {
        match key {
            "G" | "Q" => {
                self.add_tag(value, key == "Q").map_err(|e| Error::Nix {
                    msg: format!("handle_db_line failed: failed to add_tag: {}", e),
                    source: e.get_errno(),
                })?;
            }
            "S" => {
                self.add_devlink(&format!("/dev/{}", value))
                    .map_err(|e| Error::Nix {
                        msg: format!("handle_db_line failed: failed to add_devlink: {}", e),
                        source: e.get_errno(),
                    })?;
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

                self.add_property_internal(k, v).map_err(|e| Error::Nix {
                    msg: format!("handle_db_line failed: {}", e),
                    source: e.get_errno(),
                })?;
            }
            "I" => {
                let time = value.parse::<u64>().map_err(|e| Error::Nix {
                    msg: format!(
                        "handle_db_line failed: invalid initialized time '{}': {}",
                        value, e
                    ),
                    source: Errno::EINVAL,
                })?;

                self.set_usec_initialized(time).map_err(|e| Error::Nix {
                    msg: format!("handle_db_line failed: {}", e),
                    source: Errno::EINVAL,
                })?;
            }
            "L" => {
                let priority = value.parse::<i32>().map_err(|e| Error::Nix {
                    msg: format!(
                        "handle_db_line failed: failed to parse devlink priority '{}': {}",
                        value, e
                    ),
                    source: Errno::EINVAL,
                })?;

                self.devlink_priority.replace(priority);
            }
            "W" => {
                log::debug!("watch handle in database is deprecated.");
            }
            "V" => {
                let version = value.parse::<u32>().map_err(|e| Error::Nix {
                    msg: format!(
                        "handle_db_line failed: failed to parse database version '{}': {}",
                        value, e
                    ),
                    source: Errno::EINVAL,
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

        let syspath = self.get_syspath().map_err(|e| Error::Nix {
            msg: format!("shallow_clone failed: {}", e),
            source: e.get_errno(),
        })?;

        device
            .set_syspath(&syspath, false)
            .map_err(|e| Error::Nix {
                msg: format!("shallow_clone failed: {}", e),
                source: e.get_errno(),
            })?;

        let subsystem = self.get_subsystem().map_err(|e| Error::Nix {
            msg: format!("shallow_clone failed: {}", e),
            source: e.get_errno(),
        })?;

        device.set_subsystem(&subsystem).map_err(|e| Error::Nix {
            msg: format!("shallow_clone failed: {}", e),
            source: e.get_errno(),
        })?;

        if subsystem == "drivers" {
            device
                .driver_subsystem
                .replace(self.driver_subsystem.borrow().clone());
        }

        if let Ok(ifindex) = self.get_property_value("IFINDEX") {
            device.set_ifindex(&ifindex).map_err(|e| Error::Nix {
                msg: format!("shallow_clone failed: failed to set_ifindex ({})", e),
                source: e.get_errno(),
            })?;
        }

        if let Ok(major) = self.get_property_value("MAJOR") {
            let minor = self.get_property_value("MINOR")?;
            device.set_devnum(&major, &minor).map_err(|e| Error::Nix {
                msg: format!("shallow_clone failed: {}", e),
                source: e.get_errno(),
            })?;
        }

        device.read_uevent_file().map_err(|e| Error::Nix {
            msg: format!("shallow_clone failed: {}", e),
            source: e.get_errno(),
        })?;

        Ok(device)
    }

    /// amend key and value to device object
    pub fn amend_key_value(&self, key: &str, value: &str) -> Result<(), Error> {
        match key {
            "DEVPATH" => self.set_syspath(&format!("/sys{}", value), false)?,
            "ACTION" => self.set_action_from_string(value)?,
            "SUBSYSTEM" => self.set_subsystem(value)?,
            "DEVTYPE" => self.set_devtype(value)?,
            "DEVNAME" => self.set_devname(value)?,
            "SEQNUM" => self.set_seqnum_from_string(value)?,
            "DRIVER" => self.set_driver(value)?,
            "IFINDEX" => self.set_ifindex(value)?,
            "USEC_INITIALIZED" => {
                self.set_usec_initialized(value.parse::<u64>().map_err(|e| Error::Nix {
                    msg: format!(
                        "amend_key_value failed: failed to parse initialized time '{}': {}",
                        value, e
                    ),
                    source: Errno::EINVAL,
                })?)?
            }
            "DEVMODE" => self.set_devmode(value)?,
            "DEVUID" => self.set_devuid(value)?,
            "DEVGID" => self.set_devgid(value)?,
            "DISKSEQ" => self.set_diskseq(value)?,
            "DEVLINKS" => self.add_devlinks(value)?,
            "TAGS" | "CURRENT_TAGS" => self.add_tags(value, key == "CURRENT_TAGS")?,
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
    /// return the tag iterator
    pub fn tag_iter(&self) -> HashSetRefWrapper<String> {
        if let Err(e) = self.read_db() {
            log::error!(
                "failed to read db of '{}': {}",
                self.get_device_id()
                    .unwrap_or_else(|_| self.devpath.borrow().clone()),
                e
            )
        }

        HashSetRefWrapper {
            r: self.all_tags.borrow(),
        }
    }

    /// return the current tag iterator
    pub fn current_tag_iter(&self) -> HashSetRefWrapper<String> {
        if let Err(e) = self.read_db() {
            log::error!(
                "failed to read db of '{}': {}",
                self.get_device_id()
                    .unwrap_or_else(|_| self.devpath.borrow().clone()),
                e
            )
        }

        HashSetRefWrapper {
            r: self.current_tags.borrow(),
        }
    }

    /// return the tag iterator
    pub fn devlink_iter(&self) -> HashSetRefWrapper<String> {
        if let Err(e) = self.read_db() {
            log::error!(
                "failed to read db of '{}': {}",
                self.get_device_id()
                    .unwrap_or_else(|_| self.devpath.borrow().clone()),
                e
            )
        }

        HashSetRefWrapper {
            r: self.devlinks.borrow(),
        }
    }

    /// return the tag iterator
    pub fn property_iter(&self) -> HashMapRefWrapper<String, String> {
        if let Err(e) = self.properties_prepare() {
            log::error!(
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
}

#[cfg(test)]
mod tests {
    use crate::{
        device::*,
        device_enumerator::{DeviceEnumerationType, DeviceEnumerator},
        utils::LoopDev,
    };
    use libc::S_IFBLK;

    /// test a single device
    fn test_device_one(device: &mut Device) {
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
                        // test get_usec_since_initialized: todo
                    }

                    match device.get_property_value("ID_NET_DRIVER") {
                        Ok(_) => {}
                        Err(e) => {
                            assert_eq!(e.get_errno(), Errno::ENOENT);
                        }
                    }
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
                                assert!(basic::errno_util::errno_is_privilege(e.get_errno()));
                            }
                        }
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
    }

    #[test]
    fn test_devices_all() {
        let mut enumerator = DeviceEnumerator::new();
        enumerator.set_enumerator_type(DeviceEnumerationType::All);
        for device in enumerator.iter() {
            test_device_one(&mut *device.as_ref().borrow_mut());
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
            dev.add_tag("test_tag", true).unwrap();

            for tag in &dev.tag_iter() {
                assert_eq!(tag, "test_tag");
            }

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
                    _ => {
                        return Err(Error::Nix {
                            msg: "unwanted property".to_string(),
                            source: nix::Error::EINVAL,
                        })
                    }
                }
            }

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
                    _ => {
                        return Err(Error::Nix {
                            msg: "unwanted property".to_string(),
                            source: nix::Error::EINVAL,
                        })
                    }
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
            dev.add_tags("tag1:tag2", true)?;
            dev.add_property("key", "value")?;
            dev.set_devlink_priority(10);
            dev.set_usec_initialized(1000)?;

            dev.update_db()?;

            let db_path = format!("{}{}", DB_BASE_DIR, dev.get_device_id()?);

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
            dev.update_tag("test_update_tag", true)?;
            let tag_path = format!(
                "/run/devmaster/tags/test_update_tag/{}",
                dev.get_device_id()?
            );
            assert!(Path::new(tag_path.as_str()).exists());

            dev.update_tag("test_update_tag", false)?;
            assert!(!Path::new(tag_path.as_str()).exists());

            Ok(())
        }

        if let Err(e) = LoopDev::inner_process("/tmp/test_update_tag", 1024 * 10, inner_test) {
            assert!(e.is_errno(nix::Error::EACCES) || e.is_errno(nix::Error::EBUSY));
        }
    }
}
