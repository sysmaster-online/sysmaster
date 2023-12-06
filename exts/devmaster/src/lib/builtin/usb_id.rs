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

//! usb_id builtin
//!

use crate::builtin::Builtin;
use crate::rules::exec_unit::ExecuteUnit;
use crate::utils::commons::*;
use crate::{error::*, log_dev};
use device::Device;
use snafu::ResultExt;
use std::ffi::CString;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::rc::Rc;

#[repr(C, packed)]
#[allow(non_snake_case)]
struct UsbInterfaceDescriptor {
    bLength: u8,
    bDescriptorType: u8,
    bInterfaceNumber: u8,
    bAlternateSetting: u8,
    bNumEndpoints: u8,
    bInterfaceClass: u8,
    bInterfaceSubClass: u8,
    bInterfaceProtocol: u8,
    iInterface: u8,
}

#[derive(Default, Clone)]
struct UsbInfo {
    vendor: String,
    vendor_enc: String,
    vendor_id: String,
    model: String,
    model_enc: String,
    product_id: String,
    revision: String,
    serial: String,
    serial_short: String,
    type_str: String,
    instance: String,
    packed_if: String,
    ifnum: String,
    driver: String,
    protocol: i32,
    if_class: String,
}

/// usb_id builtin command
pub struct UsbId;

impl UsbId {
    const USB_IFTYPE_TABLE: [(i32, &'static str); 9] = [
        (1, "audio"),
        (3, "hid"),
        (5, "Physical"),
        (6, "media"),
        (7, "printer"),
        (8, "storage"),
        (9, "hub"),
        (0x0e, "video"),
        (0xff, "generic"), // fallback for undefined values
    ];

    fn usb_iftype(if_class_num: i32) -> Option<&'static str> {
        let result = Self::USB_IFTYPE_TABLE
            .iter()
            .find(|&&(class_num, _)| class_num == if_class_num);
        result.map(|&(_, name)| name)
    }

    const SUBTYPE_MAP: [(i32, &'static str); 6] = [
        (1, "rbc"),
        (2, "atapi"),
        (3, "tape"),
        (4, "floppy"),
        (6, "scsi"),
        (0, "generic"),
    ];

    fn usb_mass_storage_ifsubtype(from: &str, protocol: &mut i32) -> Option<&'static str> {
        *protocol = 0;
        if let Ok(num) = from.parse::<i32>() {
            for (n, s) in Self::SUBTYPE_MAP {
                if n == num {
                    *protocol = n;
                    return Some(s);
                }
            }
        }
        Some("generic")
    }

