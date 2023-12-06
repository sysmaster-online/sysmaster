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

//! subcommand for devctl trigger
//!

use crate::subcmds::utils::find_device;
use crate::Result;

use device::device_monitor::{DeviceMonitor, MonitorNetlinkGroup};
use device::{
    device_enumerator::{DeviceEnumerationType, DeviceEnumerator},
    DeviceAction,
};
use event::{EventState, EventType, Events, Source};
use std::os::unix::io::RawFd;
use std::path::Path;
use std::{cell::RefCell, collections::HashSet, rc::Rc};

#[derive(Debug)]
pub struct TriggerArgs {
    action: Option<String>,
    r#type: Option<String>,
    verbose: bool,
    dry_run: bool,
    subsystem_match: Option<Vec<String>>,
    subsystem_nomatch: Option<Vec<String>>,
    attr_match: Option<Vec<String>>,
    attr_nomatch: Option<Vec<String>>,
    property_match: Option<Vec<String>>,
    tag_match: Option<Vec<String>>,
    sysname_match: Option<Vec<String>>,
    name_match: Option<Vec<String>>,
    parent_match: Option<Vec<String>>,
    settle: bool,
    uuid: bool,
    devices: Vec<String>,
}

impl TriggerArgs {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        action: Option<String>,
        r#type: Option<String>,
        verbose: bool,
        dry_run: bool,
        subsystem_match: Option<Vec<String>>,
        subsystem_nomatch: Option<Vec<String>>,
        attr_match: Option<Vec<String>>,
        attr_nomatch: Option<Vec<String>>,
        property_match: Option<Vec<String>>,
        tag_match: Option<Vec<String>>,
        sysname_match: Option<Vec<String>>,
        name_match: Option<Vec<String>>,
        parent_match: Option<Vec<String>>,
        settle: bool,
        uuid: bool,
        devices: Vec<String>,
    ) -> Self {
        TriggerArgs {
            action,
            r#type,
            verbose,
            dry_run,
            subsystem_match,
            subsystem_nomatch,
            attr_match,
            attr_nomatch,
            property_match,
            tag_match,
            sysname_match,
            name_match,
            parent_match,
            settle,
            uuid,
            devices,
        }
    }

    /// subcommand for trigger a fake device action, then the kernel will report an uevent
    pub fn subcommand(&self) -> Result<()> {
        // if no device is declared, enumerate all devices or subsystems and drivers under /sys/
        let mut enumerator = DeviceEnumerator::new();
        if let Err(err) = enumerator.allow_uninitialized() {
            return Err(err.get_errno());
        }

        let action = match &self.action {
            Some(a) => a.parse::<DeviceAction>().unwrap(),
            None => DeviceAction::Change,
        };

        if let Some(subsystems) = &self.subsystem_match {
            for subsystem in subsystems {
                if let Err(e) = enumerator.add_match_subsystem(subsystem, true) {
                    log::error!("Failed to add subsystem match {:?}", subsystem);
                    return Err(e.get_errno());
                }
            }
        }

        if let Some(subsystems) = &self.subsystem_nomatch {
            for subsystem in subsystems {
                if let Err(e) = enumerator.add_match_subsystem(subsystem, false) {
                    log::error!("Failed to add negative subsystem match {:?}", subsystem);
                    return Err(e.get_errno());
                }
            }
        }

        if let Some(attrs) = &self.attr_match {
            for attr in attrs {
                let (key, val) = keyval(attr);
                if let Err(e) = enumerator.add_match_sysattr(&key, &val, true) {
                    log::error!("Failed to add sysattr match {:?}={:?}", key, val);
                    return Err(e.get_errno());
                }
            }
        }

        if let Some(attrs) = &self.attr_nomatch {
            for attr in attrs {
                let (key, val) = keyval(attr);
                if let Err(e) = enumerator.add_match_sysattr(&key, &val, false) {
                    log::error!("Failed to add negative sysattr match {:?}={:?}", key, val);
                    return Err(e.get_errno());
                }
            }
        }

        if let Some(properties) = &self.property_match {
            for property in properties {
                let (key, val) = keyval(property);
                if let Err(e) = enumerator.add_match_property(&key, &val) {
                    log::error!("Failed to add property match {:?}={:?}", key, val);
                    return Err(e.get_errno());
                }
            }
        }

        if let Some(tags) = &self.tag_match {
            for tag in tags {
                if let Err(e) = enumerator.add_match_tag(tag) {
                    log::error!("Failed to add tag match {:?}", tag);
                    return Err(e.get_errno());
                }
            }
        }

        if let Some(sysnames) = &self.sysname_match {
            for sysname in sysnames {
                if let Err(e) = enumerator.add_match_sysname(sysname, true) {
                    log::error!("Failed to add sysname match {:?}", sysname);
                    return Err(e.get_errno());
                }
            }
        }

        if let Some(names) = &self.name_match {
            for name in names {
                let dev = match find_device(name, "/dev") {
                    Ok(dev) => dev,
                    Err(e) => {
                        log::error!("Failed to open the device {:?}", name);
                        return Err(e);
                    }
                };

                if let Err(e) = enumerator.add_match_parent_incremental(&dev) {
                    log::error!("Failed to add parent match {:?} err:{:?}", name, e);
                    return Err(e.get_errno());
                }
            }
        }

        if let Some(parents) = &self.parent_match {
            for parent in parents {
                let dev = match find_device(parent, "/sys") {
                    Ok(dev) => dev,
                    Err(e) => {
                        log::error!("Failed to open the device {:?}", parent);
                        return Err(e);
                    }
                };

                if let Err(e) = enumerator.add_match_parent_incremental(&dev) {
                    log::error!("Failed to add parent match {:?} err:{:?}", parent, e);
                    return Err(e.get_errno());
                }
            }
        }

        for dev in &self.devices {
            let device = match find_device(dev, "") {
                Ok(d) => d,
                Err(e) => {
                    log::error!("Failed to open the device: {:?} err:{:?}", dev, e);
                    return Err(e);
                }
            };
            if let Err(err) = enumerator.add_match_parent_incremental(&device) {
                return Err(err.get_errno());
            }
        }

        let etype = match &self.r#type {
            Some(t) => {
                if t == "devices" {
                    DeviceEnumerationType::Devices
                } else if t == "subsystems" {
                    DeviceEnumerationType::Subsystems
                } else if t == "all" {
                    DeviceEnumerationType::All
                } else {
                    log::error!("invalid events type{}", t);
                    return Err(nix::Error::EINVAL);
                }
            }
            None => DeviceEnumerationType::Devices,
        };

        enumerator.set_enumerator_type(etype);

        let settle_path_or_ids = self.exec_list(&mut enumerator, action)?;

        let events = Events::new().unwrap();
        if self.settle {
            let monitor = Rc::new(TriggerMonitor::new(
                DeviceMonitor::new(MonitorNetlinkGroup::Userspace, None),
                settle_path_or_ids.clone(),
                self.verbose,
                self.uuid,
            ));
            events.add_source(monitor.clone()).unwrap();
            events.set_enabled(monitor, EventState::On).unwrap();
        }

        if !settle_path_or_ids.is_empty() {
            if let Err(e) = events.rloop() {
                log::error!("Event loop failed err:{:?}", e);
                return Err(nix::Error::EINVAL);
            }
        }

        Ok(())
    }

    fn exec_list(
        &self,
        enumerator: &mut DeviceEnumerator,
        action: DeviceAction,
    ) -> Result<HashSet<String>> {
        let mut uuid_supported = -1;
        let mut uuids = HashSet::new();
        let mut ret = Ok(HashSet::<String>::new());
        for device in enumerator.iter() {
            let syspath = match device.get_syspath() {
                Ok(syspath) => syspath,
                Err(_) => continue,
            };
            if self.verbose {
                println!("{}", syspath);
            }
            if self.dry_run {
                continue;
            }

            let id = match device
                .trigger_with_uuid(action, (self.uuid || self.settle) && uuid_supported != 0)
            {
                Ok(id) => id,
                Err(e) => {
                    if e.get_errno() == nix::errno::Errno::EINVAL
                        && !self.uuid
                        && self.settle
                        && uuid_supported < 0
                    {
                        /* If we specified a UUID because of the settling logic, and we got EINVAL this might
                         * be caused by an old kernel which doesn't know the UUID logic (pre-4.13). Let's try
                         * if it works without the UUID logic then. */
                        if let Err(e) = device.trigger(action) {
                            if e.get_errno() != nix::Error::EINVAL {
                                /* dropping the uuid stuff changed the return code,
                                 * hence don't bother next time */
                                uuid_supported = 0;
                            }
                        }
                        None
                    } else {
                        if ![nix::Error::ENOENT, nix::Error::ENODEV].contains(&e.get_errno()) {
                            eprintln!("Failed to trigger {:?}: {:?}", syspath, e);
                            if ret.is_ok() {
                                ret = Err(e.get_errno());
                            }
                        } else {
                            println!("Ignore to trigger {:?}: {:?}", syspath, e);
                        }

                        if [nix::Error::EACCES, nix::Error::EROFS].contains(&e.get_errno()) {
                            /* Inovoked by unprivileged user, or read only filesystem. Return earlier. */
                            return Err(e.get_errno());
                        }
                        continue;
                    }
                }
            };

            if uuid_supported < 0 {
                uuid_supported = 1;
            }

            /* If the user asked for it, write event UUID to stdout */
            if self.uuid {
                if let Some(uuid) = &id {
                    println!("{}", uuid.to_string());
                }
            }

            if self.settle {
                if uuid_supported != 0 {
                    if let Some(uuid) = id {
                        uuids.insert(uuid.to_string());
                    }
                } else {
                    uuids.insert(syspath);
                }
            }
        }

        if let Err(err) = ret {
            return Err(err);
        }

        Ok(uuids)
    }
}

