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
use basic::parse_util::{device_path_parse_devnum, parse_devnum, parse_ifindex};
use libc::{dev_t, mode_t, S_IFBLK, S_IFCHR, S_IFDIR, S_IFLNK, S_IFMT, S_IRUSR};
use nix::errno::Errno;
use nix::fcntl::{open, OFlag};
use nix::sys::stat::{self, lstat, major, makedev, minor, stat};
use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::result::Result;
use std::sync::{Arc, Mutex};

use crate::err_wrapper;
use crate::utils::readlink_value;
use crate::{error::Error, DeviceAction};

/// database directory path
pub const DB_DIRECTORY_PATH: &str = "/run/udev/data/";

/// Device
#[derive(Debug, Clone)]
pub struct Device {
    /// inotify handler
    pub watch_handle: i32,
    /// the parent device
    pub parent: Option<Arc<Mutex<Device>>>,
    /// ifindex
    pub ifindex: u32,
    /// device type
    pub devtype: String,
    /// device name, e.g., /dev/sda
    pub devname: String,
    /// device number
    pub devnum: u64,
    /// syspath with /sys/ as prefix, e.g., /sys/devices/pci0000:00/0000:00:10.0/host2/target2:0:1/2:0:1:0/block/sda
    pub syspath: String,
    /// relative path under /sys/, e.g., /devices/pci0000:00/0000:00:10.0/host2/target2:0:1/2:0:1:0/block/sda
    pub devpath: String,
    /// sysnum
    pub sysnum: String,
    /// sysname is the basename of syspath, e.g., sda
    pub sysname: String,
    /// device subsystem
    pub subsystem: String,
    /// only set when subsystem is 'drivers'
    pub driver_subsystem: String,
    /// device driver
    pub driver: String,
    /// device id
    pub device_id: String,
    /// device initialized usec
    pub usec_initialized: u64,
    /// device mode
    pub devmode: mode_t,
    /// device user id
    pub devuid: u32,
    /// device group id
    pub devgid: u32,
    // only set when device is passed through netlink
    /// uevent action
    pub action: DeviceAction,
    /// uevent seqnum, only if the device origins from uevent, the seqnum can be greater than zero
    pub seqnum: u64,
    // pub synth_uuid: u64,
    // pub partn: u32,
    /// device properties
    pub properties: HashMap<String, String>,
    /// the subset of properties that should be written to db
    pub properties_db: HashMap<String, String>,
    /// the string of properties
    pub properties_nulstr: Vec<u8>,
    /// the length of properties nulstr
    pub properties_nulstr_len: usize,
    /// cached sysattr values
    pub sysattr_values: HashMap<String, String>,
    /// names of sysattrs
    pub sysattrs: HashSet<String>,
    /// all tags
    pub all_tags: HashSet<String>,
    /// current tags
    pub current_tags: HashSet<String>,
    /// device links
    pub devlinks: HashSet<String>,
    /// device links priority
    pub devlink_priority: i32,
    /// block device sequence number, monothonically incremented by the kernel on create/attach
    pub diskseq: u64,
    /// database version
    pub database_version: u32,

    /// properties are outdated
    pub properties_buf_outdated: bool,
    /// devlinks in properties are outdated
    pub property_devlinks_outdated: bool,
    /// tags in properties are outdated
    pub property_tags_outdated: bool,
    /// whether the device is initialized by reading uevent file
    pub uevent_loaded: bool,
    /// whether the subsystem is initialized
    pub subsystem_set: bool,
    /// whether the parent is set
    pub parent_set: bool,
    /// whether the driver is set
    pub driver_set: bool,
    /// whether the database is loaded
    pub db_loaded: bool,