    fn scsi_type(from: &str) -> Option<&'static str> {
        let num = from.parse::<i32>().ok()?;
        Some(match num {
            0 | 0xE => "disk",
            1 => "tape",
            4 | 7 | 0xF => "optical",
            5 => "cd",
            _ => "generic",
        })
    }

    const USB_DT_INTERFACE: u8 = 0x04;
    fn dev_if_packed_info(dev: &Device, info: &mut UsbInfo) -> Result<()> {
        let syspath = dev.get_syspath().unwrap();
        let filename = PathBuf::from(&syspath).join("descriptors");
        let mut file = File::open(filename).context(IoSnafu {
            filename: syspath.clone(),
        })?;
        let mut buf = [0u8; 18 + 65535];
        let mut pos = 0;

        let size: usize = file.read(&mut buf).context(IoSnafu {
            filename: syspath.clone(),
        })?;
        if size < 18 {
            return Err(Error::ReadTooShort { filename: syspath });
        }

        while pos + std::mem::size_of::<UsbInterfaceDescriptor>() < size {
            let desc: UsbInterfaceDescriptor =
                unsafe { std::ptr::read_unaligned(buf.as_ptr().add(pos) as *const _) };
            if desc.bLength < 3 {
                break;
            }
            if desc.bLength > (size - std::mem::size_of::<UsbInterfaceDescriptor>()) as u8 {
                return Err(Error::Other {
                    msg: syspath,
                    errno: nix::errno::Errno::EINVAL,
                });
            }
            pos += desc.bLength as usize;

            if desc.bDescriptorType != Self::USB_DT_INTERFACE {
                continue;
            }

            let if_str = format!(
                ":{:02x}{:02x}{:02x}",
                desc.bInterfaceClass, desc.bInterfaceSubClass, desc.bInterfaceProtocol
            );

            if if_str.len() != 7 {
                continue;
            }

            if info.packed_if.contains(&if_str) {
                continue;
            }

            info.packed_if.push_str(&if_str);
        }

        if !info.packed_if.is_empty() {
            info.packed_if.push(':');
        }

        Ok(())
    }

    fn interface_directory(&self, device: &Device, info: &mut UsbInfo) -> Result<bool> {
        let dev_interface = device
            .get_parent_with_subsystem_devtype("usb", Some("usb_interface"))
            .context(DeviceSnafu)?;

        let _interface_syspath = dev_interface.get_syspath().context(DeviceSnafu)?;

        info.ifnum = dev_interface
            .get_sysattr_value("bInterfacceNumber")
            .unwrap_or_default();

        info.driver = dev_interface
            .get_sysattr_value("driver")
            .unwrap_or_default();

        info.if_class = dev_interface
            .get_sysattr_value("bInterfaceClass")
            .context(DeviceSnafu)?;

        info.type_str = match info.if_class.parse::<i32>().context(ParseIntSnafu)? {
            8 => {
                let mut type_str = String::new();
                if let Ok(if_subclass) = dev_interface.get_sysattr_value("bInterfaceSubClass") {
                    type_str = UsbId::usb_mass_storage_ifsubtype(&if_subclass, &mut info.protocol)
                        .unwrap()
                        .to_string();
                }
                type_str
            }
            i => UsbId::usb_iftype(i).unwrap().to_string(),
        };
        Ok(true)
    }

    fn mass_storage(&self, device: &Device, info: &mut UsbInfo) -> Result<bool> {
        if [2, 6].contains(&info.protocol) {
            let dev_scsi = device
                .get_parent_with_subsystem_devtype("scsi", Some("scsi_device"))
                .context(DeviceSnafu)?;

            let scsi_sysname = dev_scsi.get_sysname().context(DeviceSnafu)?;

            let mut _host: u32 = 0;
            let mut _bus: u32 = 0;
            let mut target: u32 = 0;
            let mut lun: u32 = 0;
            let cstr = CString::new(scsi_sysname.clone()).unwrap();
            let fmt = CString::new("%u:%u:%u:%u").unwrap();
            let ret = unsafe {
                libc::sscanf(
                    cstr.as_ptr(),
                    fmt.as_ptr(),
                    &mut _host as &mut libc::c_uint,
                    &mut _bus as &mut libc::c_uint,
                    &mut target as &mut libc::c_uint,
                    &mut lun as &mut libc::c_uint,
                )
            };
            if ret != 4 {
                log_dev!(
                    debug,
                    &dev_scsi,
                    format!("Failed to parse target number '{}'", scsi_sysname)
                );
                return Err(Error::Nix {
                    source: nix::Error::EINVAL,
                });
            }

            let scsi_vendor: String = dev_scsi.get_sysattr_value("vendor").context(DeviceSnafu)?;
            // scsi_vendor to vendor
            encode_devnode_name(&scsi_vendor, &mut info.vendor_enc);
            info.vendor = replace_whitespace(&scsi_vendor);
            info.vendor = replace_chars(&info.vendor, "");

            let scsi_model = dev_scsi.get_sysattr_value("model").context(DeviceSnafu)?;
            // scsi_model to model
            encode_devnode_name(&scsi_model, &mut info.model_enc);
            info.model = replace_whitespace(&scsi_model);
            info.model = replace_chars(&info.model, "");

            let scsi_type_str = dev_scsi.get_sysattr_value("type").context(DeviceSnafu)?;

            // scsi_type_str to type_str
            if let Some(s) = UsbId::scsi_type(&scsi_type_str) {
                info.type_str = s.to_string();
            };

            let scsi_revision = dev_scsi.get_sysattr_value("rev").context(DeviceSnafu)?;

            // scsi_revision to revision, unimplemented!()
            info.revision = replace_whitespace(&scsi_revision);
            info.revision = replace_chars(&info.revision, "");

            info.instance = format!("{}:{}", target, lun);
        }
        Ok(true)
    }

    fn set_sysattr(&self, device: &Device, info: &mut UsbInfo) -> Result<bool> {
        info.vendor_id = device.get_sysattr_value("idVendor").context(DeviceSnafu)?;

        info.product_id = device.get_sysattr_value("idProduct").context(DeviceSnafu)?;

        if info.vendor.is_empty() {
            let usb_vendor = match device.get_sysattr_value("manufacturer") {
                Ok(s) => s,
                Err(_) => info.vendor_id.clone(),
            };
            encode_devnode_name(&usb_vendor, &mut info.vendor_enc);
            info.vendor = replace_whitespace(&usb_vendor);
            info.vendor = replace_chars(&info.vendor, "");
        }

        if info.model.is_empty() {
            let usb_model = match device.get_sysattr_value("product") {
                Ok(s) => s,
                Err(_) => info.product_id.clone(),
            };
            encode_devnode_name(&usb_model, &mut info.model_enc);
            info.model = replace_whitespace(&usb_model);
            info.model = replace_chars(&info.model, "");
        }

        if info.revision.is_empty() {
            if let Ok(usb_revision) = device.get_sysattr_value("bcdDevice") {
                info.revision = replace_whitespace(&usb_revision);
                info.revision = replace_chars(&info.revision, "");
            }
        }

        if info.serial_short.is_empty() {
            if let Ok(mut usb_serial) = device.get_sysattr_value("serial") {
                // usb_serial to serial
                for (_idx, byte) in usb_serial.bytes().enumerate() {
                    if !(0x20..=0x7f).contains(&byte) || byte == b',' {
                        usb_serial.clear();
                        break;
                    }
                }

                if !usb_serial.is_empty() {
                    info.serial_short = replace_whitespace(&usb_serial);
                    info.serial_short = replace_chars(&info.serial_short, "");
                }
            }

            info.serial = format!("{0}_{1}", info.vendor, info.model);
            if !info.serial_short.is_empty() {
                info.serial = format!("{0}_{1}", info.serial, info.serial_short);
            }

            if !info.instance.is_empty() {
                info.serial = format!("{0}-{1}", info.serial, info.instance);
            }
        }

        Ok(true)
    }
}