fn keyval(buf: &str) -> (String, String) {
    let mut key = buf.to_string();
    let mut val = String::new();

    if let Some(pos) = buf.rfind('=') {
        let (left, right) = buf.split_at(pos);
        let right = &right[1..];
        key = left.to_string();
        val = right.to_string();
    }

    (key, val)
}

/// trigger monitor
#[derive(Debug)]
struct TriggerMonitor {
    device_monitor: DeviceMonitor,
    settle_path_or_ids: RefCell<HashSet<String>>,
    verbose: bool,
    uuid: bool,
}

/// public methods
impl TriggerMonitor {
    /// create a monitor instance for monitoring trigger
    pub fn new(
        device_monitor: DeviceMonitor,
        settle_path_or_ids: HashSet<String>,
        verbose: bool,
        uuid: bool,
    ) -> TriggerMonitor {
        TriggerMonitor {
            device_monitor,
            settle_path_or_ids: RefCell::new(settle_path_or_ids),
            verbose,
            uuid,
        }
    }
}

impl Source for TriggerMonitor {
    /// socket fd
    fn fd(&self) -> RawFd {
        self.device_monitor.fd()
    }

    /// event type
    fn event_type(&self) -> EventType {
        EventType::Io
    }

    /// epoll type
    fn epoll_event(&self) -> u32 {
        (libc::EPOLLIN) as u32
    }

