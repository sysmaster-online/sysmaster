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

//! enumerate /sys to collect devices
//!
use crate::{device::Device, err_wrapper, error::Error, utils::*};
use bitflags::bitflags;
use nix::errno::Errno;
use std::{
    borrow::BorrowMut,
    collections::{HashMap, HashSet},
    iter::Iterator,
    path::Path,
    sync::{Arc, Mutex},
};

/// decide pattern recognition
#[derive(Debug, Clone, Copy)]
pub(crate) enum FilterType {
    /// fnmatch-style glob pattern
    Glob,
    /// regular expression
    _Regular,
}

impl Default for FilterType {
    fn default() -> Self {
        Self::Glob
    }
}

/// decide how to match devices
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchInitializedType {
    /// only match devices without db entry
    No,
    /// only match devices with a db entry
    Yes,
    /// match all devices
    ALL,
    /// match devices that have no devnode/ifindex of have a db entry
    Compat,
}

impl Default for MatchInitializedType {
    fn default() -> Self {
        Self::Compat
    }
}

/// enumerate devices or subsystems under /sys
#[derive(Debug, Default)]
pub struct DeviceEnumerator {
    /// enumerator type
    pub(crate) etype: DeviceEnumerationType,
    /// key: syspath, value: device
    pub(crate) devices_by_syspath: HashMap<String, Arc<Mutex<Device>>>,
    /// sorted device vector
    pub(crate) devices: Vec<Arc<Mutex<Device>>>,

    /// whether enumerator is up to date
    pub(crate) scan_up_to_date: bool,
    /// whether devices are sorted
    pub(crate) sorted: bool,

    /// prioritized subsystems
    pub(crate) prioritized_subsystems: Vec<String>,

    /// match subsystem
    pub(crate) match_subsystem: HashSet<String>,
    /// do not match subsystem
    pub(crate) not_match_subsystem: HashSet<String>,

    /// match sysattr
    /// key: sysattr, value: match value
    pub(crate) match_sysattr: HashMap<String, HashSet<String>>,
    /// do not match sysattr
    /// key: sysattr, value: match value
    pub(crate) not_match_sysattr: HashMap<String, HashSet<String>>,

    /// match property
    /// key: property, value: match value
    pub(crate) match_property: HashMap<String, String>,

    /// match sysname
    pub(crate) match_sysname: HashSet<String>,
    /// not match sysname
    pub(crate) not_match_sysname: HashSet<String>,

    /// match tag
    pub(crate) match_tag: HashSet<String>,

    /// match parent
    pub(crate) match_parent: HashSet<String>,

    /// how to match device
    pub(crate) match_initialized: MatchInitializedType,

    /// filter type
    pub(crate) filter_type: FilterType,
}

/// decide enumerate devices or subsystems
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceEnumerationType {
    /// only enumerate devices
    Devices,
    /// only enumerate subsystems
    Subsystems,
    /// enumerate both devices and subsystems
    All,
}

impl Default for DeviceEnumerationType {
    fn default() -> Self {
        Self::Devices
    }
}

/// iterator of device enumerator
pub struct DeviceEnumeratorIter<'a> {
    current_device_index: usize,
    enumerator: &'a mut DeviceEnumerator,
}

impl DeviceEnumerator {
    /// iterate devices
    pub fn iter_mut(&mut self) -> DeviceEnumeratorIter<'_> {
        DeviceEnumeratorIter {
            current_device_index: 0,
            enumerator: self,
        }
    }
}

impl Iterator for DeviceEnumeratorIter<'_> {
    type Item = Arc<Mutex<Device>>;

    /// iterate over the devices or subsystems according to the enumerator type
    fn next(&mut self) -> Option<Self::Item> {
        match self.enumerator.etype {
            DeviceEnumerationType::Devices => {
                if !self.enumerator.scan_up_to_date && self.enumerator.scan_devices().is_err() {
                    return None;
                }
            }
            DeviceEnumerationType::Subsystems => {
                if !self.enumerator.scan_up_to_date && self.enumerator.scan_subsystems().is_err() {
                    return None;
                }
            }
            DeviceEnumerationType::All => {
                if !self.enumerator.scan_up_to_date
                    && self.enumerator.scan_devices_and_subsystems().is_err()
                {
                    return None;
                }
            }
        }

        if !self.enumerator.sorted && self.enumerator.sort_devices().is_err() {
            return None;
        }

        self.enumerator
            .devices
            .get(self.current_device_index)
            .map(|d| {
                self.current_device_index += 1;
                d.clone()
            })
    }
}

bitflags! {
    /// the flag used to control match conditions
    pub struct MatchFlag: u8 {
        /// match sysname
        const SYSNAME = 1 << 0;
        /// match subsystem
        const SUBSYSTEM = 1 << 1;
        /// match parent
        const PARENT = 1 << 2;
        /// match tag
        const TAG = 1 << 3;
        /// match all
        const ALL = (1 << 4) - 1;
    }
}