impl Builtin for UsbId {
    /// builtin command
    fn cmd(
        &self,
        exec_unit: &ExecuteUnit,
        _argc: i32,
        _argv: Vec<String>,
        test: bool,
    ) -> Result<bool> {
        let device = exec_unit.get_device();

        let mut info = UsbInfo::default();
        let mut usb_device = Rc::new(Device::default());

        let _syspath = device.get_syspath().context(DeviceSnafu)?;
        let _sysname = device.get_sysname().context(DeviceSnafu)?;
        let devtype = device.get_devtype().context(DeviceSnafu)?;

        #[allow(clippy::never_loop)]
        loop {
            if devtype == "usb_device" {
                let _ = Self::dev_if_packed_info(&device, &mut info);
                usb_device = device.clone();
                break;
            }

            match self.interface_directory(&device, &mut info) {
                Ok(true) => (),
                Ok(false) => break,
                Err(e) => return Err(e),
            };

            log::debug!("if_class:{} protocol:{}", info.if_class, info.protocol);

            let dev_usb = device
                .get_parent_with_subsystem_devtype("usb", Some("usb_interface"))
                .context(DeviceSnafu)?;

            let _ = Self::dev_if_packed_info(&dev_usb, &mut info);

            match self.mass_storage(&device, &mut info) {
                Ok(_) => (),
                Err(e) => {
                    log::error!("{:?}", e);
                }
            }

            usb_device = dev_usb.clone();
            break;
        }

        self.set_sysattr(&usb_device, &mut info)?;

        // Set up a temporary variable id_bus here to prevent deadlock.
        let id_bus = device.get_property_value("ID_BUS");
        match id_bus {
            Ok(_) => log::debug!("ID_BUS property is already set, setting only properties prefixed with \"ID_USB_\"."),
            Err(_) => {
                self.add_property(device.clone(), test, "ID_BUS", "usb")?;
                self.add_property(device.clone(), test, "ID_MODEL", &info.model)?;
                self.add_property(device.clone(), test, "ID_MODEL_ENC", &info.model_enc)?;
                self.add_property(device.clone(), test, "ID_MODEL_ID", &info.product_id)?;

                self.add_property(device.clone(), test, "ID_SERIAL", &info.serial)?;
                if !info.serial_short.is_empty() {
                        self.add_property(device.clone(), test, "ID_SERIAL_SHORT", &info.serial_short)?;
                }
                self.add_property(device.clone(), test, "ID_VENDOR", &info.vendor)?;
                self.add_property(device.clone(), test, "ID_VENDOR_ENC", &info.vendor_enc)?;
                self.add_property(device.clone(), test, "ID_VENDOR_ID", &info.vendor_id)?;

                self.add_property(device.clone(), test, "ID_REVISION", &info.revision)?;

                if !info.type_str.is_empty() {
                    self.add_property(device.clone(), test, "ID_TYPE", &info.type_str)?;
                }
                if !info.instance.is_empty() {
                    self.add_property(device.clone(), test, "ID_INSTANCE", &info.instance)?;
                }
            },
        }

        self.add_property(device.clone(), test, "ID_USB_MODEL", &info.model)?;
        self.add_property(device.clone(), test, "ID_USB_MODEL_ENC", &info.model_enc)?;
        self.add_property(device.clone(), test, "ID_USB_MODEL_ID", &info.product_id)?;
        self.add_property(device.clone(), test, "ID_USB_SERIAL", &info.serial)?;
        if !info.serial_short.is_empty() {
            self.add_property(
                device.clone(),
                test,
                "ID_USB_SERIAL_SHORT",
                &info.serial_short,
            )?;
        }

        self.add_property(device.clone(), test, "ID_USB_VENDOR", &info.vendor)?;
        self.add_property(device.clone(), test, "ID_USB_VENDOR_ENC", &info.vendor_enc)?;
        self.add_property(device.clone(), test, "ID_USB_VENDOR_ID", &info.vendor_id)?;
        self.add_property(device.clone(), test, "ID_USB_REVISION", &info.revision)?;

        if !info.type_str.is_empty() {
            self.add_property(device.clone(), test, "ID_USB_TYPE", &info.type_str)?;
        }

        if !info.instance.is_empty() {
            self.add_property(device.clone(), test, "ID_USB_INSTANCE", &info.instance)?;
        }
        if !info.packed_if.is_empty() {
            self.add_property(device.clone(), test, "ID_USB_INTERFACES", &info.packed_if)?;
        }
        if !info.ifnum.is_empty() {
            self.add_property(device.clone(), test, "ID_USB_INTERFACE_NUM", &info.ifnum)?;
        }
        if !info.driver.is_empty() {
            self.add_property(device, test, "ID_USB_DRIVER", &info.driver)?;
        }

        Ok(true)
    }

