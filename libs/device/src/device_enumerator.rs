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
use crate::{device::Device, error::*, utils::*, TAGS_BASE_DIR};
use bitflags::bitflags;
//use fnmatch_sys::fnmatch;
use basic::string::pattern_match;
use nix::errno::Errno;
use snafu::ResultExt;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    iter::Iterator,
    path::Path,
    rc::Rc,
};

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
pub struct DeviceEnumerator {
    /// enumerator type
    pub(crate) etype: RefCell<DeviceEnumerationType>,
    /// key: syspath, value: device
    pub(crate) devices_by_syspath: RefCell<HashMap<String, Rc<Device>>>,
    /// sorted device vector
    pub(crate) devices: RefCell<Vec<Rc<Device>>>,

    /// whether enumerator is up to date
    pub(crate) scan_up_to_date: RefCell<bool>,
    /// whether devices are sorted
    pub(crate) sorted: RefCell<bool>,

    /// prioritized subsystems
    pub(crate) prioritized_subsystems: RefCell<Vec<String>>,

    /// match subsystem
    pub(crate) match_subsystem: RefCell<HashSet<String>>,
    /// do not match subsystem
    pub(crate) not_match_subsystem: RefCell<HashSet<String>>,

    /// match sysattr
    /// key: sysattr, value: match value
    pub(crate) match_sysattr: RefCell<HashMap<String, String>>,
    /// do not match sysattr
    /// key: sysattr, value: match value
    pub(crate) not_match_sysattr: RefCell<HashMap<String, String>>,

    /// match property
    /// key: property, value: match value
    pub(crate) match_property: RefCell<HashMap<String, String>>,

    /// match sysname
    pub(crate) match_sysname: RefCell<HashSet<String>>,
    /// not match sysname
    pub(crate) not_match_sysname: RefCell<HashSet<String>>,

    /// match tag
    pub(crate) match_tag: RefCell<HashSet<String>>,

    /// match parent
    pub(crate) match_parent: RefCell<HashSet<String>>,

    /// how to match device
    pub(crate) match_initialized: RefCell<MatchInitializedType>,

    /// the base directory path to contain runtime temporary files of device database, tags, etc.
    pub(crate) base_path: RefCell<String>,
}

impl Default for DeviceEnumerator {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceEnumerator {
    /// create a default instance of DeviceEnumerator
    pub fn new() -> Self {
        Self {
            etype: RefCell::new(DeviceEnumerationType::All),
            devices_by_syspath: RefCell::new(HashMap::new()),
            devices: RefCell::new(Vec::new()),
            scan_up_to_date: RefCell::new(false),
            sorted: RefCell::new(false),
            prioritized_subsystems: RefCell::new(Vec::new()),
            match_subsystem: RefCell::new(HashSet::new()),
            not_match_subsystem: RefCell::new(HashSet::new()),
            match_sysattr: RefCell::new(HashMap::new()),
            not_match_sysattr: RefCell::new(HashMap::new()),
            match_property: RefCell::new(HashMap::new()),
            match_sysname: RefCell::new(HashSet::new()),
            not_match_sysname: RefCell::new(HashSet::new()),
            match_tag: RefCell::new(HashSet::new()),
            match_parent: RefCell::new(HashSet::new()),
            match_initialized: RefCell::new(MatchInitializedType::ALL),
            base_path: RefCell::new(crate::DEFAULT_BASE_DIR.to_string()),
        }
    }
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
    pub fn iter(&mut self) -> DeviceEnumeratorIter<'_> {
        DeviceEnumeratorIter {
            current_device_index: 0,
            enumerator: self,
        }
    }
}