    /// priority of event source
    fn priority(&self) -> i8 {
        0i8
    }

    /// receive device from socket and remove path or uuid from settle_path_or_ids
    fn dispatch(&self, event: &Events) -> i32 {
        let device = match self.device_monitor.receive_device() {
            Ok(ret) => match ret {
                Some(device) => device,
                None => return 0,
            },
            Err(_) => {
                return 0;
            }
        };

        let syspath = match device.get_syspath() {
            Ok(syspath) => syspath,
            Err(e) => {
                log::error!("Failed to get syspath of device event, ignoring:{:?}", e);
                return 0;
            }
        };

        let id = device.get_trigger_uuid();
        match &id {
            Ok(Some(id)) => {
                if !self.settle_path_or_ids.borrow_mut().remove(&id.to_string()) {
                    log::debug!(
                        "Got uevent not matching expected UUID, ignoring. {:?}",
                        device.get_syspath()
                    );
                    return 0;
                }
            }
            _ => {
                let mut saved = self.settle_path_or_ids.borrow_mut().remove(&syspath);
                if !saved {
                    /* When the device is renamed, the new name is broadcast, and the old name is saved
                     * in INTERFACE_OLD.
                     *
                     * TODO: remove support for INTERFACE_OLD when kernel baseline is bumped to 4.13 or
                     * higher.
                     */
                    if let Ok(old_sysname) = device.get_property_value("INTERFACE_OLD") {
                        let dir = match Path::new(&syspath).parent() {
                            Some(dir) => dir.to_str().unwrap(),
                            None => {
                                log::error!(
                                    "Failed to extract directory from {:?}, ignoring",
                                    syspath
                                );
                                return 0;
                            }
                        };
                        let old_syspath = dir.to_string() + "/" + &old_sysname;
                        saved = self.settle_path_or_ids.borrow_mut().remove(&old_syspath);
                    }
                }

                if !saved {
                    log::debug!(
                        "Got uevent for unexpected device, ignoring. {:?}",
                        device.get_syspath()
                    );
                    return 0;
                }
            }
        }

        if self.verbose {
            println!("settle {}", syspath);
        }

        if self.uuid {
            println!("settle {}", id.unwrap().unwrap().to_string());
        }

        if self.settle_path_or_ids.borrow().is_empty() {
            event.set_exit();
        }

        0
    }

    /// token of event source
    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }
}