/// public methods
impl DeviceEnumerator {
    /// create a default instance of DeviceEnumerator
    pub fn new() -> DeviceEnumerator {
        DeviceEnumerator::default()
    }

    /// set the enumerator type
    pub fn set_enumerator_type(&mut self, etype: DeviceEnumerationType) {
        if self.etype != etype {
            self.scan_up_to_date = false;
        }

        self.etype = etype;
    }

    /// add prioritized subsystem
    pub fn add_prioritized_subsystem(&mut self, subsystem: String) -> Result<(), Error> {
        self.prioritized_subsystems.push(subsystem);
        self.scan_up_to_date = false;
        Ok(())
    }

    /// add match subsystem
    pub fn add_match_subsystem(
        &mut self,
        subsystem: String,
        whether_match: bool,
    ) -> Result<(), Error> {
        match whether_match {
            true => {
                self.match_subsystem.insert(subsystem);
            }
            false => {
                self.not_match_subsystem.insert(subsystem);
            }
        }
        self.scan_up_to_date = false;
        Ok(())
    }

    /// add match sysattr
    pub fn add_match_sysattr(
        &mut self,
        sysattr: String,
        value: String,
        whether_match: bool,
    ) -> Result<(), Error> {
        match whether_match {
            true => {
                match self.match_sysattr.get_mut(&sysattr) {
                    Some(set) => set.insert(value),
                    None => {
                        self.match_sysattr
                            .insert(sysattr.to_string(), HashSet::new());
                        self.match_sysattr.get_mut(&sysattr).unwrap().insert(value)
                    }
                };
            }
            false => {
                match self.not_match_sysattr.get_mut(&sysattr) {
                    Some(set) => set.insert(value),
                    None => {
                        self.not_match_sysattr
                            .insert(sysattr.to_string(), HashSet::new());
                        self.not_match_sysattr
                            .get_mut(&sysattr)
                            .unwrap()
                            .insert(value)
                    }
                };
            }
        };

        self.scan_up_to_date = false;
        Ok(())
    }

    /// add match property
    pub fn add_match_property(&mut self, property: String, value: String) -> Result<(), Error> {
        self.match_property.insert(property, value);
        self.scan_up_to_date = false;
        Ok(())
    }

    /// add match sysname
    pub fn add_match_sysname(&mut self, sysname: String, whether_match: bool) -> Result<(), Error> {
        match whether_match {
            true => {
                self.match_sysname.insert(sysname);
            }
            false => {
                self.not_match_sysname.insert(sysname);
            }
        }
        self.scan_up_to_date = false;
        Ok(())
    }

    /// add match sysname
    pub fn add_match_tag(&mut self, tag: String) -> Result<(), Error> {
        self.match_tag.insert(tag);
        self.scan_up_to_date = false;
        Ok(())
    }

    /// add match parent
    pub fn add_match_parent_incremental(&mut self, parent: &Device) -> Result<(), Error> {
        let syspath = err_wrapper!(parent.get_syspath(), "add_match_parent_incremental")?;
        self.match_parent.insert(syspath.to_string());
        self.scan_up_to_date = false;
        Ok(())
    }

    /// add match parent
    pub fn add_match_parent(&mut self, parent: &Device) -> Result<(), Error> {
        self.match_parent.clear();
        self.add_match_parent_incremental(parent)?;
        Ok(())
    }

    /// allow uninitialized
    pub fn allow_uninitialized(&mut self) -> Result<(), Error> {
        self.match_initialized = MatchInitializedType::ALL;
        self.scan_up_to_date = false;
        Ok(())
    }

    /// add match is initialized
    pub fn add_match_is_initialized(&mut self, mtype: MatchInitializedType) -> Result<(), Error> {
        self.match_initialized = mtype;
        self.scan_up_to_date = false;
        Ok(())
    }
}

