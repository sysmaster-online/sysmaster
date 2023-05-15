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

//! path_id builtin
//!

use crate::builtin::Builtin;
use crate::builtin::Netlink;
use crate::error::{Error, Result};
use device::Device;
use std::cell::RefCell;
use std::ffi::CString;
use std::fs::read_dir;
use std::sync::{Arc, Mutex};

/// path_id builtin command
pub struct PathId;

impl Builtin for PathId {
    /// builtin command
    fn cmd(
        &self,
        device: Arc<Mutex<Device>>,
        _ret_rtnl: &mut RefCell<Option<Netlink>>,
        _argc: i32,
        _argv: Vec<String>,
        test: bool,
    ) -> Result<bool> {
        let mut supported_transport = false;
        let mut supported_parent = false;
        let mut path = String::new();
        let mut compat_path = String::new();

        self.compose_path(
            device.clone(),
            &mut path,
            &mut compat_path,
            &mut supported_transport,
            &mut supported_parent,
        )?;

        if path.is_empty() {
            return Err(Error::BuiltinCommandError {
                msg: "path is empty".to_string(),
            });
        }

        /*
         * Do not return devices with an unknown parent device type. They
         * might produce conflicting IDs if the parent does not provide a
         * unique and predictable name.
         */
        if !supported_parent {
            return Err(Error::BuiltinCommandError {
                msg: "supported_parent is false".to_string(),
            });
        }

        /*
         * Do not return block devices without a well-known transport. Some
         * devices do not expose their buses and do not provide a unique
         * and predictable name that way.
         */
        if let Ok(subsystem) = device.lock().unwrap().get_subsystem() {
            if subsystem == "block" && !supported_transport {
                return Err(Error::BuiltinCommandError {
                    msg: "block error".to_string(),
                });
            }
        }

        let mut tag = String::new();
        /* compose valid udev tag name */
        for ch in path.chars() {
            if ch.is_ascii_alphanumeric() || ch == '-' {
                tag.push(ch);
                continue;
            }

            /* skip all leading '_' */
            if tag.is_empty() {
                continue;
            }

            /* avoid second '_' */
            if tag.ends_with('_') {
                continue;
            }

            tag.push('_');
        }
        /* strip trailing '_' */
        tag = tag.trim_end_matches('_').to_string();

        self.add_property(device.clone(), test, "ID_PATH", &path)
            .unwrap_or(());
        self.add_property(device.clone(), test, "ID_PATH_TAG", &tag)
            .unwrap_or(());

        /*
         * Compatible link generation for ATA devices
         * we assign compat_link to the env variable
         * ID_PATH_ATA_COMPAT
         */
        if !compat_path.is_empty() {
            self.add_property(device, test, "ID_PATH_ATA_COMPAT", &compat_path)
                .unwrap_or(());
        }

        Ok(true)
    }

    /// builtin init function
    fn init(&self) {
        // nothing to do.
    }

    /// builtin exit function
    fn exit(&self) {
        // nothing to do.
    }

    /// check whether builtin command should reload
    fn should_reload(&self) -> bool {
        false
    }

    /// the help of builtin command
    fn help(&self) -> String {
        "Compose persistent device path".to_string()
    }

    /// whether the builtin command can only run once
    fn run_once(&self) -> bool {
        true
    }
}