    /// builtin init function
    fn init(&self) {}

    /// builtin exit function
    fn exit(&self) {}

    /// check whether builtin command should reload
    fn should_reload(&self) -> bool {
        false
    }

    /// the help of builtin command
    fn help(&self) -> String {
        "USB device properties".to_string()
    }

    /// whether the builtin command can only run once
    fn run_once(&self) -> bool {
        true
    }
}
#[cfg(test)]
mod test {
    use crate::{
        builtin::{usb_id::UsbId, Builtin},
        rules::exec_unit::ExecuteUnit,
    };
    use device::device_enumerator::DeviceEnumerator;

    #[test]
    fn test_usb_mass_storage_ifsubtype() {
        let mut protocol = 0;
        assert_eq!(
            UsbId::usb_mass_storage_ifsubtype("1", &mut protocol),
            Some("rbc")
        );
        assert_eq!(
            UsbId::usb_mass_storage_ifsubtype("2", &mut protocol),
            Some("atapi")
        );
        assert_eq!(
            UsbId::usb_mass_storage_ifsubtype("3", &mut protocol),
            Some("tape")
        );
        assert_eq!(
            UsbId::usb_mass_storage_ifsubtype("4", &mut protocol),
            Some("floppy")
        );

        assert_eq!(
            UsbId::usb_mass_storage_ifsubtype("6", &mut protocol),
            Some("scsi")
        );
        assert_eq!(
            UsbId::usb_mass_storage_ifsubtype("0", &mut protocol),
            Some("generic")
        );
        assert_eq!(
            UsbId::usb_mass_storage_ifsubtype("7", &mut protocol),
            Some("generic")
        );
    }

    #[test]
    fn test_scsi_type() {
        assert_eq!(UsbId::scsi_type("0"), Some("disk"));
        assert_eq!(UsbId::scsi_type("14"), Some("disk"));
        assert_eq!(UsbId::scsi_type("1"), Some("tape"));
        assert_eq!(UsbId::scsi_type("4"), Some("optical"));
        assert_eq!(UsbId::scsi_type("7"), Some("optical"));
        assert_eq!(UsbId::scsi_type("15"), Some("optical"));
        assert_eq!(UsbId::scsi_type("5"), Some("cd"));
        assert_eq!(UsbId::scsi_type("10"), Some("generic"));
    }

    #[test]
    fn test_usb_id() {
        let mut enumerator = DeviceEnumerator::new();

        for device in enumerator.iter() {
            let builtin = UsbId {};
            if let Ok(str) = device.get_devpath() {
                if !str.contains("usb") {
                    continue;
                }
            }
            let exec_unit = ExecuteUnit::new(device);
            let _ = builtin.cmd(&exec_unit, 0, vec![], true);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;

    #[test]
    fn test_sscanf() {
        let mut host: u32 = 0;
        let mut bus: u32 = 0;
        let mut target: u32 = 0;
        let mut lun: u32 = 0;
        let cstr = CString::new("100:200:300:400").unwrap();
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

        assert_eq!(ret, 4);
        assert_eq!(host, 100);
        assert_eq!(bus, 200);
        assert_eq!(target, 300);
        assert_eq!(lun, 400);
    }
}