    /// whether the device object is initialized
    pub is_initialized: bool,
    /// don not read more information from uevent/db
    pub sealed: bool,
    /// persist device db during switching root from initrd
    pub db_persist: bool,
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
            watch_handle: -1,
            ifindex: 0,
            devtype: String::new(),
            devname: String::new(),
            devnum: 0,
            syspath: String::new(),
            devpath: String::new(),
            sysnum: String::new(),
            sysname: String::new(),
            subsystem: String::new(),
            driver_subsystem: String::new(),
            driver: String::new(),
            device_id: String::new(),
            usec_initialized: 0,
            devmode: mode_t::MAX,
            devuid: std::u32::MAX,
            devgid: std::u32::MAX,
            action: DeviceAction::default(),
            seqnum: 0,
            properties: HashMap::new(),
            properties_db: HashMap::new(),
            properties_nulstr: vec![],
            properties_nulstr_len: 0,
            sysattr_values: HashMap::new(),
            sysattrs: HashSet::new(),
            all_tags: HashSet::new(),
            current_tags: HashSet::new(),
            devlinks: HashSet::new(),
            properties_buf_outdated: true,
            uevent_loaded: false,
            subsystem_set: false,
            diskseq: 0,
            parent: None,
            parent_set: false,
            driver_set: false,
            property_devlinks_outdated: true,
            property_tags_outdated: true,
            is_initialized: false,
            db_loaded: false,
            sealed: false,
            database_version: 0,
            devlink_priority: 0,
            db_persist: false,
        }
    }

    /// create Device from buffer
    pub fn from_nulstr(nulstr: &[u8]) -> Result<Device, Error> {
        let mut device = Device::new();
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
                "DEVPATH" => device.set_syspath("/sys".to_string() + value, false)?,
                "ACTION" => device.set_action_from_string(value.to_string())?,
                "SUBSYSTEM" => device.set_subsystem(value.to_string())?,
                "DEVTYPE" => device.set_devtype(value.to_string())?,
                "MINOR" => minor = value.to_string(),
                "MAJOR" => major = value.to_string(),
                "DEVNAME" => device.set_devname(value.to_string())?,
                "SEQNUM" => device.set_seqnum_from_string(value.to_string())?,
                // "PARTN" => {}
                // "SYNTH_UUID" => {}
                // "USEC_INITIALIZED" => {}
                // "DRIVER" => {}
                // "IFINDEX" => {}
                // "DEVMODE" => {}
                // "DEVUID" => {}
                // "DEVGUID" => {}
                // "DISKSEQ" => {}
                // "DEVLINKS" => {}
                "TAGS" | "CURRENT_TAGS" => {}
                _ => {
                    device.add_property_internal(key.to_string(), value.to_string())?;
                }
            }
        }

        if !major.is_empty() {
            device.set_devnum(major, minor)?;
        }

        device.update_properties_bufs()?;

        Ok(device)
    }

    /// create a Device instance from devname
    /// devname is the device path under /dev
    /// e.g. /dev/block/8:0
    /// e.g. /dev/char/7:0
    /// e.g. /dev/sda
    pub fn from_devname(devname: String) -> Result<Device, Error> {
        if !devname.starts_with("/dev") {
            return Err(Error::Nix {
                msg: format!("the devname does not start with /dev {devname}"),
                source: Errno::EINVAL,
            });
        }

        let device = if let Ok((mode, devnum)) = device_path_parse_devnum(devname.clone()) {
            Device::from_mode_and_devnum(mode, devnum)?
        } else {
            match stat(Path::new(&devname)) {
                Ok(st) => Device::from_mode_and_devnum(st.st_mode, st.st_rdev)?,
                Err(e) => {
                    return Err(Error::Nix {
                        msg: format!("syscall stat failed: {devname}"),
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
    pub fn from_syspath(syspath: String, strict: bool) -> Result<Device, Error> {
        if strict && !syspath.starts_with("/sys/") {
            return Err(Error::Nix {
                msg: format!(
                    "from_syspath failed: syspath {} doesn't start with /sys",
                    syspath
                ),
                source: nix::errno::Errno::EINVAL,
            });
        }

        let mut device = Device::default();
        device.set_syspath(syspath, true)?;

        Ok(device)
    }

    /// create a Device instance from path
    /// path falls into two kinds: devname (/dev/...) and syspath (/sys/devices/...)
    pub fn from_path(path: String) -> Result<Device, Error> {
        if path.starts_with("/dev") {
            return Device::from_devname(path);
        }

        Device::from_syspath(path, false)
    }

    /// create a Device instance from devnum
    pub fn from_devnum(device_type: char, devnum: dev_t) -> Result<Device, Error> {
        if device_type != 'b' && device_type != 'c' {
            return Err(Error::Nix {
                msg: format!("from_devnum failed: invalid device type {}", device_type),
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
                    msg: format!("from_ifindex failed: if_indextoname {} failed", ifindex),
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
        let mut dev = Self::from_syspath(syspath.clone(), true).map_err(|e| Error::Nix {
            msg: format!("from_ifindex failed: from_syspath {} ({})", syspath, e),
            source: e.get_errno(),
        })?;

        let i = match dev.get_ifindex() {
            Ok(i) => i,
            Err(e) => {
                if e.get_errno() == Errno::ENOENT {
                    return Err(Error::Nix {
                        msg: format!("from_ifindex failed: get_ifindex ({})", e),
                        source: Errno::ENXIO,
                    });
                }

                return Err(Error::Nix {
                    msg: format!("from_ifindex failed: get_ifindex ({})", e),
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
    pub fn from_subsystem_sysname(subsystem: String, sysname: String) -> Result<Device, Error> {
        let sysname = sysname.replace('/', "!");
        if subsystem == "subsystem" {
            match Device::from_syspath(format!("/sys/bus/{}", sysname), true) {
                Ok(d) => return Ok(d),
                Err(e) => {
                    if e.get_errno() != Errno::ENODEV {
                        return Err(Error::Nix {
                            msg: format!("from_subsystem_sysname failed: from_syspath bus ({})", e),
                            source: e.get_errno(),
                        });
                    }
                }
            }

            match Device::from_syspath(format!("/sys/class/{}", sysname), true) {
                Ok(d) => return Ok(d),
                Err(e) => {
                    if e.get_errno() != Errno::ENODEV {
                        return Err(Error::Nix {
                            msg: format!(
                                "from_subsystem_sysname failed: from_syspath class ({})",
                                e
                            ),
                            source: e.get_errno(),
                        });
                    }
                }
            }
        } else if subsystem == "module" {
            match Device::from_syspath(format!("/sys/module/{}", sysname), true) {
                Ok(d) => return Ok(d),
                Err(e) => {
                    if e.get_errno() != Errno::ENODEV {
                        return Err(Error::Nix {
                            msg: format!(
                                "from_subsystem_sysname failed: from_syspath module ({})",
                                e
                            ),
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
                    match Device::from_syspath(syspath, true) {
                        Ok(d) => return Ok(d),
                        Err(e) => {
                            if e.get_errno() != Errno::ENODEV {
                                return Err(Error::Nix {
                                    msg: format!(
                                        "from_subsystem_sysname failed: from_syspath drivers ({})",
                                        e
                                    ),
                                    source: e.get_errno(),
                                });
                            }
                        }
                    }
                }
            }
        }

        let syspath = format!("/sys/bus/{}/devices/{}", subsystem, sysname);
        match Device::from_syspath(syspath, true) {
            Ok(d) => return Ok(d),
            Err(e) => {
                if e.get_errno() != Errno::ENODEV {
                    return Err(Error::Nix {
                        msg: format!(
                            "from_subsystem_sysname failed: from_syspath drivers ({})",
                            e
                        ),
                        source: e.get_errno(),
                    });
                }
            }
        }

        let syspath = format!("/sys/class/{}/{}", subsystem, sysname);
        match Device::from_syspath(syspath, true) {
            Ok(d) => return Ok(d),
            Err(e) => {
                if e.get_errno() != Errno::ENODEV {
                    return Err(Error::Nix {
                        msg: format!(
                            "from_subsystem_sysname failed: from_syspath drivers ({})",
                            e
                        ),
                        source: e.get_errno(),
                    });
                }
            }
        }

        let syspath = format!("/sys/firmware/{}/{}", subsystem, sysname);
        match Device::from_syspath(syspath, true) {
            Ok(d) => return Ok(d),
            Err(e) => {
                if e.get_errno() != Errno::ENODEV {
                    return Err(Error::Nix {
                        msg: format!(
                            "from_subsystem_sysname failed: from_syspath drivers ({})",
                            e
                        ),
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
    pub fn set_sysattr_value(
        &mut self,
        sysattr: String,
        value: Option<String>,
    ) -> Result<(), Error> {
        if value.is_none() {
            self.remove_cached_sysattr_value(sysattr)?;
            return Ok(());
        }

        let sysattr_path = self.syspath.clone() + "/" + sysattr.as_str();

        let mut file = match OpenOptions::new().write(true).open(sysattr_path.clone()) {
            Ok(f) => f,
            Err(e) => {
                return Err(Error::Nix {
                    msg: format!("failed to open sysattr file {}", sysattr_path),
                    source: Errno::from_i32(e.raw_os_error().unwrap_or_default()),
                })
            }
        };

        if let Err(e) = file.write(value.clone().unwrap().as_bytes()) {
            self.remove_cached_sysattr_value(sysattr)?;
            return Err(Error::Nix {
                msg: format!("failed to write sysattr file {}", sysattr_path),
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
    pub fn from_device_id(id: String) -> Result<Device, Error> {
        if id.len() < 2 {
            return Err(Error::Nix {
                msg: format!("from_device_id failed: invalid id {}", id),
                source: Errno::EINVAL,
            });
        }

        match id.chars().next() {
            Some('b') | Some('c') => {
                let devnum = parse_devnum(id[1..].to_string()).map_err(|_| Error::Nix {
                    msg: format!("from_device_id failed: parse_devnum {} failed", id),
                    source: Errno::EINVAL,
                })?;

                return Device::from_devnum(id.chars().next().unwrap(), devnum).map_err(|e| {
                    Error::Nix {
                        msg: format!("from_device_id failed: from_devnum ({})", e),
                        source: e.get_errno(),
                    }
                });
            }
            Some('n') => {
                let ifindex = parse_ifindex(id[1..].to_string()).map_err(|_| Error::Nix {
                    msg: format!("from_device_id failed: parse_ifindex {} failed", id),
                    source: Errno::EINVAL,
                })?;

                Device::from_ifindex(ifindex).map_err(|e| Error::Nix {
                    msg: format!("from_device_id failed: from_ifindex ({})", e),
                    source: e.get_errno(),
                })
            }
            Some('+') => {
                let sep = match id.find(':') {
                    Some(idx) => {
                        if idx == id.len() - 1 {
                            return Err(Error::Nix {
                                msg: format!("from_device_id failed: invalid device id {}", id),
                                source: Errno::EINVAL,
                            });
                        }

                        idx
                    }
                    None => {
                        return Err(Error::Nix {
                            msg: format!("from_device_id failed: invalid device id {}", id),
                            source: Errno::EINVAL,
                        });
                    }
                };

                let subsystem = id[1..sep].to_string();
                let sysname = id[sep + 1..].to_string();
                Device::from_subsystem_sysname(subsystem, sysname).map_err(|e| Error::Nix {
                    msg: format!("from_device_id failed: from_subsystem_sysname ({})", e),
                    source: e.get_errno(),
                })
            }
            _ => Err(Error::Nix {
                msg: format!("from_device_id failed: invalid id {}", id),
                source: Errno::EINVAL,
            }),
        }
    }

    /// trigger a fake device action, then kernel will report an uevent
    pub fn trigger(&mut self, action: DeviceAction) -> Result<(), Error> {
        self.set_sysattr_value("uevent".to_string(), Some(format!("{}", action)))
    }

    /// get the syspath of the device
    pub fn get_syspath(&self) -> Result<&str, Error> {
        if self.syspath.is_empty() {
            return Err(Error::Nix {
                msg: "syspath is not found".to_string(),
                source: Errno::ENOENT,
            });
        }

        Ok(&self.syspath)
    }

    /// get the devpath of the device
    pub fn get_devpath(&self) -> Result<&str, Error> {
        if self.devpath.is_empty() {
            return Err(Error::Nix {
                msg: "devpath is not found".to_string(),
                source: Errno::ENOENT,
            });
        }

        Ok(&self.devpath)
    }

    /// get the sysname of the device
    pub fn get_sysname(&mut self) -> Result<&str, Error> {
        if self.sysname.is_empty() {
            err_wrapper!(self.set_sysname_and_sysnum(), "get_sysname")?;
        }

        Ok(&self.sysname)
    }

    /// get the parent of the device
    pub fn get_parent(&mut self) -> Result<Arc<Mutex<Device>>, Error> {
        if !self.parent_set {
            match Device::new_from_child(self) {
                Ok(parent) => self.parent = Some(Arc::new(Mutex::new(parent))),
                Err(e) => {
                    // it is okay if no parent device is found,
                    if e.get_errno() != Errno::ENODEV {
                        return Err(Error::Nix {
                            msg: format!("get parent failed because ({})", e),
                            source: e.get_errno(),
                        });
                    }
                }
            };
            self.parent_set = true;
        }

        if self.parent.is_none() {
            return Err(Error::Nix {
                msg: format!("device {} has no parent", self.devpath),
                source: Errno::ENOENT,
            });
        }

        return Ok(self.parent.as_ref().unwrap().clone());
    }

    /// get the parent of the device
    pub fn get_parent_with_subsystem_devtype(
        &mut self,
        subsystem: &str,
        devtype: Option<&str>,
    ) -> Result<Arc<Mutex<Device>>, Error> {
        let mut parent = match self.get_parent() {
            Ok(parent) => parent,
            Err(e) => return Err(e),
        };

        loop {
            let parent_subsystem = parent.lock().unwrap().get_subsystem();

            if parent_subsystem.is_ok() && parent_subsystem.unwrap() == subsystem {
                if devtype.is_none() {
                    break;
                }

                let parent_devtype = parent.lock().unwrap().get_devtype();
                if parent_devtype.is_ok() && parent_devtype.unwrap() == devtype.unwrap() {
                    break;
                }
            }

            let tmp = parent.lock().unwrap().get_parent().map_err(|e| Error::Nix {
                msg: format!("get_parent_with_subsystem_devtype failed: failed to find satisfied parent ({})", e),
                source: e.get_errno(),
            })?;
            parent = tmp;
        }

        Ok(parent)
    }

    /// get subsystem
    pub fn get_subsystem(&mut self) -> Result<String, Error> {
        if !self.subsystem_set {
            let subsystem_path = self.syspath.clone() + "/subsystem";
            let subsystem_path = Path::new(subsystem_path.as_str());

            // get the base name of absolute subsystem path
            // e.g. /sys/devices/pci0000:00/0000:00:10.0/host2/target2:0:1/2:0:1:0/block/sda/subsystem -> ../../../../../../../../class/block
            // get `block`
            let filename = if Path::exists(Path::new(subsystem_path)) {
                readlink_value(subsystem_path).map_err(|e| Error::Nix {
                    msg: format!("get_subsystem failed: readlink_value ({})", e),
                    source: e.get_errno(),
                })?
            } else {
                "".to_string()
            };

            if !filename.is_empty() {
                self.set_subsystem(filename)?;
            } else if self.devpath.starts_with("/module/") {
                self.set_subsystem("module".to_string())?;
            } else if self.devpath.contains("/drivers/") || self.devpath.contains("/drivers") {
                self.set_drivers_subsystem()?;
            } else if self.devpath.starts_with("/class/") || self.devpath.starts_with("/bus/") {
                self.set_subsystem("subsystem".to_string())?;
            } else {
                self.subsystem_set = true;
            }
        };

        if !self.subsystem.is_empty() {
            Ok(self.subsystem.clone())
        } else {
            Err(Error::Nix {
                msg: "get_subsystem failed: no available subsystem".to_string(),
                source: Errno::ENOENT,
            })
        }
    }

    /// get the ifindex of device
    pub fn get_ifindex(&mut self) -> Result<u32, Error> {
        self.read_uevent_file().map_err(|e| Error::Nix {
            msg: format!("get_ifindex failed: read_uevent_file ({})", e),
            source: e.get_errno(),
        })?;

        if self.ifindex == 0 {
            return Err(Error::Nix {
                msg: "get_ifindex failed: no ifindex".to_string(),
                source: Errno::ENOENT,
            });
        }

        Ok(self.ifindex)
    }

    /// get the device type
    pub fn get_devtype(&mut self) -> Result<String, Error> {
        match self.read_uevent_file() {
            Ok(_) => {}
            Err(e) => {
                return Err(e);
            }
        }

        if self.devtype.is_empty() {
            return Err(Error::Nix {
                msg: "get_devtype failed: no available devname".to_string(),
                source: Errno::ENOENT,
            });
        }

        Ok(self.devtype.clone())
    }

    /// get devnum
    pub fn get_devnum(&mut self) -> Result<u64, Error> {
        match self.read_uevent_file() {
            Ok(_) => {}
            Err(e) => {
                return Err(e);
            }
        }

        if major(self.devnum) == 0 {
            return Err(Error::Nix {
                msg: "get_devnum failed: no devnum".to_string(),
                source: Errno::ENOENT,
            });
        }

        Ok(self.devnum)
    }

    /// get driver
    pub fn get_driver(&mut self) -> Result<String, Error> {
        if !self.driver_set {
            let syspath = self.get_syspath().unwrap().to_string();
            let driver_path_str = syspath + "/driver";
            let driver_path = Path::new(&driver_path_str);
            let driver = match readlink_value(driver_path) {
                Ok(filename) => filename,
                Err(e) => {
                    if e.get_errno() != Errno::ENOENT {
                        return Err(Error::Nix {
                            msg: format!("get_driver failed: readlink_value ({})", e),
                            source: e.get_errno(),
                        });
                    }

                    String::new()
                }
            };

            // if the device has no driver, clear it from internal property
            self.set_driver(driver).map_err(|e| Error::Nix {
                msg: format!("get_driver failed: set_driver ({})", e),
                source: e.get_errno(),
            })?;
        }

        if self.driver.is_empty() {
            return Err(Error::Nix {
                msg: "get_driver failed: no driver".to_string(),
                source: Errno::ENOENT,
            });
        }

        Ok(self.driver.clone())
    }

    /// get device name
    pub fn get_devname(&mut self) -> Result<String, Error> {
        match self.read_uevent_file() {
            Ok(_) => {}
            Err(e) => {
                return Err(e);
            }
        }

        if self.devname.is_empty() {
            return Err(Error::Nix {
                msg: "get_devname failed: no available devname".to_string(),
                source: Errno::ENOENT,
            });
        }

        Ok(self.devname.clone())
    }

    /// get device sysnum
    pub fn get_sysnum(&mut self) -> Result<String, Error> {
        if self.sysname.is_empty() {
            self.set_sysname_and_sysnum().map_err(|e| Error::Nix {
                msg: format!("get_sysnum failed: get_sysnum ({})", e),
                source: e.get_errno(),
            })?;
        }

        if self.sysnum.is_empty() {
            return Err(Error::Nix {
                msg: "get_sysnum failed: no sysnum".to_string(),
                source: Errno::ENOENT,
            });
        }

        Ok(self.sysnum.clone())
    }

    /// get device action
    pub fn get_action(&self) -> Result<DeviceAction, Error> {
        if self.action == DeviceAction::Invalid {
            return Err(Error::Nix {
                msg: format!(
                    "get_action failed: {} does not have uevent action",
                    self.devpath
                ),
                source: Errno::ENOENT,
            });
        }

        Ok(self.action)
    }

    /// get device seqnum, if seqnum is greater than zero, return Ok, otherwise return Err
    pub fn get_seqnum(&self) -> Result<u64, Error> {
        if self.seqnum == 0 {
            return Err(Error::Nix {
                msg: "get_seqnum failed: no seqnum".to_string(),
                source: Errno::ENOENT,
            });
        }

        Ok(self.seqnum)
    }

    /// get device diskseq
    pub fn get_diskseq(&mut self) -> Result<u64, Error> {
        self.read_uevent_file().map_err(|e| Error::Nix {
            msg: format!("get_diskseq failed: failed to read_uevent_file ({})", e),
            source: e.get_errno(),
        })?;

        if self.diskseq == 0 {
            return Err(Error::Nix {
                msg: format!("get_diskseq failed: {} does not have diskseq", self.devpath),
                source: Errno::ENOENT,
            });
        }

        Ok(self.diskseq)
    }

    /// get is initialized
    pub fn get_is_initialized(&mut self) -> Result<bool, Error> {
        // match self.read_db
        match self.read_db() {
            Ok(_) => {}
            Err(e) => {
                if e.get_errno() == Errno::ENOENT {
                    return Ok(false);
                }

                return Err(Error::Nix {
                    msg: format!("get_is_initialized failed: failed to read_db ({})", e),
                    source: e.get_errno(),
                });
            }
        }

        Ok(self.is_initialized)
    }

    /// get initialized usec
    pub fn get_usec_initialized(&mut self) -> Result<u64, Error> {
        todo!("require get_is_initialized");
    }

    /// get usec since initialization
    pub fn get_usec_since_initialized(&mut self) -> Result<u64, Error> {
        todo!("require get_usec_initialized");
    }

    /// check whether the device has the tag
    pub fn has_tag(&mut self, tag: &str) -> Result<bool, Error> {
        self.read_db().map_err(|e| Error::Nix {
            msg: format!("has_tag failed: failed to read db ({})", e),
            source: e.get_errno(),
        })?;

        Ok(self.all_tags.contains(tag))
    }

    /// check whether the device has the current tag
    pub fn has_current_tag(&mut self, tag: &str) -> Result<bool, Error> {
        self.read_db().map_err(|e| Error::Nix {
            msg: format!("has_tag failed: failed to read db ({})", e),
            source: e.get_errno(),
        })?;

        Ok(self.current_tags.contains(tag))
    }

    /// get the value of specific device property
    pub fn get_property_value(&mut self, key: &str) -> Result<String, Error> {
        self.properties_prepare().map_err(|e| Error::Nix {
            msg: format!("get_property_value failed: properties_prepare ({})", e),
            source: e.get_errno(),
        })?;

        match self.properties.get(key) {
            Some(v) => Ok(v.clone()),
            None => Err(Error::Nix {
                msg: format!("get_property_value failed: no key {}", key),
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
    pub fn get_sysattr_value(&mut self, sysattr: &str) -> Result<String, Error> {
        // check whether the sysattr is already cached
        match self.get_cached_sysattr_value(sysattr.to_string()) {
            Ok(v) => {
                return Ok(v);
            }
            Err(e) => {
                if e.get_errno() != Errno::ESTALE {
                    return Err(Error::Nix {
                        msg: format!("get_sysattr_value failed: get_cached_sysattr_value ({})", e),
                        source: e.get_errno(),
                    });
                }
            }
        }

        let syspath = self.get_syspath().unwrap().to_string();
        let sysattr_path = syspath + "/" + sysattr;
        // let sysattr_path = sysattr_path.as_str();
        let value = match lstat(sysattr_path.as_str()) {
            Ok(stat) => {
                if stat.st_mode & S_IFMT == S_IFLNK {
                    if ["driver", "subsystem", "module"].contains(&sysattr) {
                        readlink_value(sysattr_path)?
                    } else {
                        return Err(Error::Nix {
                            msg: format!("get_sysattr_value failed: invalid sysattr {}", sysattr),
                            source: Errno::EINVAL,
                        });
                    }
                } else if stat.st_mode & S_IFMT == S_IFDIR {
                    return Err(Error::Nix {
                        msg: format!(
                            "get_sysattr_value failed: sysattr can not be directory {}",
                            sysattr
                        ),
                        source: Errno::EISDIR,
                    });
                } else if stat.st_mode & S_IRUSR == 0 {
                    return Err(Error::Nix {
                        msg: format!(
                            "get_sysattr_value failed: non-readable sysattr file {}",
                            sysattr
                        ),
                        source: Errno::EPERM,
                    });
                } else {
                    // read full virtual file
                    let mut fd = std::fs::OpenOptions::new()
                        .read(true)
                        .open(sysattr_path.clone())
                        .map_err(|e| Error::Nix {
                            msg: format!(
                                "get_sysattr_value failed: open sysattr file failed {} ({})",
                                sysattr_path, e
                            ),
                            source: Errno::from_i32(e.raw_os_error().unwrap_or_default()),
                        })?;
                    let mut value = String::new();
                    fd.read_to_string(&mut value).map_err(|e| Error::Nix {
                        msg: format!(
                            "get_sysattr_value failed: read sysattr file failed {} ({})",
                            sysattr_path, e
                        ),
                        source: Errno::from_i32(e.raw_os_error().unwrap_or_default()),
                    })?;
                    value.trim_end().to_string()
                }
            }

            Err(e) => {
                self.remove_cached_sysattr_value(sysattr.to_string())
                    .unwrap();
                return Err(Error::Nix {
                    msg: format!("get_sysattr_value failed: lstat {}", sysattr_path),
                    source: e,
                });
            }
        };

        self.cache_sysattr_value(sysattr.to_string(), value.clone())
            .map_err(|e| Error::Nix {
                msg: format!("get_sysattr_value failed: cache_sysattr_value ({})", e),
                source: e.get_errno(),
            })?;

        Ok(value)
    }

    /// trigger with uuid
    pub fn trigger_with_uuid(&mut self, _action: DeviceAction) -> Result<[u8; 8], Error> {
        todo!()
    }

    /// open device
    pub fn open(&mut self, oflags: OFlag) -> Result<i32, Error> {
        let devname = self.get_devname().map_err(|e| {
            if e.get_errno() == Errno::ENOENT {
                Error::Nix {
                    msg: format!("open failed: failed to get_devname ({})", e),
                    source: Errno::ENOEXEC,
                }
            } else {
                Error::Nix {
                    msg: format!("open failed: failed to get_devname ({})", e),
                    source: e.get_errno(),
                }
            }
        })?;

        let devnum = self.get_devnum().map_err(|e| {
            if e.get_errno() == Errno::ENOENT {
                Error::Nix {
                    msg: format!("open failed: failed to get_devnum ({})", e),
                    source: Errno::ENOEXEC,
                }
            } else {
                Error::Nix {
                    msg: format!("open failed: failed to get_devnum ({})", e),
                    source: e.get_errno(),
                }
            }
        })?;

        let subsystem = match self.get_subsystem() {
            Ok(s) => s,
            Err(e) => {
                if e.get_errno() != Errno::ENOENT {
                    return Err(Error::Nix {
                        msg: format!("open failed: failed to get_subsystem ({})", e),
                        source: e.get_errno(),
                    });
                }

                "".to_string()
            }
        };

        let fd = match open(
            devname.as_str(),
            if oflags.intersects(OFlag::O_PATH) {
                oflags
            } else {
                OFlag::O_CLOEXEC | OFlag::O_NOFOLLOW | OFlag::O_PATH
            },
            stat::Mode::empty(),
        ) {
            Ok(fd) => fd,
            Err(e) => {
                return Err(Error::Nix {
                    msg: format!("open failed: failed to open {}", devname),
                    source: e,
                })
            }
        };

        let stat = match nix::sys::stat::fstat(fd) {
            Ok(s) => s,
            Err(e) => {
                return Err(Error::Nix {
                    msg: format!("open failed: failed to fstat fd {} for {}", fd, devname),
                    source: e,
                })
            }
        };

        if stat.st_rdev != devnum {
            return Err(Error::Nix {
                msg: format!(
                    "open failed: device number is inconsistent, st_rdev {}, devnum {}",
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
                        "open failed: subsystem is inconsistent, st_mode {}, subsystem {}",
                        stat.st_mode, subsystem
                    ),
                    source: Errno::ENXIO,
                });
            }
        } else if stat.st_mode & S_IFMT != S_IFCHR {
            // the device is not char
            return Err(Error::Nix {
                msg: format!(
                    "open failed: subsystem is inconsistent, st_mode {}, subsystem {}",
                    stat.st_mode, subsystem
                ),
                source: Errno::ENXIO,
            });
        }

        // if open flags has O_PATH, then we cannot check diskseq
        if oflags.intersects(OFlag::O_PATH) {
            return Ok(fd);
        }

        let mut diskseq: u64 = 0;

        if self.get_is_initialized().map_err(|e| Error::Nix {
            msg: format!("open failed: failed to get_is_initialized ({})", e),
            source: e.get_errno(),
        })? {
            match self.get_property_value("ID_IGNORE_DISKSEQ") {
                Ok(value) => {
                    if !value.parse::<bool>().map_err(|e| Error::Nix {
                        msg: format!(
                            "open failed: failed to parse value {} to boolean ({})",
                            value, e
                        ),
                        source: Errno::EINVAL,
                    })? {
                        match self.get_diskseq() {
                            Ok(n) => diskseq = n,
                            Err(e) => {
                                if e.get_errno() != Errno::ENOENT {
                                    return Err(Error::Nix {
                                        msg: format!("open failed: failed to get_diskseq ({})", e),
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
                        msg: format!("open failed: failed to get_property_value \"ID_IGNORE_DISKSEQ\" ({})", e),
                        source: e.get_errno(),
                    });
                    }
                }
            }
        }

        let fd2 = basic::fd_util::fd_reopen(fd, oflags).map_err(|e| Error::Nix {
            msg: format!("open failed: failed to reopen fd {}", fd),
            source: match e {
                basic::Error::Nix { source } => source,
                _ => Errno::EINVAL,
            },
        })?;

        if diskseq == 0 {
            return Ok(fd2);
        }

        let q = basic::fd_util::fd_get_diskseq(fd2).map_err(|e| Error::Nix {
            msg: format!("open failed: failed to get diskseq on fd {}", fd2),
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

        Ok(fd2)
    }

    /// add property into device
    pub fn add_property(&mut self, key: String, value: String) -> Result<(), Error> {
        self.add_property_aux(key.clone(), value.clone(), false)?;

        if !key.starts_with('.') {
            self.add_property_aux(key, value, true)
                .map_err(|e| Error::Nix {
                    msg: format!("add_property failed: add_property_aux ({})", e),
                    source: e.get_errno(),
                })?;
        }

        Ok(())
    }

    /// shadow clone a device object and import properties from db
    pub fn clone_with_db(&mut self) -> Result<Device, Error> {
        let mut device = self.shallow_clone().map_err(|e| Error::Nix {
            msg: format!("clone_with_db failed: failed to shallow_clone ({})", e),
            source: e.get_errno(),
        })?;

        device.read_db().map_err(|e| Error::Nix {
            msg: format!("clone_with_db failed: failed to read_db ({})", e),
            source: e.get_errno(),
        })?;

        device.sealed = true;

        Ok(device)
    }

    /// add tag to the device object
    pub fn add_tag(&mut self, tag: String, both: bool) -> Result<(), Error> {
        self.all_tags.insert(tag.clone());

        if both {
            self.current_tags.insert(tag);
        }
        self.property_tags_outdated = true;
        Ok(())
    }

    /// remove specific tag
    pub fn remove_tag(&mut self, tag: &String) {
        self.current_tags.remove(tag);
        self.property_tags_outdated = true;
    }

    /// cleanup all tags
    pub fn cleanup_tags(&mut self) {
        self.all_tags.clear();
        self.current_tags.clear();

        self.property_tags_outdated = true;
    }

    /// set device db as persist
    pub fn set_db_persist(&mut self) {
        self.db_persist = true;
    }

    /// set the link priority of device node
    pub fn set_devlink_priority(&mut self, priority: i32) {
        self.devlink_priority = priority;
    }

    /// get the device id
    /// device id is used to identify database file in /run/devmaster/data/
    pub fn get_device_id(&mut self) -> Result<String, Error> {
        if self.device_id.is_empty() {
            let subsystem = self.get_subsystem().map_err(|e| Error::Nix {
                msg: format!("get_device_id failed: get_subsystem ({})", e),
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
                let sysname = self
                    .get_sysname()
                    .map_err(|e| Error::Nix {
                        msg: format!("get_device_id failed: ({})", e),
                        source: e.get_errno(),
                    })?
                    .to_string();

                if subsystem == "drivers" {
                    id = format!("+drivers:{}:{}", self.driver_subsystem, sysname);
                } else {
                    id = format!("+{}:{}", subsystem, sysname);
                }
            }
            self.device_id = id;
        }

        Ok(self.device_id.clone())
    }

    /// cleanup devlinks
    pub fn cleanup_devlinks(&mut self) {
        self.devlinks.clear();
        self.property_devlinks_outdated = true;
    }

    /// add devlink records to the device object
    pub fn add_devlink(&mut self, devlink: String) -> Result<(), Error> {
        self.devlinks.insert(devlink);
        self.property_devlinks_outdated = true;

        Ok(())
    }
}

/// internal methods
impl Device {
    /// create a Device instance based on mode and devnum
    pub(crate) fn from_mode_and_devnum(mode: mode_t, devnum: dev_t) -> Result<Device, Error> {
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

        let mut device = Device::default();
        device.set_syspath(syspath, true)?;

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
            msg: format!(
                "from_mode_and_devnum failed: failed to verify subsystem ({})",
                e
            ),
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
    pub(crate) fn set_syspath(&mut self, path: String, verify: bool) -> Result<(), Error> {
        let p = if verify {
            let path = match fs::canonicalize(path.clone()) {
                Ok(pathbuf) => pathbuf,
                Err(e) => {
                    if let Some(libc::ENOENT) = e.raw_os_error() {
                        return Err(Error::Nix {
                            msg: format!("set_syspath failed: invalid syspath {}", path),
                            source: Errno::ENODEV,
                        });
                    }

                    return Err(Error::Nix {
                        msg: format!("set_syspath failed: failed to canonicalize {}", path),
                        source: Errno::from_i32(e.raw_os_error().unwrap_or_default()),
                    });
                }
            };

            if !path.starts_with("/sys") {
                // todo: what if sysfs is mounted on somewhere else?
                // systemd has considered this situation
                return Err(Error::Nix {
                    msg: format!("set_syspath failed: {:?} does not start with /sys", path),
                    source: Errno::EINVAL,
                });
            }

            if path.starts_with("/sys/devices/") {
                if !path.is_dir() {
                    return Err(Error::Nix {
                        msg: format!("set_syspath failed: {:?} is not a directory", path),
                        source: Errno::ENODEV,
                    });
                }

                let uevent_path = path.join("uevent");
                if !uevent_path.exists() {
                    return Err(Error::Nix {
                        msg: format!("set_syspath failed: {:?} does not contain uevent", path),
                        source: Errno::ENODEV,
                    });
                }
            } else if !path.is_dir() {
                return Err(Error::Nix {
                    msg: format!("set_syspath failed: {:?} is not a directory", path),
                    source: Errno::ENODEV,
                });
            }

            // refuse going down into /sys/fs/cgroup/ or similar places
            // where things are not arranged as kobjects in kernel

            match path.as_os_str().to_str() {
                Some(s) => s.to_string(),
                None => {
                    return Err(Error::Nix {
                        msg: format!("set_syspath failed: {:?} can not change to string", path),
                        source: Errno::EINVAL,
                    });
                }
            }
        } else {
            if !path.starts_with("/sys/") {
                return Err(Error::Nix {
                    msg: format!("set_syspath failed: {:?} does not start with /sys", path),
                    source: Errno::EINVAL,
                });
            }

            path
        };

        let devpath = match p.strip_prefix("/sys") {
            Some(p) => p,
            None => {
                return Err(Error::Nix {
                    msg: format!("set_syspath failed: syspath {} does not start with /sys", p),
                    source: Errno::EINVAL,
                });
            }
        };

        if !devpath.starts_with('/') {
            return Err(Error::Nix {
                msg: format!(
                    "set_syspath failed: devpath {} alone is not a valid device path",
                    p
                ),
                source: Errno::ENODEV,
            });
        }

        match self.add_property_internal("DEVPATH".to_string(), devpath.to_string()) {
            Ok(_) => {}
            Err(e) => {
                return Err(Error::Nix {
                    msg: format!("set_syspath failed: ({})", e),
                    source: Errno::ENODEV,
                })
            }
        }
        self.syspath = p.clone();
        self.devpath = String::from(devpath);

        Ok(())
    }

    /// set the sysname and sysnum of device object
    pub(crate) fn set_sysname_and_sysnum(&mut self) -> Result<(), Error> {
        let sysname = match self.devpath.rfind('/') {
            Some(i) => String::from(&self.devpath[i + 1..]),
            None => {
                return Err(Error::Nix {
                    msg: format!(
                        "set_sysname_and_sysnum failed: invalid devpath {}",
                        self.devpath
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
            self.sysnum = String::from(&sysname[ridx..]);
        }

        self.sysname = sysname;
        Ok(())
    }

    /// add property internal, in other words, do not write to external db
    pub(crate) fn add_property_internal(
        &mut self,
        key: String,
        value: String,
    ) -> Result<(), Error> {
        self.add_property_aux(key, value, false)
    }

    /// add property,
    /// if flag db is true, write to self.properties_db,
    /// else write to self.properties, and set self.properties_buf_outdated to true for updating
    pub(crate) fn add_property_aux(
        &mut self,
        key: String,
        value: String,
        db: bool,
    ) -> Result<(), Error> {
        if key.is_empty() {
            return Err(Error::Nix {
                msg: "invalid key".to_string(),
                source: Errno::EINVAL,
            });
        }

        let reference = if db {
            &mut self.properties_db
        } else {
            &mut self.properties
        };

        if value.is_empty() {
            reference.remove(&key);
        } else {
            reference.insert(key, value);
        }

        if !db {
            self.properties_buf_outdated = true;
        }

        Ok(())
    }

    /// get properties nulstr, if it is out of date, update it
    pub(crate) fn get_properties_nulstr(&mut self) -> Result<(&Vec<u8>, usize), Error> {
        self.update_properties_bufs()?;

        Ok((&self.properties_nulstr, self.properties_nulstr_len))
    }

    /// update properties buffer
    pub(crate) fn update_properties_bufs(&mut self) -> Result<(), Error> {
        if !self.properties_buf_outdated {
            return Ok(());
        }
        self.properties_nulstr.clear();
        for (k, v) in self.properties.iter() {
            unsafe {
                self.properties_nulstr.append(k.clone().as_mut_vec());
                self.properties_nulstr.append(&mut vec![b'=']);
                self.properties_nulstr.append(v.clone().as_mut_vec());
                self.properties_nulstr.append(&mut vec![0]);
            }
        }

        self.properties_nulstr_len = self.properties_nulstr.len();
        self.properties_buf_outdated = false;
        Ok(())
    }

    /// set subsystem
    pub(crate) fn set_subsystem(&mut self, subsystem: String) -> Result<(), Error> {
        self.add_property_internal("SUBSYSTEM".to_string(), subsystem.clone())?;
        self.subsystem_set = true;
        self.subsystem = subsystem;
        Ok(())
    }

    /// set drivers subsystem
    pub(crate) fn set_drivers_subsystem(&mut self) -> Result<(), Error> {
        let mut subsystem = String::new();
        let components: Vec<&str> = self.devpath.split('/').collect();
        for (idx, com) in components.iter().enumerate() {
            if *com == "drivers" {
                subsystem = components.get(idx - 1).unwrap().to_string();
                break;
            }
        }

        if subsystem.is_empty() {
            return Err(Error::Nix {
                msg: "invalid driver subsystem".to_string(),
                source: Errno::EINVAL,
            });
        }

        self.set_subsystem("drivers".to_string())?;
        self.driver_subsystem = subsystem;

        Ok(())
    }

    /// read uevent file and filling device attributes
    pub(crate) fn read_uevent_file(&mut self) -> Result<(), Error> {
        if self.uevent_loaded || self.sealed {
            return Ok(());
        }

        let uevent_file = self.syspath.clone() + "/uevent";

        let mut file = match fs::OpenOptions::new().read(true).open(uevent_file) {
            Ok(f) => f,
            Err(e) => match e.raw_os_error() {
                Some(n) => {
                    if [libc::EACCES, libc::ENODEV, libc::ENXIO, libc::ENOENT].contains(&n) {
                        // the uevent file may be write-only, or the device may be already removed or the device has no uevent file
                        return Ok(());
                    }
                    return Err(Error::Nix {
                        msg: "failed to open uevent file".to_string(),
                        source: Errno::from_i32(n),
                    });
                }
                None => {
                    return Err(Error::Nix {
                        msg: "failed to open uevent file".to_string(),
                        source: Errno::EINVAL,
                    });
                }
            },
        };

        let mut buf = String::new();
        file.read_to_string(&mut buf).unwrap();

        let mut major = String::new();
        let mut minor = String::new();

        for line in buf.split('\n') {
            let tokens: Vec<&str> = line.split('=').collect();
            if tokens.len() < 2 {
                break;
            }

            let (key, value) = (tokens[0], tokens[1]);

            match key {
                "DEVTYPE" => self.set_devtype(value.to_string())?,
                "IFINDEX" => self.set_ifindex(value.to_string())?,
                "DEVNAME" => self.set_devname(value.to_string())?,
                "DEVMODE" => self.set_devmode(value.to_string())?,
                "DISKSEQ" => self.set_diskseq(value.to_string())?,
                "MAJOR" => {
                    major = value.to_string();
                }
                "MINOR" => {
                    minor = value.to_string();
                }
                _ => {}
            }
        }

        if !major.is_empty() {
            self.set_devnum(major, minor)?;
        }

        self.uevent_loaded = true;

        Ok(())
    }

    /// set devtype
    pub(crate) fn set_devtype(&mut self, devtype: String) -> Result<(), Error> {
        self.add_property_internal("DEVTYPE".to_string(), devtype.clone())?;
        self.devtype = devtype;
        Ok(())
    }

    /// set ifindex
    pub(crate) fn set_ifindex(&mut self, ifindex: String) -> Result<(), Error> {
        self.add_property_internal("IFINDEX".to_string(), ifindex.clone())?;
        self.ifindex = match ifindex.parse() {
            Ok(idx) => idx,
            Err(e) => {
                return Err(Error::Nix {
                    msg: e.to_string(),
                    source: Errno::EINVAL,
                });
            }
        };
        Ok(())
    }

    /// set devname
    pub(crate) fn set_devname(&mut self, devname: String) -> Result<(), Error> {
        let devname = if devname.starts_with('/') {
            devname
        } else {
            "/dev/".to_string() + devname.as_str()
        };

        self.add_property_internal("DEVNAME".to_string(), devname.clone())?;
        self.devname = devname;
        Ok(())
    }

    /// set devmode
    pub(crate) fn set_devmode(&mut self, devmode: String) -> Result<(), Error> {
        self.add_property_internal("DEVMODE".to_string(), devmode.clone())?;

        self.devmode = match devmode.parse() {
            Ok(m) => m,
            Err(e) => {
                return Err(Error::Nix {
                    msg: e.to_string(),
                    source: Errno::EINVAL,
                });
            }
        };

        Ok(())
    }

    /// set devnum
    pub(crate) fn set_devnum(&mut self, major: String, minor: String) -> Result<(), Error> {
        let major_num: u64 = match major.parse() {
            Ok(n) => n,
            Err(e) => {
                return Err(Error::Nix {
                    msg: e.to_string(),
                    source: Errno::EINVAL,
                });
            }
        };
        let minor_num: u64 = match minor.parse() {
            Ok(n) => n,
            Err(e) => {
                return Err(Error::Nix {
                    msg: e.to_string(),
                    source: Errno::EINVAL,
                });
            }
        };

        self.add_property_internal("MAJOR".to_string(), major)?;
        self.add_property_internal("MINOR".to_string(), minor)?;
        self.devnum = makedev(major_num, minor_num);

        Ok(())
    }

    /// set diskseq
    pub(crate) fn set_diskseq(&mut self, diskseq: String) -> Result<(), Error> {
        self.add_property_internal("DISKSEQ".to_string(), diskseq.clone())?;

        let diskseq_num: u64 = match diskseq.parse() {
            Ok(n) => n,
            Err(e) => {
                return Err(Error::Nix {
                    msg: e.to_string(),
                    source: Errno::EINVAL,
                });
            }
        };

        self.diskseq = diskseq_num;

        Ok(())
    }

    /// set action
    pub(crate) fn set_action(&mut self, action: DeviceAction) -> Result<(), Error> {
        self.add_property_internal("ACTION".to_string(), action.to_string())?;
        self.action = action;
        Ok(())
    }

    /// set action from string
    pub(crate) fn set_action_from_string(&mut self, action_s: String) -> Result<(), Error> {
        let action = match action_s.parse::<DeviceAction>() {
            Ok(a) => a,
            Err(_) => {
                return Err(Error::Nix {
                    msg: format!("invalid action string {}", action_s),
                    source: Errno::EINVAL,
                });
            }
        };

        self.set_action(action)
    }

    /// set seqnum from string
    pub(crate) fn set_seqnum_from_string(&mut self, seqnum_s: String) -> Result<(), Error> {
        let seqnum: u64 = match seqnum_s.parse() {
            Ok(n) => n,
            Err(_) => {
                return Err(Error::Nix {
                    msg: format!("invalid seqnum can not be parsed to u64 {}", seqnum_s),
                    source: Errno::EINVAL,
                });
            }
        };

        self.set_seqnum(seqnum)
    }

    /// set seqnum
    pub(crate) fn set_seqnum(&mut self, seqnum: u64) -> Result<(), Error> {
        self.add_property_internal("SEQNUM".to_string(), seqnum.to_string())?;
        self.seqnum = seqnum;
        Ok(())
    }

    /// set driver
    pub(crate) fn set_driver(&mut self, driver: String) -> Result<(), Error> {
        self.add_property_internal("DRIVER".to_string(), driver.clone())?;
        self.driver_set = true;
        self.driver = driver;
        Ok(())
    }

    /// set the initialized timestamp
    pub(crate) fn set_usec_initialized(&mut self, time: u64) -> Result<(), Error> {
        self.add_property_internal("USEC_INITIALIZED".to_string(), time.to_string())?;
        self.usec_initialized = time;
        Ok(())
    }

    /// cache sysattr value
    pub(crate) fn cache_sysattr_value(
        &mut self,
        sysattr: String,
        value: String,
    ) -> Result<(), Error> {
        if value.is_empty() {
            self.remove_cached_sysattr_value(sysattr)?;
        } else {
            self.sysattr_values.insert(sysattr, value);
        }

        Ok(())
    }

    /// remove cached sysattr value
    pub(crate) fn remove_cached_sysattr_value(&mut self, sysattr: String) -> Result<(), Error> {
        self.sysattr_values.remove(&sysattr);

        Ok(())
    }

    /// get cached sysattr value
    pub(crate) fn get_cached_sysattr_value(&self, sysattr: String) -> Result<String, Error> {
        if !self.sysattr_values.contains_key(&sysattr) {
            return Err(Error::Nix {
                msg: format!(
                    "get_cached_sysattr_value failed: no cached sysattr {}",
                    sysattr
                ),
                source: Errno::ESTALE,
            });
        }

        match self.sysattr_values.get(&sysattr) {
            Some(value) => Ok(value.clone()),
            None => Err(Error::Nix {
                msg: format!(
                    "get_cached_sysattr_value failed: non-existing sysattr {}",
                    sysattr
                ),
                source: Errno::ENOENT,
            }),
        }
    }

    /// new from child
    pub(crate) fn new_from_child(device: &mut Device) -> Result<Device, Error> {
        let syspath = Path::new(err_wrapper!(device.get_syspath(), "new_from_child")?);

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
                    let path = p.to_str().unwrap().to_string();

                    match Device::from_syspath(path, true) {
                        Ok(d) => return Ok(d),
                        Err(e) => {
                            if e.get_errno() != Errno::ENODEV {
                                return Err(Error::Nix {
                                    msg: format!(
                                        "new_from_child failed: from_syspath failed ({})",
                                        e
                                    ),
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
    pub(crate) fn properties_prepare(&mut self) -> Result<(), Error> {
        self.read_uevent_file().map_err(|e| Error::Nix {
            msg: format!("properties_prepare failed: read_uevent_file ({})", e),
            source: e.get_errno(),
        })?;

        self.read_db().map_err(|e| Error::Nix {
            msg: format!("properties_prepare failed: read_db ({})", e),
            source: e.get_errno(),
        })?;

        if self.property_devlinks_outdated {
            let devlinks: Vec<String> = self.devlinks.clone().into_iter().collect();
            let devlinks: String = devlinks.join(" ");
            if !devlinks.is_empty() {
                self.add_property_internal("DEVLINKS".to_string(), devlinks)
                    .map_err(|e| Error::Nix {
                        msg: format!(
                            "properties_prepare failed: add_property_internal DEVLINKS ({})",
                            e
                        ),
                        source: e.get_errno(),
                    })?;

                self.property_devlinks_outdated = false;
            }
        }

        if self.property_tags_outdated {
            let tags: Vec<String> = self.all_tags.clone().into_iter().collect();
            let tags: String = tags.join(":");
            if !tags.is_empty() {
                self.add_property_internal("TAGS".to_string(), tags)
                    .map_err(|e| Error::Nix {
                        msg: format!(
                            "properties_prepare failed: add_property_internal TAGS ({})",
                            e
                        ),
                        source: e.get_errno(),
                    })?;
            }

            let tags: Vec<String> = self.current_tags.clone().into_iter().collect();
            let tags: String = tags.join(":");
            if !tags.is_empty() {
                self.add_property_internal("CURRENT_TAGS".to_string(), tags)
                    .map_err(|e| Error::Nix {
                        msg: format!(
                            "properties_prepare failed: add_property_internal CURRENT_TAGS ({})",
                            e
                        ),
                        source: e.get_errno(),
                    })?;
            }

            self.property_tags_outdated = false;
        }

        Ok(())
    }

    /// read database
    pub(crate) fn read_db(&mut self) -> Result<(), Error> {
        self.read_db_internal(false).map_err(|e| Error::Nix {
            msg: format!("read_db failed: failed to read_db_internal ({})", e),
            source: e.get_errno(),
        })
    }

    /// read database internally
    pub(crate) fn read_db_internal(&mut self, force: bool) -> Result<(), Error> {
        if self.db_loaded || (!force && self.sealed) {
            return Ok(());
        }

        let id = self.get_device_id().map_err(|e| Error::Nix {
            msg: format!("read_db_internal failed: failed to get_device_id ({})", e),
            source: e.get_errno(),
        })?;

        let path = DB_DIRECTORY_PATH.to_string() + &id;

        self.read_db_internal_filename(path)
            .map_err(|e| Error::Nix {
                msg: format!(
                    "read_db_internal failed: failed to read_db_internal_filename ({})",
                    e
                ),
                source: e.get_errno(),
            })
    }

    /// read database internally from specific file
    pub(crate) fn read_db_internal_filename(&mut self, filename: String) -> Result<(), Error> {
        let mut file = match fs::OpenOptions::new().read(true).open(filename.clone()) {
            Ok(f) => f,
            Err(e) => match e.raw_os_error() {
                Some(n) => {
                    if n == libc::ENOENT {
                        return Ok(());
                    }
                    return Err(Error::Nix {
                        msg: format!("read_db_internal_filename failed: db {} ({})", filename, e),
                        source: Errno::from_i32(n),
                    });
                }
                None => {
                    return Err(Error::Nix {
                        msg: format!("read_db_internal_filename failed: db {} ({})", filename, e),
                        source: Errno::EINVAL,
                    });
                }
            },
        };

        let mut buf = String::new();
        file.read_to_string(&mut buf).unwrap();

        self.is_initialized = true;
        self.db_loaded = true;

        for line in buf.split('\n') {
            if line.is_empty() {
                continue;
            }

            let key = &line[0..1];
            let value = &line[2..];

            self.handle_db_line(key, value).map_err(|e| Error::Nix {
                msg: format!(
                    "read_db_internal_filename failed: failed to handle_db_line ({})",
                    e
                ),
                source: e.get_errno(),
            })?;
        }

        Ok(())
    }

    /// handle database line
    pub(crate) fn handle_db_line(&mut self, key: &str, value: &str) -> Result<(), Error> {
        match key {
            "G" | "Q" => {
                self.add_tag(value.to_string(), key == "Q")
                    .map_err(|e| Error::Nix {
                        msg: format!("handle_db_line failed: failed to add_tag ({})", e),
                        source: e.get_errno(),
                    })?;
            }
            "S" => {
                self.add_devlink(format!("/dev/{}", value))
                    .map_err(|e| Error::Nix {
                        msg: format!("handle_db_line failed: failed to add_devlink ({})", e),
                        source: e.get_errno(),
                    })?;
            }
            "E" => {
                let tokens: Vec<_> = value.split('=').collect();
                if tokens.len() != 2 {
                    return Err(Error::Nix {
                        msg: format!("handle_db_line failed: failed to parse property {}", value),
                        source: Errno::EINVAL,
                    });
                }

                let (k, v) = (tokens[0], tokens[1]);

                self.add_property_internal(k.to_string(), v.to_string())
                    .map_err(|e| Error::Nix {
                        msg: format!(
                            "handle_db_line failed: failed to add_property_internal ({})",
                            e
                        ),
                        source: e.get_errno(),
                    })?;
            }
            "I" => {
                let time = value.parse::<u64>().map_err(|e| Error::Nix {
                    msg: format!(
                        "handle_db_line failed: failed to parse initialized time {} ({})",
                        value, e
                    ),
                    source: Errno::EINVAL,
                })?;

                self.set_usec_initialized(time).map_err(|e| Error::Nix {
                    msg: format!(
                        "handle_db_line failed: failed to set_usec_initialized ({})",
                        e
                    ),
                    source: Errno::EINVAL,
                })?;
            }
            "L" => {
                let priority = value.parse::<i32>().map_err(|e| Error::Nix {
                    msg: format!(
                        "handle_db_line failed: failed to parse devlink priority {} ({})",
                        value, e
                    ),
                    source: Errno::EINVAL,
                })?;

                self.devlink_priority = priority;
            }
            "W" => {
                log::debug!("watch handle in database is deprecated.");
            }
            "V" => {
                let version = value.parse::<u32>().map_err(|e| Error::Nix {
                    msg: format!(
                        "handle_db_line failed: failed to parse database version {} ({})",
                        value, e
                    ),
                    source: Errno::EINVAL,
                })?;

                self.database_version = version;
            }
            _ => {
                log::debug!(
                    "libdevice: unknown key {} in database line, ignore it.",
                    key
                );
            }
        }

        Ok(())
    }

    /// shallow clone a device object
    pub(crate) fn shallow_clone(&mut self) -> Result<Device, Error> {
        let mut device = Self::default();

        let syspath = self
            .get_syspath()
            .map_err(|e| Error::Nix {
                msg: format!("shallow_clone failed: ({})", e),
                source: e.get_errno(),
            })?
            .to_string();

        device.set_syspath(syspath, false).map_err(|e| Error::Nix {
            msg: format!("shallow_clone failed: failed to set_syspath ({})", e),
            source: e.get_errno(),
        })?;

        let subsystem = self.get_subsystem().map_err(|e| Error::Nix {
            msg: format!("shallow_clone failed: failed to get_subsystem ({})", e),
            source: e.get_errno(),
        })?;

        device
            .set_subsystem(subsystem.clone())
            .map_err(|e| Error::Nix {
                msg: format!("shallow_clone failed: failed to set_subsystem ({})", e),
                source: e.get_errno(),
            })?;

        if subsystem == "drivers" {
            device.driver_subsystem = self.driver_subsystem.clone();
        }

        if let Ok(ifindex) = self.get_property_value("IFINDEX") {
            device.set_ifindex(ifindex).map_err(|e| Error::Nix {
                msg: format!("shallow_clone failed: failed to set_ifindex ({})", e),
                source: e.get_errno(),
            })?;
        }

        if let Ok(major) = self.get_property_value("MAJOR") {
            let minor = self.get_property_value("MINOR").unwrap();
            device.set_devnum(major, minor).map_err(|e| Error::Nix {
                msg: format!("shallow_clone failed: failed to set_devnum ({})", e),
                source: e.get_errno(),
            })?;
        }

        device.read_uevent_file().map_err(|e| Error::Nix {
            msg: format!("shallow_clone failed: failed to read_uevent_file ({})", e),
            source: e.get_errno(),
        })?;

        Ok(device)
    }
}

/// iterator over all tags of device object
pub struct DeviceTagIter<'a> {
    device_tag_iter: std::collections::hash_set::Iter<'a, String>,
}

impl Device {
    /// return the tag iterator
    pub fn tag_iter_mut(&mut self) -> DeviceTagIter<'_> {
        if let Err(e) = self.read_db() {
            log::error!(
                "failed to read db of '{}': ({})",
                self.get_device_id()
                    .unwrap_or_else(|_| self.devpath.clone()),
                e
            )
        }

        DeviceTagIter {
            device_tag_iter: self.all_tags.iter(),
        }
    }
}

impl<'a> Iterator for DeviceTagIter<'a> {
    type Item = &'a String;

    /// get the next tag
    fn next(&mut self) -> Option<Self::Item> {
        self.device_tag_iter.next()
    }
}

/// iterator over all properties of device object
pub struct DevicePropertyIter<'a> {
    device_property_iter: std::collections::hash_map::Iter<'a, String, String>,
}

impl Device {
    /// return the tag iterator
    pub fn property_iter_mut(&mut self) -> Result<DevicePropertyIter<'_>, Error> {
        self.properties_prepare()?;

        Ok(DevicePropertyIter {
            device_property_iter: self.properties.iter(),
        })
    }
}

impl<'a> Iterator for DevicePropertyIter<'a> {
    type Item = (&'a String, &'a String);

    /// get the next tag
    fn next(&mut self) -> Option<Self::Item> {
        self.device_property_iter.next()
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::BorrowMut;

    use crate::{
        device::*,
        device_enumerator::{DeviceEnumerationType, DeviceEnumerator},
    };
    use libc::S_IFBLK;
    use nix::sys::stat::makedev;

    /// test a single device
    fn test_device_one(device: &mut Device) {
        let syspath = device.get_syspath().unwrap().to_string();
        assert!(syspath.starts_with("/sys"));
        let sysname = device.get_sysname().unwrap().to_string();

        // test Device::from_syspath()
        let device_new = Device::from_syspath(syspath.clone(), true).unwrap();
        let syspath_new = device_new.get_syspath().unwrap().to_string();
        assert_eq!(syspath, syspath_new);

        // test Device::from_path()
        let device_new = Device::from_path(syspath.clone()).unwrap();
        let syspath_new = device_new.get_syspath().unwrap().to_string();
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
                let subsystem = subsystem;
                if !subsystem.is_empty() && subsystem != "gpio" {
                    is_block = subsystem == "block";
                    let name = if subsystem == "drivers" {
                        format!("{}:{}", device.driver_subsystem, sysname)
                    } else {
                        sysname
                    };

                    match Device::from_subsystem_sysname(subsystem, name) {
                        Ok(dev) => {
                            assert_eq!(syspath, dev.get_syspath().unwrap());
                        }
                        Err(e) => {
                            assert_eq!(e.get_errno(), Errno::ENODEV);
                        }
                    }

                    let device_id = device.get_device_id().unwrap();
                    match Device::from_device_id(device_id.clone()) {
                        Ok(mut dev) => {
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
                match Device::from_devname(devname.to_string()) {
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

                match Device::from_path(devname) {
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
                            Ok(fd) => {
                                assert!(fd >= 0)
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
                let syspath_new = device_new.get_syspath().unwrap().to_string();
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
                let device_new = Device::from_devname(devname.clone()).unwrap();
                let syspath_new = device_new.get_syspath().unwrap().to_string();
                assert_eq!(syspath, syspath_new);

                let device_new = Device::from_path(devname).unwrap();
                let syspath_new = device_new.get_syspath().unwrap().to_string();
                assert_eq!(syspath, syspath_new);
            }
            Err(e) => {
                assert_eq!(e.get_errno(), Errno::ENOENT);
            }
        }

        device.get_devpath().unwrap().to_string();

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
        for device in enumerator.iter_mut() {
            test_device_one(device.lock().unwrap().borrow_mut());
        }
    }

    /// test whether Device::from_mode_and_devnum can create Device instance normally
    #[ignore]
    #[test]
    fn test_from_mode_and_devnum() {
        let devnum = makedev(8, 0);
        let mode = S_IFBLK;
        let device = Device::from_mode_and_devnum(mode, devnum).unwrap();

        assert_eq!(
            "/sys/devices/pci0000:00/0000:00:10.0/host2/target2:0:1/2:0:1:0/block/sda",
            device.syspath
        );
        assert_eq!(
            "/devices/pci0000:00/0000:00:10.0/host2/target2:0:1/2:0:1:0/block/sda",
            device.devpath
        );
        assert_eq!("block", device.subsystem);
        assert_eq!(makedev(8, 0), device.devnum);
        assert_eq!("/dev/sda", device.devname);
    }

    /// test whether Device::from_devname can create Device instance normally
    #[ignore]
    #[test]
    fn test_from_devname() {
        let devname = "/dev/sda".to_string();
        let device = Device::from_devname(devname).unwrap();

        assert_eq!(
            "/sys/devices/pci0000:00/0000:00:10.0/host2/target2:0:1/2:0:1:0/block/sda",
            device.syspath
        );
        assert_eq!(
            "/devices/pci0000:00/0000:00:10.0/host2/target2:0:1/2:0:1:0/block/sda",
            device.devpath
        );
        assert_eq!("block", device.subsystem);
        assert_eq!("/dev/sda", device.devname);
    }

    /// test whether Device::set_sysattr_value can work normally
    #[ignore]
    #[test]
    fn test_set_sysattr_value() {
        let devname = "/dev/sda".to_string();
        let mut device = Device::from_devname(devname).unwrap();

        device
            .set_sysattr_value("uevent".to_string(), Some("change".to_string()))
            .unwrap();
    }

    /// test device tag iterator
    #[ignore]
    #[test]
    fn test_device_tag_iterator() {
        let mut dev = Device::from_device_id("b8:1".to_string()).unwrap();
        println!("{}", dev.get_syspath().unwrap());
        dev.add_tag("test_tag".to_string(), true).unwrap();
        for tag in dev.tag_iter_mut() {
            println!("{}", tag);
        }
    }

    /// test device property iterator
    #[ignore]
    #[test]
    fn test_device_property_iterator() {
        let mut dev = Device::from_path("/dev/sda".to_string()).unwrap();
        let mut dev_clone = dev.shallow_clone().unwrap();

        for (k, v) in dev_clone.properties.iter() {
            println!("Shallow: {}={}", k, v);
        }

        for (k, v) in dev_clone.property_iter_mut().unwrap() {
            println!("Prepared: {}={}", k, v);
        }
    }
}