impl PathId {
    fn compose_path(
        &self,
        dev: Arc<Mutex<Device>>,
        path: &mut String,
        compat_path: &mut String,
        supported_transport: &mut bool,
        supported_parent: &mut bool,
    ) -> Result<bool> {
        let mut parent = Option::Some(dev.clone());
        loop {
            let subsys = parent
                .as_ref()
                .unwrap()
                .lock()
                .unwrap()
                .get_subsystem()
                .unwrap_or_else(|_| "".to_string());
            let mut sysname = String::from(
                parent
                    .as_ref()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .get_sysname()
                    .unwrap_or(""),
            );
            if !subsys.is_empty() && !sysname.is_empty() {
                if subsys == "scsi_tape" {
                    self.hanlde_scsi_tape(parent.as_ref().unwrap().clone(), path);
                } else if subsys == "scsi" {
                    parent = self.hanlde_scsi(
                        parent.as_ref().unwrap().clone(),
                        path,
                        compat_path,
                        supported_parent,
                    );
                    *supported_transport = true;
                } else if subsys == "cciss" {
                    parent = self.hanlde_cciss(parent.as_ref().unwrap().clone(), path);
                    *supported_transport = true;
                } else if subsys == "usb" {
                    parent = self.hanlde_usb(parent.as_ref().unwrap().clone(), path);
                    *supported_transport = true;
                } else if subsys == "bcma" {
                    parent = self.handle_bcma(parent.as_ref().unwrap().clone(), path);
                    *supported_transport = true;
                } else if subsys == "serio" {
                    parent = self.handle_serio(parent.as_ref().unwrap().clone(), path);
                } else if subsys == "pci" || subsys == "acpi" || subsys == "xen" {
                    parent = self.handle_subsys(
                        parent.as_ref().unwrap().clone(),
                        subsys,
                        path,
                        &mut sysname,
                        compat_path,
                    );
                    *supported_parent = true;
                } else if subsys == "platform"
                    || subsys == "amba"
                    || subsys == "scm"
                    || subsys == "ccw"
                    || subsys == "ccwgroup"
                    || subsys == "iucv"
                {
                    parent = self.handle_subsys(
                        parent.as_ref().unwrap().clone(),
                        subsys,
                        path,
                        &mut sysname,
                        compat_path,
                    );
                    *supported_transport = true;
                    *supported_parent = true;
                } else if subsys == "virtio" {
                    parent = self.skip_subsystem(parent.as_ref().unwrap().clone(), &subsys);
                    *supported_transport = true;
                } else if subsys == "ap" {
                    parent = self.handle_ap(parent.as_ref().unwrap().clone(), path);
                    *supported_transport = true;
                } else if subsys == "nvme" {
                    parent = self.handle_nvme(
                        dev.clone(),
                        parent.as_ref().unwrap().clone(),
                        path,
                        compat_path,
                        supported_parent,
                        supported_transport,
                    );
                } else if subsys == "nvme-subsystem" {
                    parent = self.handle_nvme_subsystem(
                        dev.clone(),
                        parent.as_ref().unwrap().clone(),
                        path,
                        compat_path,
                        supported_parent,
                        supported_transport,
                    )?;
                } else if subsys == "spi" {
                    parent = self.handle_nvme_spi(parent.as_ref().unwrap().clone(), path);
                }
            }

            if parent.is_none() {
                break;
            }

            let temp = match parent.as_ref().unwrap().lock().unwrap().get_parent() {
                Ok(res) => Some(res),
                Err(_) => {
                    break;
                }
            };

            parent = temp;
        }

        Ok(true)
    }

    fn hanlde_scsi_tape(&self, dev: Arc<Mutex<Device>>, path: &mut String) {
        let name = match dev.lock().unwrap().get_sysname() {
            Ok(name) => String::from(name),
            Err(_) => return,
        };

        if "nst" == name || "st" == name {
            self.path_prepend(path, name);
        } else if name.starts_with("nst") && "lma".contains(&name[3..=3]) {
            self.path_prepend(path, format!("nst{}", &name[3..=3]));
        } else if name.starts_with("st") && "lma".contains(&name[2..=2]) {
            self.path_prepend(path, format!("st{}", &name[2..=2]));
        }
    }

    fn hanlde_scsi(
        &self,
        parent: Arc<Mutex<Device>>,
        path: &mut String,
        compat_path: &mut String,
        supported_parent: &mut bool,
    ) -> Option<Arc<Mutex<Device>>> {
        let devtype = match parent.lock().unwrap().get_devtype() {
            Ok(devtype) => devtype,
            Err(_) => return Some(parent.clone()),
        };

        if devtype != "scsi_device" {
            return Some(parent);
        }

        let id = parent.lock().unwrap().get_sysattr_value("ieee1394_id");
        if id.is_ok() {
            self.path_prepend(path, format!("ieee1394-0x{}", id.unwrap()));
            *supported_parent = true;
            return self.skip_subsystem(parent, "scsi");
        }

        let name = match parent.lock().unwrap().get_syspath() {
            Ok(name) => String::from(name),
            Err(_) => return None,
        };

        if name == "/rport-" {
            *supported_parent = true;
            return self.hanlde_scsi_fibre_channel(parent, path);
        }

        if name == "/end_device-" {
            *supported_parent = true;
            return self.hanlde_scsi_sas(parent, path);
        }

        if name == "/session" {
            *supported_parent = true;
            return self.hanlde_scsi_iscsi(parent, path);
        }

        if name == "/ata" {
            return self.hanlde_scsi_ata(parent, path, compat_path);
        }

        if name == "/vmbus_" {
            return self.hanlde_scsi_hyperv(parent, path, 37);
        } else if name == "VMBUS" {
            return self.hanlde_scsi_hyperv(parent, path, 38);
        }
        self.handle_scsi_default(parent, path)
    }