impl Iterator for DeviceEnumeratorIter<'_> {
    type Item = Rc<Device>;

    /// iterate over the devices or subsystems according to the enumerator type
    fn next(&mut self) -> Option<Self::Item> {
        match self.enumerator.etype.clone().take() {
            DeviceEnumerationType::Devices => {
                let scan_up_to_date = *self.enumerator.scan_up_to_date.borrow();
                if !scan_up_to_date && self.enumerator.scan_devices().is_err() {
                    return None;
                }
            }
            DeviceEnumerationType::Subsystems => {
                let scan_up_to_date = *self.enumerator.scan_up_to_date.borrow();
                if !scan_up_to_date && self.enumerator.scan_subsystems().is_err() {
                    return None;
                }
            }
            DeviceEnumerationType::All => {
                let scan_up_to_date = *self.enumerator.scan_up_to_date.borrow();
                if !scan_up_to_date && self.enumerator.scan_devices_and_subsystems().is_err() {
                    return None;
                }
            }
        }

        let sorted = *self.enumerator.sorted.borrow();
        if !sorted && self.enumerator.sort_devices().is_err() {
            return None;
        }

        self.enumerator
            .devices
            .borrow()
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
    /// set the enumerator type
    pub fn set_enumerator_type(&mut self, etype: DeviceEnumerationType) {
        if *self.etype.borrow() != etype {
            self.scan_up_to_date.replace(false);
        }

        self.etype.replace(etype);
    }

    /// add prioritized subsystem
    pub fn add_prioritized_subsystem(&mut self, subsystem: &str) -> Result<(), Error> {
        self.prioritized_subsystems
            .borrow_mut()
            .push(subsystem.to_string());
        self.scan_up_to_date.replace(false);
        Ok(())
    }

    /// add match subsystem
    pub fn add_match_subsystem(
        &mut self,
        subsystem: &str,
        whether_match: bool,
    ) -> Result<(), Error> {
        match whether_match {
            true => {
                self.match_subsystem
                    .borrow_mut()
                    .insert(subsystem.to_string());
            }
            false => {
                self.not_match_subsystem
                    .borrow_mut()
                    .insert(subsystem.to_string());
            }
        }
        self.scan_up_to_date.replace(false);
        Ok(())
    }

    /// add match sysattr
    pub fn add_match_sysattr(
        &mut self,
        sysattr: &str,
        value: &str,
        whether_match: bool,
    ) -> Result<(), Error> {
        match whether_match {
            true => {
                self.match_sysattr
                    .borrow_mut()
                    .insert(sysattr.to_string(), value.to_string());
            }
            false => {
                self.not_match_sysattr
                    .borrow_mut()
                    .insert(sysattr.to_string(), value.to_string());
            }
        };

        self.scan_up_to_date.replace(false);
        Ok(())
    }

    /// add match property
    pub fn add_match_property(&mut self, property: &str, value: &str) -> Result<(), Error> {
        self.match_property
            .borrow_mut()
            .insert(property.to_string(), value.to_string());
        self.scan_up_to_date.replace(false);
        Ok(())
    }

    /// add match sysname
    pub fn add_match_sysname(&mut self, sysname: &str, whether_match: bool) -> Result<(), Error> {
        match whether_match {
            true => {
                self.match_sysname.borrow_mut().insert(sysname.to_string());
            }
            false => {
                self.not_match_sysname
                    .borrow_mut()
                    .insert(sysname.to_string());
            }
        }
        self.scan_up_to_date.replace(false);
        Ok(())
    }

    /// add match sysname
    pub fn add_match_tag(&mut self, tag: &str) -> Result<(), Error> {
        self.match_tag.borrow_mut().insert(tag.to_string());
        self.scan_up_to_date.replace(false);
        Ok(())
    }

    /// add match parent
    pub fn add_match_parent_incremental(&mut self, parent: &Device) -> Result<(), Error> {
        let syspath = parent.get_syspath()?;
        self.match_parent.borrow_mut().insert(syspath);
        self.scan_up_to_date.replace(false);
        Ok(())
    }

    /// add match parent
    pub fn add_match_parent(&mut self, parent: &Device) -> Result<(), Error> {
        self.match_parent.borrow_mut().clear();
        self.add_match_parent_incremental(parent)?;
        Ok(())
    }

    /// allow uninitialized
    pub fn allow_uninitialized(&mut self) -> Result<(), Error> {
        self.match_initialized.replace(MatchInitializedType::ALL);
        self.scan_up_to_date.replace(false);
        Ok(())
    }

    /// add match is initialized
    pub fn add_match_is_initialized(&mut self, mtype: MatchInitializedType) -> Result<(), Error> {
        self.match_initialized.replace(mtype);
        self.scan_up_to_date.replace(false);
        Ok(())
    }
}