/// internal methods
impl DeviceEnumerator {
    /// sort devices in order
    pub(crate) fn sort_devices(&mut self) -> Result<(), Error> {
        if self.sorted {
            return Ok(());
        }

        let mut devices = Vec::<Arc<Mutex<Device>>>::new();
        let mut n_sorted = 0;

        for prioritized_subsystem in self.prioritized_subsystems.iter() {
            // find all devices with the prioritized subsystem
            loop {
                let m = devices.len();
                // find a device with the prioritized subsystem
                for (syspath, device) in self.devices_by_syspath.iter() {
                    let mut binding = device.lock().unwrap();
                    let subsys = match binding.borrow_mut().get_subsystem() {
                        Ok(ret) => ret,
                        Err(_) => {
                            continue;
                        }
                    };
                    if !subsys.eq(prioritized_subsystem) {
                        continue;
                    }

                    devices.push(device.clone());

                    let mut path = Path::new(syspath);
                    // the ancestors of this device should also be found out
                    while let Some(dir) = path.parent() {
                        let dir_str = dir.to_str().unwrap();
                        match self.devices_by_syspath.get(&dir_str.to_string()) {
                            Some(d) => devices.push(d.clone()),
                            None => break,
                        }

                        path = dir;
                    }

                    break;
                }

                // remove already sorted devices from the hashmap (self.devices_by_syspath)
                // avoid get repeated devices from the hashmap later
                for device in devices.iter().skip(m) {
                    let binding = device.lock().unwrap();
                    let syspath = err_wrapper!(binding.get_syspath(), "sort_devices")?;

                    self.devices_by_syspath.remove(syspath);
                }

                if m == devices.len() {
                    break;
                }
            }
            devices[n_sorted..].sort_by(|a, b| {
                device_compare(
                    a.lock().unwrap().borrow_mut(),
                    b.lock().unwrap().borrow_mut(),
                )
            });
            n_sorted = devices.len();
        }

        // get the rest unsorted devices in the hashmap
        for (_, device) in self.devices_by_syspath.iter() {
            devices.push(device.clone());
        }

        // the sorted devices are removed from the hashmap previously
        // insert them back
        for device in devices[..n_sorted].iter() {
            self.devices_by_syspath.insert(
                device
                    .lock()
                    .unwrap()
                    .borrow_mut()
                    .get_syspath()
                    .unwrap()
                    .to_string(),
                device.clone(),
            );
        }

        devices[n_sorted..].sort_by(|a, b| {
            device_compare(
                a.lock().unwrap().borrow_mut(),
                b.lock().unwrap().borrow_mut(),
            )
        });
        self.devices = devices;
        self.sorted = true;
        // self.current_device_index = 0;
        Ok(())
    }

    /// add device
    pub(crate) fn add_device(&mut self, device: Arc<Mutex<Device>>) -> Result<bool, Error> {
        let mut binding = device.lock().unwrap();
        let syspath = err_wrapper!(binding.borrow_mut().get_syspath(), "add_device")?;

        match self
            .devices_by_syspath
            .insert(syspath.to_string(), device.clone())
        {
            Some(_) => self.sorted = false,
            None => {
                // return Ok(false) if the hashmap already exists the device
                return Ok(false);
            }
        }

        self.sorted = false;

        // return Ok(true) if the hashmap is updated
        Ok(true)
    }