    fn hanlde_scsi_fibre_channel(
        &self,
        parent: Arc<Mutex<Device>>,
        path: &mut String,
    ) -> Option<Arc<Mutex<Device>>> {
        let targetdev = match parent
            .lock()
            .unwrap()
            .get_parent_with_subsystem_devtype("scsi", Some("scsi_target"))
        {
            Ok(dev) => dev,
            Err(_) => return None,
        };
        let sysname = match targetdev.lock().unwrap().get_sysname() {
            Ok(sysname) => String::from(sysname),
            Err(_) => return None,
        };
        let mut fcdev = match Device::from_subsystem_sysname("fc_transport".to_string(), sysname) {
            Ok(dev) => dev,
            Err(_) => return None,
        };

        let port = match fcdev.get_sysattr_value("port_name") {
            Ok(port) => port,
            Err(_) => return None,
        };

        let lun = self.format_lun_number(parent.clone());
        let s = format!("fc-{}-{}", port, lun);
        self.path_prepend(path, s);
        Some(parent)
    }

    fn hanlde_scsi_sas(
        &self,
        parent: Arc<Mutex<Device>>,
        path: &mut String,
    ) -> Option<Arc<Mutex<Device>>> {
        let targetdev = match parent
            .lock()
            .unwrap()
            .get_parent_with_subsystem_devtype("scsi", Some("scsi_target"))
        {
            Ok(dev) => dev,
            Err(_) => return None,
        };
        let target_parent = match targetdev.lock().unwrap().get_parent() {
            Ok(dev) => dev,
            Err(_) => return None,
        };
        let sysname = match target_parent.lock().unwrap().get_sysname() {
            Ok(sysname) => String::from(sysname),
            Err(_) => return None,
        };
        let mut asadev = match Device::from_subsystem_sysname("sas_device".to_string(), sysname) {
            Ok(dev) => dev,
            Err(_) => return None,
        };

        let sas_address = match asadev.get_sysattr_value("asa_address") {
            Ok(addr) => addr,
            Err(_) => return None,
        };

        let lun = self.format_lun_number(parent.clone());
        let s = format!("sas-{}-{}", sas_address, lun);
        self.path_prepend(path, s);

        Some(parent)
    }

    fn hanlde_scsi_iscsi(
        &self,
        parent: Arc<Mutex<Device>>,
        path: &mut String,
    ) -> Option<Arc<Mutex<Device>>> {
        let transportdev = parent.clone();
        let mut sysname;
        /* find iscsi session */
        loop {
            let transportdev = match transportdev.lock().unwrap().get_parent() {
                Ok(dev) => dev,
                Err(_) => return None,
            };
            sysname = match transportdev.lock().unwrap().get_sysname() {
                Ok(name) => String::from(name),
                Err(_) => return None,
            };
            if sysname.starts_with("session") {
                break;
            }
        }

        /* find iscsi session device */
        let mut sessiondev =
            match Device::from_subsystem_sysname("iscsi_session".to_string(), sysname) {
                Ok(dev) => dev,
                Err(_) => return None,
            };

        let target = match sessiondev.get_sysattr_value("asa_address") {
            Ok(port) => port,
            Err(_) => return None,
        };

        let sysnum = match transportdev.lock().unwrap().get_sysnum() {
            Ok(num) => num,
            Err(_) => return None,
        };

        let connname = format!("connection{}:0", sysnum);
        let mut conndev =
            match Device::from_subsystem_sysname("iscsi_connection".to_string(), connname) {
                Ok(dev) => dev,
                Err(_) => return None,
            };
        let addr = match conndev.get_sysattr_value("persistent_address") {
            Ok(addr) => addr,
            Err(_) => return None,
        };
        let port = match conndev.get_sysattr_value("persistent_port") {
            Ok(port) => port,
            Err(_) => return None,
        };

        let lun = self.format_lun_number(parent.clone());
        self.path_prepend(
            path,
            format!("ip-{}:{}-iscsi-{}-{}", addr, port, target, lun),
        );

        Some(parent)
    }