/// internal methods
impl DeviceEnumerator {
    /// sort devices in order
    pub(crate) fn sort_devices(&mut self) -> Result<(), Error> {
        if *self.sorted.borrow() {
            return Ok(());
        }

        let mut devices = Vec::<Rc<Device>>::new();
        let mut n_sorted = 0;

        for prioritized_subsystem in self.prioritized_subsystems.borrow().iter() {
            // find all devices with the prioritized subsystem
            loop {
                let m = devices.len();
                // find a device with the prioritized subsystem
                for (syspath, device) in self.devices_by_syspath.borrow().iter() {
                    let subsys = match device.get_subsystem() {
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
                        match self.devices_by_syspath.borrow().get(dir_str) {
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
                    let syspath = device.get_syspath()?;

                    self.devices_by_syspath.borrow_mut().remove(&syspath);
                }

                if m == devices.len() {
                    break;
                }
            }
            devices[n_sorted..].sort_by(|a, b| device_compare(a, b));
            n_sorted = devices.len();
        }

        // get the rest unsorted devices in the hashmap
        for (_, device) in self.devices_by_syspath.borrow().iter() {
            devices.push(device.clone());
        }

        // the sorted devices are removed from the hashmap previously
        // insert them back
        for device in devices[..n_sorted].iter() {
            self.devices_by_syspath
                .borrow_mut()
                .insert(device.get_syspath().unwrap().to_string(), device.clone());
        }

        devices[n_sorted..].sort_by(|a, b| device_compare(a, b));
        self.devices.replace(devices);
        self.sorted.replace(true);

        Ok(())
    }

    /// add device
    pub(crate) fn add_device(&self, device: Rc<Device>) -> Result<bool, Error> {
        let syspath = device.get_syspath()?;

        match self.devices_by_syspath.borrow_mut().insert(syspath, device) {
            Some(_) => {
                let _ = self.sorted.replace(false);
            }
            None => {
                // return Ok(false) if the hashmap already exists the device
                return Ok(false);
            }
        }

        self.sorted.replace(false);

        // return Ok(true) if the hashmap is updated
        Ok(true)
    }

    /// check whether a device matches at least one property
    pub(crate) fn match_property(&self, device: Rc<Device>) -> Result<bool, Error> {
        if self.match_property.borrow().is_empty() {
            return Ok(true);
        }

        for (property_pattern, value_pattern) in self.match_property.borrow().iter() {
            for (property, value) in &device.property_iter() {
                if !pattern_match(property_pattern, property, 0) {
                    continue;
                }

                if pattern_match(value_pattern, value, 0) {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// check whether the tag of a device matches
    pub(crate) fn match_tag(&self, _device: Rc<Device>) -> Result<bool, Error> {
        // todo!("device database is not available for tag");
        Ok(true)
    }

    /// check whether the sysname of a device matches
    pub(crate) fn match_sysname(&self, sysname: &str) -> bool {
        self.set_pattern_match(
            &self.match_sysname.borrow(),
            &self.not_match_sysname.borrow(),
            sysname,
        )
    }

    /// check whether the initialized state of a device matches
    pub(crate) fn match_initialized(&self, _device: Rc<Device>) -> Result<bool, Error> {
        // todo!("device database is not available for initialized");
        Ok(true)
    }

    /// check whether the subsystem of a device matches
    pub(crate) fn match_subsystem(&self, subsystem: &str) -> bool {
        self.set_pattern_match(
            &self.match_subsystem.borrow(),
            &self.not_match_subsystem.borrow(),
            subsystem,
        )
    }

    /// check whether a device matches conditions according to flags
    pub(crate) fn test_matches(&self, device: Rc<Device>, flags: MatchFlag) -> Result<bool, Error> {
        if (flags & MatchFlag::SYSNAME).bits() != 0 && !self.match_sysname(&device.get_sysname()?) {
            return Ok(false);
        }

        if (flags & MatchFlag::SUBSYSTEM).bits() != 0 {
            let subsystem = match device.get_subsystem() {
                Ok(s) => s,
                Err(e) => {
                    if e.get_errno() == Errno::ENOENT {
                        return Ok(false);
                    }

                    return Err(Error::Nix {
                        msg: format!("test_matches failed: no subsystem: {}", e),
                        source: e.get_errno(),
                    });
                }
            };

            if !self.match_subsystem(&subsystem) {
                return Ok(false);
            }
        }

        if (flags & MatchFlag::PARENT).bits() != 0
            && !device.match_parent(&self.match_parent.borrow(), &HashSet::new())
        {
            return Ok(false);
        }

        if (flags & MatchFlag::TAG).bits() != 0 && !self.match_tag(device.clone())? {
            return Ok(false);
        }

        if !self.match_initialized(device.clone())? {
            return Ok(false);
        }

        if !self.match_property(device.clone())? {
            return Ok(false);
        }

        if !device.match_sysattr(
            &self.match_sysattr.borrow(),
            &self.not_match_sysattr.borrow(),
        ) {
            return Ok(false);
        }

        Ok(true)
    }

    /// add parent device
    pub(crate) fn add_parent_devices(
        &self,
        device: Rc<Device>,
        flags: MatchFlag,
    ) -> Result<(), Error> {
        let mut d = device;
        loop {
            let parent = match d.get_parent() {
                Ok(ret) => ret.clone(),
                Err(e) => {
                    // reach the top
                    if e.get_errno() == Errno::ENOENT {
                        break;
                    }

                    return Err(Error::Nix {
                        msg: format!("add_parent_devices failed: {}", e),
                        source: e.get_errno(),
                    });
                }
            };

            d = parent.clone();

            if !self.test_matches(parent.clone(), flags)? {
                continue;
            }

            if !self.add_device(parent)? {
                break;
            }
        }
        Ok(())
    }

    /// scan directory and add all matched devices
    /// basedir should be subdirectory under /sys/
    /// e.g., /devices/...
    pub(crate) fn scan_dir_and_add_devices(
        &self,
        basedir: String,
        mut subdirs: Vec<String>,
    ) -> Result<(), Error> {
        let mut ret = Result::<(), Error>::Ok(());
        let mut path: Vec<String> = vec!["/sys".to_string(), basedir];
        path.append(&mut subdirs);
        let path = path.join("/");
        let path = match Path::new(&path).canonicalize().context(Io {
            msg: format!("failed to canonicalize '{}'", path),
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
            let entry = match entry.context(Io {
                msg: "failed to read directory entry".to_string(),
            }) {
                Ok(i) => i,
                Err(e) => {
                    ret = Err(e);
                    continue;
                }
            };

            if !relevant_sysfs_subdir(&entry) {
                continue;
            }

            if !self.match_sysname(entry.file_name().to_str().unwrap_or_default()) {
                continue;
            }

            let syspath = match entry.path().canonicalize().context(Io {
                msg: format!("failed to canonicalize '{:?}'", entry.path()),
            }) {
                Ok(ret) => ret,
                Err(e) => {
                    if e.get_errno() != Errno::ENODEV {
                        ret = Err(e);
                    }
                    continue;
                }
            };

            let device = match Device::from_syspath(syspath.to_str().unwrap_or_default(), true) {
                Ok(ret) => Rc::new(ret),
                Err(e) => {
                    if e.get_errno() != nix::errno::Errno::ENODEV {
                        ret = Err(Error::Nix {
                            msg: format!("scan_dir_and_add_devices failed: {}", e),
                            source: e.get_errno(),
                        });
                    }
                    continue;
                }
            };

            match self.test_matches(device.clone(), MatchFlag::ALL & !MatchFlag::SYSNAME) {
                Ok(true) => {}
                Ok(false) => {
                    continue;
                }
                Err(e) => {
                    ret = Err(e);
                    continue;
                }
            };

            match self.add_device(device.clone()) {
                Ok(_) => {}
                Err(e) => {
                    ret = Err(Error::Nix {
                        msg: format!("scan_dir_and_add_devices failed: {}", e),
                        source: e.get_errno(),
                    });
                }
            };

            // also include all potentially matching parent devices.
            let _ = self
                .add_parent_devices(device.clone(), MatchFlag::ALL)
                .map_err(|e| {
                    ret = Err(e);
                });
        }

        ret
    }

    /// scan directory
    pub(crate) fn scan_dir(
        &self,
        basedir: String,
        subdir: Option<String>,
        subsystem: Option<String>,
    ) -> Result<(), Error> {
        let path_str = "/sys/".to_string() + basedir.as_str();
        let path = Path::new(&path_str).canonicalize().context(Io {
            msg: format!("fail to canonicalize '{}'", path_str),
        })?;

        let dir = std::fs::read_dir(path);
        if let Err(e) = dir {
            if e.raw_os_error().unwrap_or_default() == libc::ENOENT {
                return Ok(());
            } else {
                return Err(Error::Nix {
                    msg: format!("scan_dir failed: can't read directory '{}'", basedir),
                    source: Errno::from_i32(e.raw_os_error().unwrap_or_default()),
                });
            };
        }

        let mut ret = Result::<(), Error>::Ok(());

        for entry in dir.unwrap() {
            let entry = match entry.context(Io {
                msg: format!("failed to read entry under '{}'", path_str),
            }) {
                Ok(e) => e,
                Err(e) => {
                    ret = Err(e);
                    continue;
                }
            };

            if !relevant_sysfs_subdir(&entry) {
                continue;
            }

            if !self.match_subsystem(
                subsystem
                    .clone()
                    .unwrap_or_else(|| entry.file_name().to_str().unwrap_or_default().to_string())
                    .as_str(),
            ) {
                continue;
            }

            let mut subdirs = vec![entry.file_name().to_str().unwrap_or_default().to_string()];

            if subdir.is_some() {
                subdirs.push(subdir.clone().unwrap());
            }

            let _ = self
                .scan_dir_and_add_devices(basedir.clone(), subdirs)
                .map_err(|e| {
                    ret = Err(e);
                });
        }
        ret
    }

    /// scan devices for a single tag
    pub(crate) fn scan_devices_tag(&self, tag: &str) -> Result<(), Error> {
        let path = Path::new(self.base_path.borrow().as_str())
            .join(TAGS_BASE_DIR)
            .join(tag);
        let dir = std::fs::read_dir(&path);
        let tag_dir = match dir.context(Io {
            msg: format!("failed to read '{:?}'", path),
        }) {
            Ok(d) => d,
            Err(e) => {
                if e.get_errno() == Errno::ENOENT {
                    return Ok(());
                } else {
                    return Err(e);
                }
            }
        };

        /* TODO: filter away subsystems? */

        let mut ret = Ok(());
        for entry in tag_dir {
            let entry = match entry.context(Io {
                msg: format!("failed to read entry under '{:?}'", path),
            }) {
                Ok(e) => e,
                Err(e) => {
                    ret = Err(e);
                    continue;
                }
            };

            let file_name = entry.file_name().to_str().unwrap().to_string();
            if file_name.contains('.') {
                continue;
            }

            let device = match Device::from_device_id(&file_name) {
                Ok(device) => Rc::new(device),
                Err(e) => {
                    if e.get_errno() != nix::errno::Errno::ENODEV {
                        /* this is necessarily racy, so ignore missing devices */
                        ret = Err(e);
                    }
                    continue;
                }
            };

            /* Generated from tag, hence not necessary to check tag again. */
            match self.test_matches(device.clone(), MatchFlag::ALL & (!MatchFlag::TAG)) {
                Ok(flag) => {
                    if !flag {
                        continue;
                    }
                }
                Err(e) => {
                    ret = Err(e);
                    continue;
                }
            }

            if let Err(e) = self.add_device(device) {
                ret = Err(e);
                continue;
            }
        }

        ret
    }

    /// scan devices tags
    pub(crate) fn scan_devices_tags(&self) -> Result<(), Error> {
        let mut ret = Result::<(), Error>::Ok(());

        for tag in self.match_tag.borrow().iter() {
            if let Err(e) = self.scan_devices_tag(tag) {
                ret = Err(e);
            }
        }
        ret
    }

    /// parent add child
    pub(crate) fn parent_add_child(&mut self, path: &str, flags: MatchFlag) -> Result<bool, Error> {
        let device = match Device::from_syspath(path, true) {
            Ok(dev) => Rc::new(dev),
            Err(err) => {
                if err.get_errno() == nix::errno::Errno::ENODEV {
                    /* this is necessarily racy, so ignore missing devices */
                    return Ok(false);
                }
                return Err(err);
            }
        };

        if !self.test_matches(device.clone(), flags)? {
            return Ok(false);
        }

        self.add_device(device)
    }

    /// parent crawl children
    pub(crate) fn parent_crawl_children(
        &mut self,
        path: &str,
        stack: &mut HashSet<String>,
    ) -> Result<(), Error> {
        let entries = match std::fs::read_dir(path).context(Io {
            msg: format!("failed to read '{}'", path),
        }) {
            Ok(ret) => ret,
            Err(e) => {
                if e.get_errno() == Errno::ENOENT {
                    return Ok(());
                } else {
                    return Err(e);
                }
            }
        };
        let mut ret = Result::<(), Error>::Ok(());
        for entry in entries {
            let entry = match entry.context(Io {
                msg: format!("failed to read entry under'{:?}'", path),
            }) {
                Ok(e) => e,
                Err(e) => {
                    ret = Err(e);
                    continue;
                }
            };

            let file_name = entry.file_name().to_str().unwrap_or_default().to_string();
            if file_name.is_empty() || file_name.starts_with('.') {
                continue;
            }

            let file_type = entry.file_type();
            if file_type.is_err() || !file_type.unwrap().is_dir() {
                continue;
            }

            let entry_path = match entry.path().canonicalize().context(Io {
                msg: format!("fail to canonicalize '{:?}'", entry.path()),
            }) {
                Ok(p) => p.to_str().unwrap_or_default().to_string(),
                Err(e) => {
                    ret = Err(e);
                    continue;
                }
            };

            if self.match_sysname(&file_name) {
                if let Err(e) = self.parent_add_child(
                    &entry_path,
                    MatchFlag::ALL & !(MatchFlag::SYSNAME | MatchFlag::PARENT),
                ) {
                    ret = Err(e);
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
        for path in match_parent_copy.borrow().iter() {
            if let Err(e) = self
                .parent_add_child(path, MatchFlag::ALL & !MatchFlag::PARENT)
                .map(|_| ())
            {
                ret = Err(e)
            }

            if let Err(e) = self.parent_crawl_children(path, &mut stack) {
                ret = Err(e)
            }
        }

        while let Some(path) = stack.iter().next().map(|p| p.to_string()) {
            stack.remove(&path);

            if let Err(e) = self.parent_crawl_children(&path, &mut stack) {
                ret = Err(e)
            }
        }

        ret
    }

    /// scan all devices
    pub(crate) fn scan_devices_all(&mut self) -> Result<(), Error> {
        let mut ret = Result::<(), Error>::Ok(());

        if let Err(e) = self.scan_dir("bus".to_string(), Some("devices".to_string()), None) {
            ret = Err(e);
        }

        if let Err(e) = self.scan_dir("class".to_string(), None, None) {
            ret = Err(e);
        }

        ret
    }

    /// scan all non devices
    pub(crate) fn scan_subsystems_all(&mut self) -> Result<(), Error> {
        let mut ret = Result::<(), Error>::Ok(());

        if self.match_subsystem("module") {
            let _ = self
                .scan_dir_and_add_devices("module".to_string(), vec![])
                .map_err(|e| {
                    ret = Err(e);
                    log::error!("Enumerator: failed to scan modules");
                });
        }

        if self.match_subsystem("subsystem") {
            let _ = self
                .scan_dir_and_add_devices("bus".to_string(), vec![])
                .map_err(|e| {
                    ret = Err(e);
                    log::error!("Enumerator: failed to scan subsystems");
                });
        }

        if self.match_subsystem("drivers") {
            let _ = self
                .scan_dir(
                    "bus".to_string(),
                    Some("drivers".to_string()),
                    Some("drivers".to_string()),
                )
                .map_err(|e| {
                    ret = Err(e);
                    log::error!("Enumerator: failed to scan drivers");
                });
        }

        ret
    }

    /// scan devices
    pub fn scan_devices(&mut self) -> Result<(), Error> {
        if *self.scan_up_to_date.borrow() && *self.etype.borrow() == DeviceEnumerationType::Devices
        {
            return Ok(());
        }

        // clean up old devices
        self.devices_by_syspath.borrow_mut().clear();
        self.devices.borrow_mut().clear();

        let mut ret = Result::<(), Error>::Ok(());

        if !self.match_tag.borrow().is_empty() {
            let _ = self.scan_devices_tags().map_err(|e| ret = Err(e));
        } else if !self.match_parent.borrow().is_empty() {
            let _ = self.scan_devices_children().map_err(|e| ret = Err(e));
        } else if let Err(e) = self.scan_devices_all() {
            ret = Err(e);
        }

        self.scan_up_to_date.replace(true);
        self.etype.replace(DeviceEnumerationType::Devices);

        ret
    }

    /// scan subsystems
    pub(crate) fn scan_subsystems(&mut self) -> Result<(), Error> {
        if *self.scan_up_to_date.borrow()
            && *self.etype.borrow() == DeviceEnumerationType::Subsystems
        {
            return Ok(());
        }

        // clean up old devices
        self.devices_by_syspath.borrow_mut().clear();
        self.devices.borrow_mut().clear();

        let ret = self.scan_subsystems_all();

        self.scan_up_to_date.replace(true);
        self.etype.replace(DeviceEnumerationType::Subsystems);

        ret
    }

    /// scan devices and subsystems
    pub(crate) fn scan_devices_and_subsystems(&mut self) -> Result<(), Error> {
        if *self.scan_up_to_date.borrow() && *self.etype.borrow() == DeviceEnumerationType::All {
            return Ok(());
        }

        // clean up old devices
        self.devices_by_syspath.borrow_mut().clear();
        self.devices.borrow_mut().clear();

        let mut ret = Result::<(), Error>::Ok(());

        if !self.match_tag.borrow().is_empty() {
            ret = self.scan_devices_tags();
        } else if !self.match_parent.borrow().is_empty() {
            ret = self.scan_devices_children();
        } else {
            let _ = self.scan_devices_all().map_err(|e| {
                ret = Err(e);
                log::error!("Failed to scan devices.");
            });

            let _ = self.scan_subsystems_all().map_err(|e| {
                ret = Err(e);
            });
        }

        self.scan_up_to_date.replace(true);
        self.etype.replace(DeviceEnumerationType::All);

        ret
    }

    /// if any exclude pattern matches, return false
    /// if include pattern set is empty, return true
    /// if any include pattern matches, return true, else return false
    pub(crate) fn set_pattern_match(
        &self,
        include_pattern_set: &HashSet<String>,
        exclude_pattern_set: &HashSet<String>,
        value: &str,
    ) -> bool {
        for pattern in exclude_pattern_set.iter() {
            if pattern_match(pattern, value, 0) {
                return false;
            }
        }

        if include_pattern_set.is_empty() {
            return true;
        }

        for pattern in include_pattern_set.iter() {
            if pattern_match(pattern, value, 0) {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::{DeviceEnumerator, MatchInitializedType};
    use crate::{device_enumerator::DeviceEnumerationType, Device};
    use std::{collections::HashSet, rc::Rc};

    #[test]
    fn test_enumerator_inialize() {
        let mut enumerator = DeviceEnumerator::new();
        enumerator.add_match_subsystem("block", true).unwrap();
        enumerator.add_match_subsystem("char", false).unwrap();
        enumerator.add_match_sysattr("dev", "8:0", true).unwrap();
        enumerator.add_match_sysattr("ro", "1", false).unwrap();
        enumerator.add_match_property("DEVTYPE", "block").unwrap();
        enumerator.add_match_property("DEVTYPE", "char").unwrap();
        enumerator.add_match_sysname("sda", true).unwrap();
        enumerator.add_match_sysname("sdb", false).unwrap();
        enumerator.add_prioritized_subsystem("net").unwrap();
        enumerator.add_match_tag("devmaster").unwrap();

        let dev = Device::from_subsystem_sysname("net", "lo").unwrap();

        enumerator.add_match_parent_incremental(&dev).unwrap();
        enumerator.add_match_parent(&dev).unwrap();
        enumerator.allow_uninitialized().unwrap();
        enumerator
            .add_match_is_initialized(MatchInitializedType::ALL)
            .unwrap();
    }

    #[test]
    fn test_scan_devices() {
        let mut enumerator = DeviceEnumerator::new();
        enumerator.set_enumerator_type(DeviceEnumerationType::Devices);

        for i in enumerator.iter() {
            i.get_devpath().unwrap();
        }
    }

    #[test]
    fn test_scan_subsystems() {
        let mut enumerator = DeviceEnumerator::new();
        enumerator.set_enumerator_type(DeviceEnumerationType::Subsystems);

        for i in enumerator.iter() {
            i.get_devpath().expect("can not get the devpath");
        }
    }

    #[test]
    fn test_scan_devices_and_subsystems() {
        let mut enumerator = DeviceEnumerator::new();
        enumerator.set_enumerator_type(DeviceEnumerationType::All);

        for i in enumerator.iter() {
            i.get_devpath().expect("can not get the devpath");
        }
    }

    trait State {
        fn trans(self: Box<Self>, s: &str) -> Box<dyn State>;
    }

    struct StateMachine {
        state: Option<Box<dyn State>>,
    }

    impl StateMachine {
        fn trans(&mut self, s: &str) {
            if let Some(state) = self.state.take() {
                self.state = Some(state.trans(s));
            }
        }
    }

    struct Init;

    impl State for Init {
        fn trans(self: Box<Self>, s: &str) -> Box<dyn State> {
            match s {
                "net" => Box::new(Front {}),
                _ => panic!(),
            }
        }
    }

    struct Front;

    impl State for Front {
        fn trans(self: Box<Self>, s: &str) -> Box<dyn State> {
            match s {
                "net" => Box::new(Front {}),
                _ => Box::new(Tail {}),
            }
        }
    }

    struct Tail;

    impl State for Tail {
        fn trans(self: Box<Self>, s: &str) -> Box<dyn State> {
            match s {
                "net" => panic!(),
                _ => Box::new(Tail {}),
            }
        }
    }

    #[test]
    fn test_priority_subsystem() {
        /* If the prioritized subsystem is set, e.g., "net",
         * the "net" devices should be ordered in the front of the scanned devices.
         */
        let mut ert = DeviceEnumerator::new();
        ert.set_enumerator_type(DeviceEnumerationType::Devices);
        ert.add_prioritized_subsystem("net").unwrap();
        ert.add_prioritized_subsystem("block").unwrap();

        let mut sm = StateMachine {
            state: Some(Box::new(Init {})),
        };

        for dev in ert.iter() {
            sm.trans(&dev.get_subsystem().unwrap());
        }

        ert.sort_devices().unwrap();
    }

    #[test]
    fn test_match_property() {
        let mut ert = DeviceEnumerator::new();

        let dev = Rc::new(Device::from_subsystem_sysname("net", "lo").unwrap());

        dev.add_property("helloxxx", "worldxxx").unwrap();

        ert.add_match_property("hello*", "world*").unwrap();
        assert!(ert.match_property(dev).unwrap());
    }

    #[test]
    fn test_match_subsystem() {
        let mut ert = DeviceEnumerator::new();

        ert.add_match_subsystem("net", true).unwrap();
        ert.add_match_subsystem("block", false).unwrap();

        assert!(ert.match_subsystem("net"));
        assert!(!ert.match_subsystem("block"));
    }

    #[test]
    fn test_match_sysname() {
        let mut ert = DeviceEnumerator::new();
        ert.add_match_sysname("loop*", true).unwrap();
        ert.add_match_sysname("sd*", false).unwrap();
        assert!(ert.match_sysname("loop1"));
        assert!(!ert.match_sysname("sda"));
    }

    #[test]
    fn test_match_tag() {
        let mut ert = DeviceEnumerator::new();
        let dev = Rc::new(Device::from_subsystem_sysname("net", "lo").unwrap());
        dev.set_base_path("/tmp/devmaster");
        dev.add_tag("devmaster", true);
        dev.update_tag("devmaster", true).unwrap();
        ert.add_match_tag("devmaster").unwrap();
        assert!(ert.match_tag(dev.clone()).unwrap());

        ert.set_enumerator_type(DeviceEnumerationType::Devices);
        ert.scan_devices().unwrap();

        dev.update_tag("devmaster", false).unwrap();
    }

    #[test]
    fn test_match_parent_incremental() {
        let mut ert = DeviceEnumerator::new();
        let dev = Device::from_subsystem_sysname("net", "lo").unwrap();
        assert!(dev.match_parent(&ert.match_parent.borrow(), &HashSet::new()));
        ert.add_match_parent_incremental(&dev).unwrap();
        assert!(dev.match_parent(&ert.match_parent.borrow(), &HashSet::new()));
        let dev_1 = Device::from_subsystem_sysname("drivers", "usb:usb").unwrap();
        assert!(!dev_1.match_parent(&ert.match_parent.borrow(), &HashSet::new()));

        ert.set_enumerator_type(DeviceEnumerationType::Devices);
        ert.scan_devices().unwrap();
    }

    #[test]
    fn test_match_is_initialized() {
        let mut ert = DeviceEnumerator::new();
        let dev = Rc::new(Device::from_subsystem_sysname("net", "lo").unwrap());
        ert.add_match_is_initialized(MatchInitializedType::ALL)
            .unwrap();
        assert!(ert.match_initialized(dev).unwrap());
    }

    #[test]
    fn test_scan_subsystems_all() {
        let mut ert = DeviceEnumerator::new();
        ert.add_match_subsystem("module", true).unwrap();
        ert.add_match_subsystem("subsystem", true).unwrap();
        ert.add_match_subsystem("drivers", true).unwrap();
    }
}