    /// check whether a device matches at least one property
    pub(crate) fn match_property(&self, device: &Device) -> Result<bool, Error> {
        if self.match_property.is_empty() {
            return Ok(true);
        }

        for (property_pattern, value_pattern) in self.match_property.iter() {
            for (property, value) in device.properties.iter() {
                if !self
                    .pattern_match(property_pattern, property)
                    .map_err(|e| Error::Nix {
                        msg: format!("match_property failed: pattern_match property ({})", e),
                        source: e.get_errno(),
                    })?
                {
                    continue;
                }

                if self
                    .pattern_match(value_pattern, value)
                    .map_err(|e| Error::Nix {
                        msg: format!("match_property failed: pattern_match value ({})", e),
                        source: e.get_errno(),
                    })?
                {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// check whether the tag of a device matches
    pub(crate) fn match_tag(&self, _device: &Device) -> Result<bool, Error> {
        // todo!("device database is not available for tag");
        Ok(true)
    }

    /// check whether the sysname of a device matches
    pub(crate) fn match_sysname(&self, sysname: &str) -> Result<bool, Error> {
        self.set_pattern_match(&self.match_sysname, &self.not_match_sysname, sysname)
    }

    /// check whether the initialized state of a device matches
    pub(crate) fn match_initialized(&self, _device: &Device) -> Result<bool, Error> {
        // todo!("device database is not available for initialized");
        Ok(true)
    }

    /// check whether the subsystem of a device matches
    pub(crate) fn match_subsystem(&self, subsystem: &str) -> Result<bool, Error> {
        self.set_pattern_match(&self.match_subsystem, &self.not_match_subsystem, subsystem)
    }

    /// check whether a device matches parent
    pub(crate) fn match_parent(&self, device: &Device) -> Result<bool, Error> {
        if self.match_parent.is_empty() {
            return Ok(true);
        }

        for parent in self.match_parent.iter() {
            if device.syspath.starts_with(parent) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// check whether the sysattrs of a device matches
    pub(crate) fn match_sysattr(&self, device: &Device) -> Result<bool, Error> {
        for (sysattr, patterns) in self.match_sysattr.iter() {
            if !self.match_sysattr_value(device, sysattr, patterns)? {
                return Ok(false);
            }
        }

        for (sysattr, patterns) in self.not_match_sysattr.iter() {
            if self.match_sysattr_value(device, sysattr, patterns)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// check whether the value of specific sysattr of a device matches
    pub(crate) fn match_sysattr_value(
        &self,
        _device: &Device,
        _sysattr: &str,
        _patterns: &HashSet<String>,
    ) -> Result<bool, Error> {
        todo!("Device::get_sysattr_value has not been implemented.");
        // Ok(false)
    }

    /// check whether a device matches conditions according to flags
    pub(crate) fn test_matches(
        &self,
        device: &mut Device,
        flags: MatchFlag,
    ) -> Result<bool, Error> {
        if (flags & MatchFlag::SYSNAME).bits() != 0 {
            match self.match_sysname(device.get_sysname().unwrap()) {
                Ok(ret) => match ret {
                    true => {}
                    false => return Ok(false),
                },
                Err(e) => {
                    return Err(Error::Nix {
                        msg: format!("test_matches failed: match_sysname ({})", e),
                        source: e.get_errno(),
                    })
                }
            }
        }

        if (flags & MatchFlag::SUBSYSTEM).bits() != 0 {
            let subsystem = match device.get_subsystem() {
                Ok(s) => s,
                Err(e) => {
                    if e.get_errno() == Errno::ENOENT {
                        return Ok(false);
                    }

                    return Err(Error::Nix {
                        msg: format!("test_matches failed: get_subsystem ({})", e),
                        source: e.get_errno(),
                    });
                }
            };

            match self.match_subsystem(&subsystem) {
                Ok(ret) => match ret {
                    true => {}
                    false => return Ok(false),
                },
                Err(e) => {
                    return Err(Error::Nix {
                        msg: format!("test_matches failed: match_subsystem ({})", e),
                        source: e.get_errno(),
                    });
                }
            }
        }

        if (flags & MatchFlag::PARENT).bits() != 0 {
            match self.match_parent(device) {
                Ok(ret) => match ret {
                    true => {}
                    false => return Ok(false),
                },
                Err(e) => {
                    return Err(Error::Nix {
                        msg: format!("test_matches failed: match_parent ({})", e),
                        source: e.get_errno(),
                    });
                }
            }
        }

        if (flags & MatchFlag::TAG).bits() != 0 {
            match self.match_tag(device) {
                Ok(ret) => match ret {
                    true => {}
                    false => return Ok(false),
                },
                Err(e) => {
                    return Err(Error::Nix {
                        msg: format!("test_matches failed: match_tag ({})", e),
                        source: e.get_errno(),
                    });
                }
            }
        }

        match self.match_initialized(device) {
            Ok(ret) => match ret {
                true => {}
                false => return Ok(false),
            },
            Err(e) => {
                return Err(Error::Nix {
                    msg: format!("test_matches failed: match_initialized ({})", e),
                    source: e.get_errno(),
                });
            }
        }

        match self.match_property(device) {
            Ok(ret) => match ret {
                true => {}
                false => return Ok(false),
            },
            Err(e) => {
                return Err(Error::Nix {
                    msg: format!("test_matches failed: match_property ({})", e),
                    source: e.get_errno(),
                });
            }
        }

        match self.match_sysattr(device) {
            Ok(ret) => match ret {
                true => {}
                false => return Ok(false),
            },
            Err(e) => {
                return Err(Error::Nix {
                    msg: format!("test_matches failed: match_sysattr ({})", e),
                    source: e.get_errno(),
                });
            }
        }

        Ok(true)
    }

    /// add parent device
    pub(crate) fn add_parent_devices(
        &mut self,
        device: Arc<Mutex<Device>>,
        flags: MatchFlag,
    ) -> Result<(), Error> {
        let mut d = device;
        loop {
            let parent = match d.lock().unwrap().borrow_mut().get_parent() {
                Ok(ret) => ret.clone(),
                Err(e) => {
                    // reach the top
                    if e.get_errno() == Errno::ENOENT {
                        break;
                    }

                    return Err(Error::Nix {
                        msg: format!("add_parent_devices failed: get_parent ({})", e),
                        source: e.get_errno(),
                    });
                }
            };

            d = parent.clone();

            if !self
                .test_matches(parent.lock().unwrap().borrow_mut(), flags)
                .map_err(|e| Error::Nix {
                    msg: format!("add_parent_devices failed: test_matches ({})", e),
                    source: e.get_errno(),
                })?
            {
                continue;
            }

            if !self.add_device(parent.clone()).map_err(|e| Error::Nix {
                msg: format!("add_parent_devices failed: add_device ({})", e),
                source: e.get_errno(),
            })? {
                break;
            }
        }
        Ok(())
    }

    /// scan directory and add all matched devices
    /// basedir should be subdirectory under /sys/
    /// e.g., /devices/...
    pub(crate) fn scan_dir_and_add_devices(
        &mut self,
        basedir: String,
        mut subdirs: Vec<String>,
    ) -> Result<(), Error> {
        let mut ret = Result::<(), Error>::Ok(());
        let mut path: Vec<String> = vec!["/sys".to_string(), basedir];
        path.append(&mut subdirs);
        let path = path.join("/");
        let path = match Path::new(&path).canonicalize().map_err(|e| Error::Nix {
            msg: format!(
                "scan_dir_and_add_devices failed: canonicalize {} ({})",
                path, e
            ),
            source: Errno::from_i32(e.raw_os_error().unwrap_or_default()),
        }) {
            Ok(p) => p,
            Err(e) => {
                if e.get_errno() == Errno::ENOENT {
                    return Ok(());
                }
                return Err(e);
            }
        };

        let entries = std::fs::read_dir(path).unwrap();
        for entry in entries {
            if let Err(e) = entry {
                ret = Err(Error::Nix {
                    msg: format!("scan_dir_and_add_devices failed: read entries ({})", e),
                    source: Errno::from_i32(e.raw_os_error().unwrap_or_default()),
                });
                continue;
            }

            let entry = entry.unwrap();

            if !relevant_sysfs_subdir(&entry) {
                continue;
            }

            if !self.match_sysname(entry.file_name().to_str().unwrap_or_default())? {
                continue;
            }

            let syspath = match entry.path().canonicalize() {
                Ok(ret) => ret,
                Err(e) => {
                    if let Some(errno) = e.raw_os_error() {
                        if errno != libc::ENODEV {
                            ret = Err(Error::Nix {
                                msg: format!(
                                    "scan_dir_and_add_devices failed: canonicalize {:?} ({})",
                                    entry, e
                                ),
                                source: Errno::from_i32(errno),
                            });
                        }
                    }
                    continue;
                }
            };

            let device = match Device::from_syspath(
                syspath.to_str().unwrap_or_default().to_string(),
                true,
            ) {
                Ok(ret) => Arc::new(Mutex::new(ret)),
                Err(e) => {
                    if e.get_errno() != nix::errno::Errno::ENODEV {
                        ret = Err(Error::Nix {
                            msg: format!("scan_dir_and_add_devices failed: from_syspath ({})", e),
                            source: e.get_errno(),
                        });
                    }
                    continue;
                }
            };

            match self.test_matches(
                device.lock().unwrap().borrow_mut(),
                MatchFlag::ALL & !MatchFlag::SYSNAME,
            ) {
                Ok(true) => {}
                Ok(false) => {
                    continue;
                }
                Err(e) => {
                    ret = Err(Error::Nix {
                        msg: format!("scan_dir_and_add_devices failed: test_matches ({})", e),
                        source: e.get_errno(),
                    });
                    continue;
                }
            };

            match self.add_device(device.clone()) {
                Ok(_) => {}
                Err(e) => {
                    ret = Err(Error::Nix {
                        msg: format!("scan_dir_and_add_devices failed: add_device ({})", e),
                        source: e.get_errno(),
                    });
                }
            };

            // also include all potentially matching parent devices.
            match self.add_parent_devices(device.clone(), MatchFlag::ALL) {
                Ok(_) => {}
                Err(e) => {
                    ret = Err(Error::Nix {
                        msg: format!(
                            "scan_dir_and_add_devices failed: add_parent_devices ({})",
                            e
                        ),
                        source: e.get_errno(),
                    });
                }
            };
        }

        ret
    }

    /// scan directory
    pub(crate) fn scan_dir(
        &mut self,
        basedir: String,
        subdir: Option<String>,
        subsystem: Option<String>,
    ) -> Result<(), Error> {
        let path_str = "/sys/".to_string() + basedir.as_str();
        let path = match Path::new(&path_str).canonicalize() {
            Ok(ret) => ret,
            Err(e) => {
                return Err(Error::Nix {
                    msg: format!("scan_dir failed: canonicalize {} ({})", basedir, e),
                    source: Errno::EINVAL,
                });
            }
        };

        let dir = std::fs::read_dir(path);
        if let Err(e) = dir {
            if e.raw_os_error().unwrap_or_default() == libc::ENOENT {
                return Ok(());
            } else {
                return Err(Error::Nix {
                    msg: format!("scan_dir failed: read_dir {}", basedir),
                    source: Errno::from_i32(e.raw_os_error().unwrap_or_default()),
                });
            };
        }

        let mut ret = Result::<(), Error>::Ok(());

        for entry in dir.unwrap() {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    ret = Err(Error::Nix {
                        msg: format!("scan_dir failed: read entries from directory {}", path_str),
                        source: Errno::from_i32(e.raw_os_error().unwrap_or_default()),
                    });
                    continue;
                }
            };

            if !relevant_sysfs_subdir(&entry) {
                continue;
            }

            match self.match_subsystem(
                subsystem
                    .clone()
                    .unwrap_or_else(|| entry.file_name().to_str().unwrap_or_default().to_string())
                    .as_str(),
            ) {
                Ok(false) | Err(_) => continue,
                Ok(true) => {}
            }

            let mut subdirs = vec![entry.file_name().to_str().unwrap_or_default().to_string()];

            if subdir.is_some() {
                subdirs.push(subdir.clone().unwrap());
            }

            if let Err(e) = self.scan_dir_and_add_devices(basedir.clone(), subdirs) {
                ret = Err(Error::Nix {
                    msg: format!("scan_dir failed: scan_dir_and_add_devices ({})", e),
                    source: e.get_errno(),
                });
            }
        }
        ret
    }

    /// scan devices for a single tag
    pub(crate) fn scan_devices_tag(&mut self, _tag: String) -> Result<(), Error> {
        todo!("scan_devices_tag has not been implemented.");
        // Ok(())
    }

    /// scan devices tags
    pub(crate) fn scan_devices_tags(&mut self) -> Result<(), Error> {
        let mut ret = Result::<(), Error>::Ok(());
        let tag_clone = self.match_tag.clone();
        for tag in tag_clone {
            if let Err(e) = self.scan_devices_tag(tag) {
                ret = Err(Error::Nix {
                    msg: format!("scan_devices_tags failed: scan_devices_tag ({})", e),
                    source: e.get_errno(),
                });
            }
        }
        ret
    }

    /// parent add child
    pub(crate) fn parent_add_child(
        &mut self,
        path: String,
        flags: MatchFlag,
    ) -> Result<bool, Error> {
        let device = Arc::new(Mutex::new(Device::from_syspath(path, true)?));

        if !self.test_matches(device.lock().unwrap().borrow_mut(), flags)? {
            return Ok(false);
        }

        self.add_device(device)
    }

    /// parent crawl children
    pub(crate) fn parent_crawl_children(
        &mut self,
        path: String,
        stack: &mut HashSet<String>,
    ) -> Result<(), Error> {
        let entries = match std::fs::read_dir(path.clone()) {
            Ok(ret) => ret,
            Err(e) => {
                let errno = e.raw_os_error().unwrap_or_default();
                if errno == libc::ENOENT {
                    return Ok(());
                } else {
                    return Err(Error::Nix {
                        msg: format!("parent_crawl_children failed: read directory {}", path),
                        source: Errno::from_i32(errno),
                    });
                }
            }
        };
        let mut ret = Result::<(), Error>::Ok(());
        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    ret = Err(Error::Nix {
                        msg: format!(
                            "parent_crawl_children failed: read entries under {}",
                            path.clone()
                        ),
                        source: Errno::from_i32(e.raw_os_error().unwrap_or_default()),
                    });
                    continue;
                }
            };
            let file_name = entry.file_name();
            let file_name = file_name.to_str().unwrap_or_default();
            if file_name.is_empty() || file_name.starts_with('.') {
                continue;
            }

            let file_type = entry.file_type();
            if file_type.is_err() || !file_type.unwrap().is_dir() {
                continue;
            }

            let entry_path = match entry.path().canonicalize() {
                Ok(p) => p,
                Err(e) => {
                    ret = Err(Error::Nix {
                        msg: format!("parent_crawl_children failed: canonicalize {:?}", entry),
                        source: Errno::from_i32(e.raw_os_error().unwrap_or_default()),
                    });
                    continue;
                }
            }
            .to_str()
            .unwrap_or_default()
            .to_string();

            if let Ok(true) = self.match_sysname(file_name) {
                if let Err(e) = self.parent_add_child(
                    entry_path.clone(),
                    MatchFlag::ALL & !(MatchFlag::SYSNAME | MatchFlag::PARENT),
                ) {
                    ret = Err(Error::Nix {
                        msg: format!("parent_crawl_children failed: match_sysname ({})", e),
                        source: e.get_errno(),
                    });
                };
            }

            let _ = stack.insert(entry_path);
        }

        ret
    }

    /// scan device children
    pub(crate) fn scan_devices_children(&mut self) -> Result<(), Error> {
        let mut ret = Result::<(), Error>::Ok(());
        let mut stack = HashSet::<String>::new();

        // copy self.match_parent to deal with rust's abandon on mutable reference along with unmutable reference
        let match_parent_copy = self.match_parent.clone();
        for path in match_parent_copy.iter() {
            if let Err(e) = self
                .parent_add_child(path.clone(), MatchFlag::ALL & !MatchFlag::PARENT)
                .map(|_| ())
            {
                ret = Err(Error::Nix {
                    msg: format!(
                        "scan_devices_children failed: scan_devices_children ({})",
                        e
                    ),
                    source: e.get_errno(),
                })
            }

            if let Err(e) = self.parent_crawl_children(path.clone(), &mut stack) {
                ret = Err(Error::Nix {
                    msg: format!(
                        "scan_devices_children failed: parent_crawl_children ({})",
                        e
                    ),
                    source: e.get_errno(),
                })
            }
        }

        while let Some(path) = stack.iter().next() {
            let p = path.clone();
            stack.remove(&p);

            if let Err(e) = self.parent_crawl_children(p, &mut stack) {
                ret = Err(Error::Nix {
                    msg: format!(
                        "scan_devices_children failed: parent_crawl_children ({})",
                        e
                    ),
                    source: e.get_errno(),
                })
            }
        }

        ret
    }

    /// scan all devices
    pub(crate) fn scan_devices_all(&mut self) -> Result<(), Error> {
        let mut ret = Result::<(), Error>::Ok(());

        if let Err(e) = self.scan_dir("bus".to_string(), Some("devices".to_string()), None) {
            ret = Err(Error::Nix {
                msg: format!("scan_devices_all failed: scan_dir /sys/bus ({})", e),
                source: e.get_errno(),
            })
        }

        if let Err(e) = self.scan_dir("class".to_string(), None, None) {
            ret = Err(Error::Nix {
                msg: format!("scan_devices_all failed: scan_dir /sys/class ({})", e),
                source: e.get_errno(),
            })
        }

        ret
    }

    /// scan all non devices
    pub(crate) fn scan_subsystems_all(&mut self) -> Result<(), Error> {
        let mut ret = Result::<(), Error>::Ok(());

        if self.match_subsystem("module").map_err(|e| Error::Nix {
            msg: format!("scan_subsystems_all failed: match_subsystem module ({})", e),
            source: e.get_errno(),
        })? {
            if let Err(e) = self.scan_dir_and_add_devices("module".to_string(), vec![]) {
                ret = Err(Error::Nix {
                    msg: format!(
                        "scan_subsystems_all failed: scan_dir_and_add_devices module ({})",
                        e
                    ),
                    source: e.get_errno(),
                })
            }
        }

        if self.match_subsystem("subsystem").map_err(|e| Error::Nix {
            msg: format!(
                "scan_subsystems_all failed: match_subsystem subsystem ({})",
                e
            ),
            source: e.get_errno(),
        })? {
            if let Err(e) = self.scan_dir_and_add_devices("bus".to_string(), vec![]) {
                ret = Err(Error::Nix {
                    msg: format!(
                        "scan_subsystems_all failed: scan_dir_and_add_devices subsystem ({})",
                        e
                    ),
                    source: e.get_errno(),
                })
            }
        }

        if self.match_subsystem("drivers").map_err(|e| Error::Nix {
            msg: format!(
                "scan_subsystems_all failed: match_subsystem drivers ({})",
                e
            ),
            source: e.get_errno(),
        })? {
            if let Err(e) = self.scan_dir(
                "bus".to_string(),
                Some("drivers".to_string()),
                Some("drivers".to_string()),
            ) {
                ret = Err(Error::Nix {
                    msg: format!(
                        "scan_subsystems_all failed: scan_dir_and_add_devices drivers ({})",
                        e
                    ),
                    source: e.get_errno(),
                })
            }
        }

        ret
    }

    /// scan devices
    pub(crate) fn scan_devices(&mut self) -> Result<(), Error> {
        if self.scan_up_to_date && self.etype == DeviceEnumerationType::Devices {
            return Ok(());
        }

        // clean up old devices
        self.devices_by_syspath.clear();
        self.devices.clear();

        let mut ret = Result::<(), Error>::Ok(());

        if !self.match_tag.is_empty() {
            if let Err(e) = self.scan_devices_tags() {
                ret = Err(Error::Nix {
                    msg: format!("scan_devices failed: scan_devices_tags ({})", e),
                    source: e.get_errno(),
                })
            }
        } else if !self.match_parent.is_empty() {
            if let Err(e) = self.scan_devices_children() {
                ret = Err(Error::Nix {
                    msg: format!("scan_devices failed: scan_devices_children ({})", e),
                    source: e.get_errno(),
                })
            }
        } else if let Err(e) = self.scan_devices_all() {
            ret = Err(Error::Nix {
                msg: format!("scan_devices failed: scan_devices_all ({})", e),
                source: e.get_errno(),
            })
        }

        self.scan_up_to_date = true;
        self.etype = DeviceEnumerationType::Devices;

        ret
    }

    /// scan subsystems
    pub(crate) fn scan_subsystems(&mut self) -> Result<(), Error> {
        if self.scan_up_to_date && self.etype == DeviceEnumerationType::Subsystems {
            return Ok(());
        }

        // clean up old devices
        self.devices_by_syspath.clear();
        self.devices.clear();

        let ret = self.scan_subsystems_all().map_err(|e| Error::Nix {
            msg: format!("scan_subsystems failed: scan_subsystems_all ({})", e),
            source: e.get_errno(),
        });

        self.scan_up_to_date = true;
        self.etype = DeviceEnumerationType::Subsystems;

        ret
    }

    /// scan devices and subsystems
    pub(crate) fn scan_devices_and_subsystems(&mut self) -> Result<(), Error> {
        if self.scan_up_to_date && self.etype == DeviceEnumerationType::All {
            return Ok(());
        }

        // clean up old devices
        self.devices_by_syspath.clear();
        self.devices.clear();

        let mut ret = Result::<(), Error>::Ok(());

        if !self.match_tag.is_empty() {
            if let Err(e) = self.scan_devices_tags() {
                ret = Err(Error::Nix {
                    msg: format!(
                        "scan_devices_and_subsystems failed: scan_devices_tags ({})",
                        e
                    ),
                    source: e.get_errno(),
                })
            }
        } else if !self.match_parent.is_empty() {
            if let Err(e) = self.scan_devices_children() {
                ret = Err(Error::Nix {
                    msg: format!(
                        "scan_devices_and_subsystems failed: scan_devices_children ({})",
                        e
                    ),
                    source: e.get_errno(),
                })
            }
        } else {
            if let Err(e) = self.scan_devices_all() {
                ret = Err(Error::Nix {
                    msg: format!(
                        "scan_devices_and_subsystems failed: scan_devices_all ({})",
                        e
                    ),
                    source: e.get_errno(),
                })
            }

            if let Err(e) = self.scan_subsystems_all() {
                ret = Err(Error::Nix {
                    msg: format!(
                        "scan_devices_and_subsystems failed: scan_subsystems_all ({})",
                        e
                    ),
                    source: e.get_errno(),
                })
            }
        }

        self.scan_up_to_date = true;
        self.etype = DeviceEnumerationType::All;

        ret
    }

    /// get sorted devices
    #[allow(dead_code)]
    pub(crate) fn get_devices(&mut self) -> Option<&Vec<Arc<Mutex<Device>>>> {
        if !self.scan_up_to_date {
            return None;
        }

        if self.sort_devices().is_err() {
            return None;
        }

        Some(&self.devices)
    }

    /// pattern match
    /// if the enumerator filter is of Glob type, use unix glob-fnmatch to check whether match
    /// if the enumerator filter is of Regular type, use typical regular expression to check whether match
    pub(crate) fn pattern_match(&self, pattern: &str, value: &str) -> Result<bool, Error> {
        let regex: regex::Regex = match self.filter_type {
            FilterType::Glob => {
                match fnmatch_regex::glob_to_regex(pattern) {
                    Ok(ret) => ret,
                    Err(e) => {
                        return Err(Error::Nix {
                        msg: format!("pattern_match failed: change glob-fnmatch to regular expression {}", e),
                        source: Errno::EINVAL,
                    });
                    }
                }
            }
            FilterType::_Regular => match pattern.parse() {
                Ok(ret) => ret,
                Err(e) => {
                    return Err(Error::Nix {
                        msg: format!(
                            "pattern_match failed: parse string to regular expression: {}",
                            e
                        ),
                        source: Errno::EINVAL,
                    });
                }
            },
        };

        Ok(regex.is_match(value))
    }

    /// if any exclude pattern matches, return false
    /// if include pattern set is empty, return true
    /// if any include pattern matches, return true, else return false
    pub(crate) fn set_pattern_match(
        &self,
        include_pattern_set: &HashSet<String>,
        exclude_pattern_set: &HashSet<String>,
        value: &str,
    ) -> Result<bool, Error> {
        for pattern in exclude_pattern_set.iter() {
            if self.pattern_match(pattern, value).map_err(|e| Error::Nix {
                msg: format!("set_pattern_match failed: pattern_match exclude ({})", e),
                source: e.get_errno(),
            })? {
                return Ok(false);
            }
        }

        if include_pattern_set.is_empty() {
            return Ok(true);
        }

        for pattern in include_pattern_set.iter() {
            if self.pattern_match(pattern, value).map_err(|e| Error::Nix {
                msg: format!("set_pattern_match failed: pattern_match include ({})", e),
                source: e.get_errno(),
            })? {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use crate::device_enumerator::DeviceEnumerationType;

    use super::DeviceEnumerator;

    #[ignore]
    #[test]
    fn test_enumerator_inialize() {
        let mut enumerator = DeviceEnumerator::new();
        enumerator
            .add_match_subsystem(String::from("block"), true)
            .unwrap();
        enumerator
            .add_match_subsystem(String::from("char"), false)
            .unwrap();
        enumerator
            .add_match_sysattr(String::from("dev"), String::from("8:0"), true)
            .unwrap();
        enumerator
            .add_match_sysattr(String::from("ro"), String::from("1"), false)
            .unwrap();
        enumerator
            .add_match_property(String::from("DEVTYPE"), String::from("block"))
            .unwrap();
        enumerator
            .add_match_property(String::from("DEVTYPE"), String::from("char"))
            .unwrap();
        enumerator
            .add_match_sysname(String::from("sda"), true)
            .unwrap();
        enumerator
            .add_match_sysname(String::from("sdb"), false)
            .unwrap();
    }

    #[test]
    fn test_scan_devices() {
        let mut enumerator = DeviceEnumerator::new();
        enumerator.set_enumerator_type(DeviceEnumerationType::Devices);

        for i in enumerator.iter_mut() {
            i.lock().unwrap().get_devpath().unwrap();
        }
    }

    #[test]
    fn test_scan_subsystems() {
        let mut enumerator = DeviceEnumerator::new();
        enumerator.set_enumerator_type(DeviceEnumerationType::Subsystems);

        for i in enumerator.iter_mut() {
            i.lock()
                .expect("device is locked somewhere else")
                .get_devpath()
                .expect("can not get the devpath");
        }
    }

    #[test]
    fn test_scan_devices_and_subsystems() {
        let mut enumerator = DeviceEnumerator::new();
        enumerator.set_enumerator_type(DeviceEnumerationType::All);

        for i in enumerator.iter_mut() {
            i.lock()
                .expect("device is locked somewhere else")
                .get_devpath()
                .expect("can not get the devpath");
        }
    }
}