    fn hanlde_scsi_ata(
        &self,
        parent: Arc<Mutex<Device>>,
        path: &mut String,
        compat_path: &mut String,
    ) -> Option<Arc<Mutex<Device>>> {
        let sysname = match parent.lock().unwrap().get_sysname() {
            Ok(name) => String::from(name),
            Err(_) => return None,
        };
        let mut host: u32 = 0;
        let mut bus: u32 = 0;
        let mut target: u32 = 0;
        let mut lun: u32 = 0;
        let cstr = CString::new(sysname).unwrap();
        let fmt = CString::new("%u:%u:%u:%u").unwrap();
        let ret = unsafe {
            libc::sscanf(
                cstr.as_ptr(),
                fmt.as_ptr(),
                &mut host as &mut libc::c_uint,
                &mut bus as &mut libc::c_uint,
                &mut target as &mut libc::c_uint,
                &mut lun as &mut libc::c_uint,
            )
        };
        if ret != 4 {
            return None;
        }

        let targetdev = match parent
            .lock()
            .unwrap()
            .get_parent_with_subsystem_devtype("scsi", Some("scsi_host"))
        {
            Ok(dev) => dev,
            Err(_) => return None,
        };

        let target_parent = match targetdev.lock().unwrap().get_parent() {
            Ok(dev) => dev,
            Err(_e) => return None,
        };

        let sysname = match target_parent.lock().unwrap().get_sysname() {
            Ok(name) => String::from(name),
            Err(_) => return None,
        };

        let mut atadev = match Device::from_subsystem_sysname("ata_port".to_string(), sysname) {
            Ok(dev) => dev,
            Err(_) => return None,
        };

        let port_no = match atadev.get_sysattr_value("port_no") {
            Ok(port) => port,
            Err(_) => return None,
        };

        if bus != 0 {
            /* Devices behind port multiplier have a bus != 0 */
            self.path_prepend(path, format!("ata-{}.{}.0", port_no, bus))
        } else {
            /* Master/Slave are distinguished by target id */
            self.path_prepend(path, format!("ata-{}.{}", port_no, bus))
        }

        /* old compatible persistent link for ATA devices */
        if !compat_path.is_empty() {
            self.path_prepend(path, format!("ata-{}", port_no))
        }

        Some(parent)
    }

    fn hanlde_scsi_hyperv(
        &self,
        parent: Arc<Mutex<Device>>,
        path: &mut String,
        giud_str_len: usize,
    ) -> Option<Arc<Mutex<Device>>> {
        let hostdev = match parent
            .lock()
            .unwrap()
            .get_parent_with_subsystem_devtype("scsi", Some("scsi_host"))
        {
            Ok(dev) => dev,
            Err(_) => return None,
        };

        let vmbusdev = match hostdev.lock().unwrap().get_parent() {
            Ok(dev) => dev,
            Err(_e) => return None,
        };

        let guid_str = match vmbusdev.lock().unwrap().get_sysattr_value("device_id") {
            Ok(str) => str,
            Err(_e) => return None,
        };

        if guid_str.len() < giud_str_len || !guid_str.starts_with('{') || !guid_str.ends_with('}') {
            return None;
        }

        let mut guid = String::new();
        for ch in guid_str.chars() {
            if ch == '-' {
                continue;
            }
            guid.push(ch);
        }

        let lun = self.format_lun_number(parent.clone());
        self.path_prepend(path, format!("vmbus-{}-{}", guid, lun));
        Some(parent)
    }

    fn handle_scsi_default(
        &self,
        parent: Arc<Mutex<Device>>,
        path: &mut String,
    ) -> Option<Arc<Mutex<Device>>> {
        let mut basenum = -1;
        let hostdev = match parent
            .lock()
            .unwrap()
            .get_parent_with_subsystem_devtype("scsi", Some("scsi_host"))
        {
            Ok(dev) => dev,
            Err(_) => return None,
        };
        let name = match parent.lock().unwrap().get_sysname() {
            Ok(name) => String::from(name),
            Err(_) => return None,
        };

        let mut host: i32 = 0;
        let mut bus: i32 = 0;
        let mut target: i32 = 0;
        let mut lun: i32 = 0;
        let cstr = CString::new(name).unwrap();
        let fmt = CString::new("%d:%d:%d:%d").unwrap();
        let ret = unsafe {
            libc::sscanf(
                cstr.as_ptr(),
                fmt.as_ptr(),
                &mut host as &mut libc::c_int,
                &mut bus as &mut libc::c_int,
                &mut target as &mut libc::c_int,
                &mut lun as &mut libc::c_int,
            )
        };
        if ret != 4 {
            return None;
        }

        /*
         * Rebase host offset to get the local relative number
         *
         * Note: This is by definition racy, unreliable and too simple.
         * Please do not copy this model anywhere. It's just a left-over
         * from the time we had no idea how things should look like in
         * the end.
         *
         * Making assumptions about a global in-kernel counter and use
         * that to calculate a local offset is a very broken concept. It
         * can only work as long as things are in strict order.
         *
         * The kernel needs to export the instance/port number of a
         * controller directly, without the need for rebase magic like
         * this. Manual driver unbind/bind, parallel hotplug/unplug will
         * get into the way of this "I hope it works" logic.
         */
        let base = match hostdev.lock().unwrap().get_syspath() {
            Ok(base) => String::from(base),
            Err(_) => return None,
        };
        let pos = match base.rfind('/') {
            Some(n) => n,
            None => return None,
        };

        let base = &base[..pos];
        let dir = match read_dir(base) {
            Ok(dir) => dir,
            Err(_) => return None,
        };

        for entry in dir {
            let de = match entry {
                Ok(de) => de,
                Err(_) => return None,
            };
            let d_name = match de.file_name().to_str() {
                Some(name) => String::from(name),
                None => return None,
            };

            if d_name.starts_with('.') {
                continue;
            }
            let d_type = match de.file_type() {
                Ok(t) => t,
                Err(_) => return None,
            };
            if !d_type.is_dir() || !d_type.is_symlink() {
                continue;
            }
            if d_name.starts_with("host") {
                continue;
            }
            let d_name = &d_name[4..];
            let i = match d_name.parse::<i32>() {
                Ok(i) => i,
                Err(_) => return None,
            };
            /*
             * find the smallest number; the host really needs to export its
             * own instance number per parent device; relying on the global host
             * enumeration and plainly rebasing the numbers sounds unreliable
             */
            if basenum == -1 || i < basenum {
                basenum = i;
            }
        }
        if basenum == -1 {
            return Some(hostdev);
        }
        host -= basenum;
        self.path_prepend(
            path,
            format!(
                "scsi-{}:{}:{}:{}",
                host as u32, bus as u32, target as u32, lun as u32
            ),
        );
        Some(hostdev)
    }

    fn format_lun_number(&self, dev: Arc<Mutex<Device>>) -> String {
        let sysnum = match dev.lock().unwrap().get_sysnum() {
            Ok(sysnum) => sysnum,
            Err(_) => return String::new(),
        };

        let lun = match sysnum.parse::<u64>() {
            Ok(lun) => lun,
            Err(_) => return String::new(),
        };
        let mut path = String::new();
        if lun < 256 {
            self.path_prepend(&mut path, format!("lun-{}", lun));
        } else {
            self.path_prepend(
                &mut path,
                format!(
                    "lun-0x{:04x}{:04x}00000000",
                    lun & 0xffff,
                    (lun >> 16) & 0xffff
                ),
            );
        }
        path
    }

    fn hanlde_cciss(
        &self,
        parent: Arc<Mutex<Device>>,
        path: &mut String,
    ) -> Option<Arc<Mutex<Device>>> {
        let name = match parent.lock().unwrap().get_sysname() {
            Ok(s) => String::from(s),
            Err(_) => return None,
        };
        let mut controller: u32 = 0;
        let mut disk: u32 = 0;
        let cstr = CString::new(name.clone()).unwrap();
        let fmt = CString::new("c%ud%u%*s").unwrap();
        let ret = unsafe {
            libc::sscanf(
                cstr.as_ptr(),
                fmt.as_ptr(),
                &mut controller as &mut libc::c_uint,
                &mut disk as &mut libc::c_uint,
            )
        };
        if ret != 2 {
            return None;
        }

        self.path_prepend(path, format!("cciss-disk{}", disk));
        self.skip_subsystem(parent, &name)
    }

    fn hanlde_usb(
        &self,
        parent: Arc<Mutex<Device>>,
        path: &mut String,
    ) -> Option<Arc<Mutex<Device>>> {
        let devtype = match parent.lock().unwrap().get_devtype() {
            Ok(devtype) => devtype,
            Err(_) => return Some(parent.clone()),
        };

        if devtype != "usb_interface" && devtype != "usb_device" {
            return Some(parent);
        }

        let sysname = match parent.lock().unwrap().get_sysname() {
            Ok(sysname) => String::from(sysname),
            Err(_) => return Some(parent.clone()),
        };

        let pos = match sysname.find('-') {
            Some(pos) => pos,
            None => return Some(parent),
        };
        let port = &sysname[pos + 1..];

        self.path_prepend(path, format!("usb-0:{}", port));
        self.skip_subsystem(parent, "usb")
    }

    fn handle_bcma(
        &self,
        parent: Arc<Mutex<Device>>,
        path: &mut String,
    ) -> Option<Arc<Mutex<Device>>> {
        let sysname = match parent.lock().unwrap().get_sysname() {
            Ok(sysname) => String::from(sysname),
            Err(_) => return None,
        };

        let mut core: u32 = 0;
        let cstr = CString::new(sysname).unwrap();
        let fmt = CString::new("bcma%*u:%u").unwrap();
        let ret =
            unsafe { libc::sscanf(cstr.as_ptr(), fmt.as_ptr(), &mut core as &mut libc::c_uint) };
        if ret != 1 {
            return None;
        }

        self.path_prepend(path, format!("bcma-{}", core));
        Some(parent)
    }

    fn handle_serio(
        &self,
        parent: Arc<Mutex<Device>>,
        path: &mut String,
    ) -> Option<Arc<Mutex<Device>>> {
        let sysnum = match parent.lock().unwrap().get_sysnum() {
            Ok(sysnum) => sysnum,
            Err(_) => return Some(parent.clone()),
        };

        if !sysnum.is_empty() {
            self.path_prepend(path, format!("serio-{}", sysnum));
            return self.skip_subsystem(parent, "serio");
        }

        Some(parent)
    }

    fn handle_subsys(
        &self,
        parent: Arc<Mutex<Device>>,
        subsys: String,
        path: &mut String,
        sysname: &mut String,
        compat_path: &mut String,
    ) -> Option<Arc<Mutex<Device>>> {
        self.path_prepend(path, format!("{}-{}", subsys, sysname));
        if !compat_path.is_empty() {
            self.path_prepend(compat_path, format!("{}-{}", subsys, sysname));
        }
        self.skip_subsystem(parent, &subsys)
    }

    fn handle_ap(
        &self,
        parent: Arc<Mutex<Device>>,
        path: &mut String,
    ) -> Option<Arc<Mutex<Device>>> {
        let systype = parent.lock().unwrap().get_sysattr_value("type");
        let func = parent.lock().unwrap().get_sysattr_value("ap_functions");
        if systype.is_ok() && func.is_ok() {
            self.path_prepend(path, format!("ap-{}-{}", systype.unwrap(), func.unwrap()));
        } else if let Ok(sysname) = parent.lock().unwrap().get_sysname() {
            self.path_prepend(path, format!("ap-{}", sysname));
        }

        self.skip_subsystem(parent, "ap")
    }

    fn handle_nvme(
        &self,
        dev: Arc<Mutex<Device>>,
        parent: Arc<Mutex<Device>>,
        path: &mut String,
        compat_path: &mut String,
        supported_parent: &mut bool,
        supported_transport: &mut bool,
    ) -> Option<Arc<Mutex<Device>>> {
        if let Ok(nsid) = dev.lock().unwrap().get_sysattr_value("nsid") {
            self.path_prepend(path, format!("nvme-{}", nsid));
            if !compat_path.is_empty() {
                self.path_prepend(compat_path, format!("nvme-{}", nsid));
            }

            *supported_parent = true;
            *supported_transport = true;
            return self.skip_subsystem(parent, "nvme");
        }
        Some(parent)
    }

    fn handle_nvme_subsystem(
        &self,
        dev: Arc<Mutex<Device>>,
        parent: Arc<Mutex<Device>>,
        path: &mut String,
        compat_path: &mut String,
        supported_parent: &mut bool,
        supported_transport: &mut bool,
    ) -> Result<Option<Arc<Mutex<Device>>>> {
        if let Ok(nsid) = dev.lock().unwrap().get_sysattr_value("nsid") {
            self.path_prepend(path, format!("nvme-{}", nsid));
            if !compat_path.is_empty() {
                self.path_prepend(compat_path, format!("nvme-{}", nsid));
            }

            let dev_other_branch = self.find_real_nvme_parent(dev.clone())?;

            *supported_parent = true;
            *supported_transport = true;
            return Ok(self.skip_subsystem(dev_other_branch, "nvme"));
        }

        Ok(Some(parent))
    }

    fn handle_nvme_spi(
        &self,
        parent: Arc<Mutex<Device>>,
        path: &mut String,
    ) -> Option<Arc<Mutex<Device>>> {
        let sysnum = match parent.lock().unwrap().get_sysnum() {
            Ok(sysnum) => sysnum,
            Err(_) => return Some(parent.clone()),
        };
        if !sysnum.is_empty() {
            self.path_prepend(path, format!("cs-{}", sysnum));
            return self.skip_subsystem(parent, "spi");
        }
        Some(parent)
    }

    fn find_real_nvme_parent(&self, dev: Arc<Mutex<Device>>) -> Result<Arc<Mutex<Device>>> {
        /* If the device belongs to "nvme-subsystem" (not to be confused with "nvme"), which happens when
         * NVMe multipathing is enabled in the kernel (/sys/module/nvme_core/parameters/multipath is Y),
         * then the syspath is something like the following:
         *   /sys/devices/virtual/nvme-subsystem/nvme-subsys0/nvme0n1
         * Hence, we need to find the 'real parent' in "nvme" subsystem, e.g,
         *   /sys/devices/pci0000:00/0000:00:1c.4/0000:3c:00.0/nvme/nvme0 */

        let sysname = match dev.lock().unwrap().get_sysname() {
            Ok(name) => String::from(name),
            Err(_) => {
                return Err(Error::BuiltinCommandError {
                    msg: "Failed to get_sysname".to_string(),
                })
            }
        };

        /* The sysname format of nvme block device is nvme%d[c%d]n%d[p%d], e.g. nvme0n1p2 or nvme0c1n2.
         * (Note, nvme device with 'c' can be ignored, as they are hidden. )
         * The sysname format of nvme subsystem device is nvme%d.
         * See nvme_alloc_ns() and nvme_init_ctrl() in drivers/nvme/host/core.c for more details. */

        if !sysname.starts_with("nvme") {
            return Err(Error::BuiltinCommandError {
                msg: "Failed to get nvme".to_string(),
            });
        }
        let end = sysname
            .trim_start_matches("nvme")
            .trim_start_matches(char::is_numeric);
        let sysname = &sysname[..sysname.len() - end.len()];

        match Device::from_subsystem_sysname("nvme".to_string(), sysname.to_string()) {
            Ok(dev) => Ok(Arc::new(Mutex::new(dev))),
            Err(e) => Err(Error::BuiltinCommandError {
                msg: format!("Failed to get_sysname :{:?}", e),
            }),
        }
    }

    fn path_prepend(&self, path: &mut String, fmt: String) {
        if path.is_empty() {
            path.push_str(&fmt);
        } else {
            path.insert_str(0, &format!("{}-", fmt));
        }
    }

    fn skip_subsystem(
        &self,
        device: Arc<Mutex<Device>>,
        subsys: &str,
    ) -> Option<Arc<Mutex<Device>>> {
        let mut dev = device.clone();
        let mut parent = device;
        loop {
            let subsystem = match parent.lock().unwrap().get_subsystem() {
                Ok(str) => str,
                Err(_e) => break,
            };

            if subsys != subsystem {
                break;
            }

            dev = parent.clone();

            let temp = match parent.lock().unwrap().get_parent() {
                Ok(res) => res,
                Err(_e) => break,
            };

            parent = temp;
        }

        Some(dev)
    }
}

#[cfg(test)]
mod tests {
    use super::PathId;
    use crate::builtin::{Builtin, Netlink};
    use device::device_enumerator::DeviceEnumerator;
    use std::cell::RefCell;

    #[test]
    fn test_builtin_example() {
        let mut enumerator = DeviceEnumerator::new();

        for device in enumerator.iter_mut() {
            let mut rtnl = RefCell::<Option<Netlink>>::from(None);

            let builtin = PathId {};
            println!("devpath:{:?}", device.lock().unwrap().get_devpath());
            if let Err(e) = builtin.cmd(device.clone(), &mut rtnl, 0, vec![], true) {
                println!("Builtin command path_id: fails:{:?}", e);
            }
        }
    }
}
