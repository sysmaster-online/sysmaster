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

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(deprecated)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use device::Device;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::mem;
use std::os::linux::raw::dev_t;
use std::rc::Rc;

use crate::libudev::*;
use libudev_macro::RefUnref;

const ACTION_CNT: usize = 8;

static ACTION_STRING_TABLE: [&str; ACTION_CNT] = [
    "add\0",
    "remove\0",
    "change\0",
    "move\0",
    "online\0",
    "offline\0",
    "bind\0",
    "unbind\0",
];

#[repr(C)]
#[derive(Debug, Clone, RefUnref)]
/// udev_device
pub struct udev_device {
    pub(crate) udev: *mut udev,
    pub(crate) device: Rc<Device>,

    /* Cache CString in udev_device memory. */
    pub(crate) syspath: CString,
    pub(crate) devnode: CString,
    pub(crate) devpath: CString,
    pub(crate) devtype: CString,
    pub(crate) driver: CString,
    pub(crate) sysname: CString,
    pub(crate) subsystem: CString,

    pub(crate) properties: HashMap<CString, CString>,
}

impl Drop for udev_device {
    fn drop(&mut self) {
        if !self.udev.is_null() {
            let _ = unsafe { Rc::from_raw(self.udev) };
        }
    }
}

impl udev_device {
    fn new(udev: *mut udev, device: Device) -> Self {
        Self {
            udev,
            device: Rc::new(device),
            syspath: CString::default(),
            devnode: CString::default(),
            devpath: CString::default(),
            devtype: CString::default(),
            driver: CString::default(),
            sysname: CString::default(),
            subsystem: CString::default(),
            properties: HashMap::default(),
        }
    }
}

#[no_mangle]
/// udev_device_new_from_device_id
pub extern "C" fn udev_device_new_from_device_id(
    udev: *mut udev,
    id: *const ::std::os::raw::c_char,
) -> *mut udev_device {
    let id = unsafe { CStr::from_ptr(id as *const i8) };

    let s = match id.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let device = match Device::from_device_id(s) {
        Ok(d) => d,
        Err(_) => return std::ptr::null_mut(),
    };

    Rc::into_raw(Rc::new(udev_device::new(udev, device))) as *mut udev_device
}

#[no_mangle]
/// udev_device_new_from_devnum
pub extern "C" fn udev_device_new_from_devnum(
    udev: *mut udev,
    type_: ::std::os::raw::c_char,
    devnum: dev_t,
) -> *mut udev_device {
    let device = match Device::from_devnum(type_ as u8 as char, devnum) {
        Ok(d) => d,
        Err(_) => return std::ptr::null_mut(),
    };

    Rc::into_raw(Rc::new(udev_device::new(udev, device))) as *mut udev_device
}

#[no_mangle]
/// udev_device_new_from_subsystem_sysname
pub extern "C" fn udev_device_new_from_subsystem_sysname(
    udev: *mut udev,
    subsystem: *const ::std::os::raw::c_char,
    sysname: *const ::std::os::raw::c_char,
) -> *mut udev_device {
    let subsystem = unsafe { CStr::from_ptr(subsystem as *const i8) }
        .to_str()
        .unwrap();
    let sysname = unsafe { CStr::from_ptr(sysname as *const i8) }
        .to_str()
        .unwrap();
    let device = match Device::from_subsystem_sysname(subsystem, sysname) {
        Ok(d) => d,
        Err(_) => {
            return std::ptr::null_mut();
        }
    };

    Rc::into_raw(Rc::new(udev_device::new(udev, device))) as *mut udev_device
}

#[no_mangle]
/// udev_device_new_from_syspath
pub extern "C" fn udev_device_new_from_syspath(
    udev: *mut udev,
    syspath: *const ::std::os::raw::c_char,
) -> *mut udev_device {
    let syspath = unsafe { CStr::from_ptr(syspath as *const i8) }
        .to_str()
        .unwrap();
    let device = match Device::from_syspath(syspath, true) {
        Ok(d) => d,
        Err(_) => {
            return std::ptr::null_mut();
        }
    };

    Rc::into_raw(Rc::new(udev_device::new(udev, device))) as *mut udev_device
}

#[no_mangle]
/// udev_device_get_syspath
pub extern "C" fn udev_device_get_syspath(
    udev_device: *mut udev_device,
) -> *const ::std::os::raw::c_char {
    let udev_device_mut: &mut udev_device = unsafe { mem::transmute(&mut *udev_device) };

    if !udev_device_mut
        .syspath
        .as_c_str()
        .to_str()
        .unwrap_or_default()
        .is_empty()
    {
        return udev_device_mut.syspath.as_ptr();
    }

    let device_rc = udev_device_mut.device.clone();

    let syspath = match device_rc.get_syspath() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let cstr = match CString::new(syspath) {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    udev_device_mut.syspath = cstr;

    udev_device_mut.syspath.as_ptr()
}

#[no_mangle]
/// udev_device_has_tag
pub extern "C" fn udev_device_has_tag(
    udev_device: *mut udev_device,
    tag: *const ::std::os::raw::c_char,
) -> ::std::os::raw::c_int {
    let udev_device_mut: &mut udev_device = unsafe { mem::transmute(&mut *udev_device) };

    let tag = unsafe { CStr::from_ptr(tag) }.to_str().unwrap();

    match udev_device_mut.device.has_tag(tag) {
        Ok(true) => 1,
        _ => 0,
    }
}

#[no_mangle]
/// udev_device_has_current_tag
pub extern "C" fn udev_device_has_current_tag(
    udev_device: *mut udev_device,
    tag: *const ::std::os::raw::c_char,
) -> ::std::os::raw::c_int {
    let udev_device_mut: &mut udev_device = unsafe { mem::transmute(&mut *udev_device) };

    let tag = unsafe { CStr::from_ptr(tag) }.to_str().unwrap();

    match udev_device_mut.device.has_current_tag(tag) {
        Ok(true) => 1,
        _ => 0,
    }
}

#[no_mangle]
/// udev_device_get_action
pub extern "C" fn udev_device_get_action(
    udev_device: *mut udev_device,
) -> *const ::std::os::raw::c_char {
    let udev_device_mut: &mut udev_device = unsafe { mem::transmute(&mut *udev_device) };

    if let Ok(action) = udev_device_mut.device.get_action() {
        let ret = ACTION_STRING_TABLE[action as usize];
        return ret.as_ptr() as *const ::std::os::raw::c_char;
    }

    std::ptr::null()
}

#[no_mangle]
/// udev_device_get_devnode
pub extern "C" fn udev_device_get_devnode(
    udev_device: *mut udev_device,
) -> *const ::std::os::raw::c_char {
    let udev_device_mut: &mut udev_device = unsafe { mem::transmute(&mut *udev_device) };

    if !udev_device_mut
        .devnode
        .as_c_str()
        .to_str()
        .unwrap_or_default()
        .is_empty()
    {
        return udev_device_mut.devnode.as_ptr();
    }

    if let Ok(devnode) = udev_device_mut.device.get_devname() {
        udev_device_mut.devnode = CString::new(devnode).unwrap();
        return udev_device_mut.devnode.as_ptr();
    }

    std::ptr::null()
}

#[no_mangle]
/// udev_device_get_devnum
pub extern "C" fn udev_device_get_devnum(udev_device: *mut udev_device) -> dev_t {
    let udev_device_mut: &mut udev_device = unsafe { mem::transmute(&mut *udev_device) };

    match udev_device_mut.device.get_devnum() {
        Ok(n) => n,
        Err(e) => {
            if !e.is_errno(nix::Error::ENOENT) {
                errno::set_errno(errno::Errno(e.get_errno() as i32));
            }

            0
        }
    }
}

#[no_mangle]
/// udev_device_get_devpath
pub extern "C" fn udev_device_get_devpath(
    udev_device: *mut udev_device,
) -> *const ::std::os::raw::c_char {
    let udev_device_mut: &mut udev_device = unsafe { mem::transmute(&mut *udev_device) };

    if !udev_device_mut
        .devpath
        .as_c_str()
        .to_str()
        .unwrap_or_default()
        .is_empty()
    {
        return udev_device_mut.devpath.as_ptr();
    }

    match udev_device_mut.device.get_devpath() {
        Ok(devpath) => {
            udev_device_mut.devpath = CString::new(devpath).unwrap();
            udev_device_mut.devpath.as_ptr()
        }
        Err(e) => {
            errno::set_errno(errno::Errno(e.get_errno() as i32));
            std::ptr::null()
        }
    }
}

#[no_mangle]
/// udev_device_get_devtype
pub extern "C" fn udev_device_get_devtype(
    udev_device: *mut udev_device,
) -> *const ::std::os::raw::c_char {
    let udev_device_mut: &mut udev_device = unsafe { mem::transmute(&mut *udev_device) };

    if !udev_device_mut
        .devtype
        .as_c_str()
        .to_str()
        .unwrap_or_default()
        .is_empty()
    {
        return udev_device_mut.devtype.as_ptr();
    }

    match udev_device_mut.device.get_devtype() {
        Ok(devtype) => {
            udev_device_mut.devtype = CString::new(devtype).unwrap();
            udev_device_mut.devtype.as_ptr()
        }
        Err(e) => {
            if !e.is_errno(nix::Error::ENOENT) {
                errno::set_errno(errno::Errno(e.get_errno() as i32));
            }

            std::ptr::null()
        }
    }
}

#[no_mangle]
/// udev_device_get_driver
pub extern "C" fn udev_device_get_driver(
    udev_device: *mut udev_device,
) -> *const ::std::os::raw::c_char {
    let udev_device_mut: &mut udev_device = unsafe { mem::transmute(&mut *udev_device) };

    if !udev_device_mut
        .driver
        .as_c_str()
        .to_str()
        .unwrap_or_default()
        .is_empty()
    {
        return udev_device_mut.driver.as_ptr();
    }

    match udev_device_mut.device.get_driver() {
        Ok(driver) => {
            udev_device_mut.driver = CString::new(driver).unwrap();
            udev_device_mut.driver.as_ptr()
        }
        Err(e) => {
            errno::set_errno(errno::Errno(e.get_errno() as i32));
            std::ptr::null()
        }
    }
}

#[no_mangle]
/// udev_device_get_sysname
pub extern "C" fn udev_device_get_sysname(
    udev_device: *mut udev_device,
) -> *const ::std::os::raw::c_char {
    let udev_device_mut: &mut udev_device = unsafe { mem::transmute(&mut *udev_device) };

    if !udev_device_mut
        .sysname
        .as_c_str()
        .to_str()
        .unwrap_or_default()
        .is_empty()
    {
        return udev_device_mut.sysname.as_ptr();
    }

    match udev_device_mut.device.get_sysname() {
        Ok(sysname) => {
            udev_device_mut.sysname = CString::new(sysname).unwrap();
            udev_device_mut.sysname.as_ptr()
        }
        Err(e) => {
            errno::set_errno(errno::Errno(e.get_errno() as i32));
            std::ptr::null()
        }
    }
}

#[no_mangle]
/// udev_device_get_subsystem
pub extern "C" fn udev_device_get_subsystem(
    udev_device: *mut udev_device,
) -> *const ::std::os::raw::c_char {
    let udev_device_mut: &mut udev_device = unsafe { mem::transmute(&mut *udev_device) };

    if !udev_device_mut
        .subsystem
        .as_c_str()
        .to_str()
        .unwrap_or_default()
        .is_empty()
    {
        return udev_device_mut.subsystem.as_ptr();
    }

    match udev_device_mut.device.get_subsystem() {
        Ok(subsystem) => {
            udev_device_mut.subsystem = CString::new(subsystem).unwrap();
            udev_device_mut.subsystem.as_ptr()
        }
        Err(e) => {
            errno::set_errno(errno::Errno(e.get_errno() as i32));
            std::ptr::null()
        }
    }
}

#[no_mangle]
/// udev_device_get_seqnum
pub extern "C" fn udev_device_get_seqnum(
    udev_device: *mut udev_device,
) -> ::std::os::raw::c_ulonglong {
    let udev_device_mut: &mut udev_device = unsafe { mem::transmute(&mut *udev_device) };

    udev_device_mut.device.get_seqnum().unwrap_or_default() as ::std::os::raw::c_ulonglong
}

#[no_mangle]
/// udev_device_get_property_value
pub extern "C" fn udev_device_get_property_value(
    udev_device: *mut udev_device,
    key: *const ::std::os::raw::c_char,
) -> *const ::std::os::raw::c_char {
    let key = unsafe { CStr::from_ptr(key) }.to_str().unwrap_or_default();
    let udev_device_mut: &mut udev_device = unsafe { mem::transmute(&mut *udev_device) };

    if udev_device_mut
        .properties
        .contains_key(&CString::new(key).unwrap())
    {
        return udev_device_mut
            .properties
            .get(&CString::new(key).unwrap())
            .unwrap()
            .as_ptr();
    }

    match udev_device_mut.device.get_property_value(key) {
        Ok(v) => {
            let key_c = CString::new(key).unwrap();
            let value_c = CString::new(v).unwrap();
            udev_device_mut.properties.insert(key_c.clone(), value_c);
            udev_device_mut.properties.get(&key_c).unwrap().as_ptr()
        }
        Err(e) => {
            errno::set_errno(errno::Errno(e.get_errno() as i32));
            std::ptr::null()
        }
    }
}

#[cfg(test)]
mod test {
    use std::{cell::RefCell, intrinsics::transmute};

    use super::*;
    use device::device_enumerator::*;

    type RD = Rc<RefCell<Device>>;
    pub type Result = std::result::Result<(), device::error::Error>;

    fn test_udev_device_new_from_devnum(device: RD) -> Result {
        let devnum = device.borrow().get_devnum()?;

        let t = if &device.borrow().get_subsystem().unwrap() == "block" {
            'b'
        } else {
            'c'
        };

        let raw_udev_device =
            udev_device_new_from_devnum(std::ptr::null_mut(), t as ::std::os::raw::c_char, devnum);

        let syspath = unsafe { CStr::from_ptr(udev_device_get_syspath(raw_udev_device)) };

        assert_eq!(
            syspath.to_str().unwrap(),
            &device.borrow().get_syspath().unwrap()
        );

        udev_device_unref(raw_udev_device);

        Ok(())
    }

    fn test_udev_device_new_from_subsystem_sysname(device: RD) -> Result {
        let subsystem = device.borrow().get_subsystem().unwrap();
        let sysname = device.borrow().get_sysname().unwrap();

        /* If subsystem is 'drivers', sysname should use '<driver subsystem>:<sysname>'. */
        let name = if subsystem == "drivers" {
            format!("{}:{}", device.borrow().driver_subsystem.borrow(), sysname)
        } else {
            sysname
        };

        let c_subsystem = CString::new(subsystem).unwrap();
        let c_name = CString::new(name).unwrap();
        let raw_udev_device = udev_device_new_from_subsystem_sysname(
            std::ptr::null_mut(),
            c_subsystem.as_ptr(),
            c_name.as_ptr(),
        );

        let syspath = unsafe { CStr::from_ptr(udev_device_get_syspath(raw_udev_device)) };

        assert_eq!(
            syspath.to_str().unwrap(),
            &device.borrow().get_syspath().unwrap()
        );

        udev_device_unref(raw_udev_device);

        Ok(())
    }

    fn test_udev_device_new_from_device_id(device: RD) -> Result {
        let id = device.borrow().get_device_id().unwrap();
        let c_id = CString::new(id).unwrap();
        let udev_device = udev_device_new_from_device_id(std::ptr::null_mut(), c_id.as_ptr());
        let syspath = unsafe { CStr::from_ptr(udev_device_get_syspath(udev_device)) };

        assert_eq!(
            device.borrow().get_syspath().unwrap(),
            syspath.to_str().unwrap().to_string()
        );

        udev_device_unref(udev_device);

        Ok(())
    }

    fn test_udev_device_new_from_syspath(device: RD) -> Result {
        let syspath = device.borrow().get_syspath().unwrap();
        let c_syspath = CString::new(syspath).unwrap();
        let udev_device = udev_device_new_from_syspath(std::ptr::null_mut(), c_syspath.as_ptr());
        let ret_syspath = unsafe { CStr::from_ptr(udev_device_get_syspath(udev_device)) };

        assert_eq!(c_syspath.to_str().unwrap(), ret_syspath.to_str().unwrap());

        udev_device_unref(udev_device);

        Ok(())
    }

    fn from_rd(device: RD) -> *mut udev_device {
        let id = device.borrow().get_device_id().unwrap();
        let c_id = CString::new(id).unwrap();
        udev_device_new_from_device_id(std::ptr::null_mut(), c_id.as_ptr())
    }

    fn test_udev_device_has_tag(device: RD) -> Result {
        let _ = device.borrow_mut().read_db_internal(true);
        device
            .borrow_mut()
            .add_tag("test_udev_device_has_tag", true);
        device.borrow_mut().update_db()?;

        let udev_device = from_rd(device.clone());

        assert!(
            udev_device_has_tag(
                udev_device,
                "test_udev_device_has_tag\0".as_ptr() as *const i8
            ) > 0
        );

        assert!(
            udev_device_has_current_tag(
                udev_device,
                "test_udev_device_has_tag\0".as_ptr() as *const i8
            ) > 0
        );

        udev_device_unref(udev_device);

        device
            .borrow_mut()
            .all_tags
            .borrow_mut()
            .remove("test_udev_device_has_tag");
        device.borrow_mut().remove_tag("test_udev_device_has_tag");
        device.borrow_mut().update_db()?;

        Ok(())
    }

    fn test_udev_device_get_action(dev: RD) -> Result {
        let udev_device = from_rd(dev);

        let udev_device_mut: &mut udev_device = unsafe { transmute(&mut *udev_device) };

        udev_device_mut
            .device
            .set_action_from_string("change")
            .unwrap();

        let ptr = udev_device_get_action(udev_device);

        let action = unsafe { CStr::from_ptr(ptr) };

        assert_eq!(action.to_str().unwrap(), "change");

        udev_device_unref(udev_device);

        Ok(())
    }

    fn test_udev_device_get_devnode(dev: RD) -> Result {
        let udev_device = from_rd(dev.clone());

        let devnode = dev.borrow().get_devname()?;

        let ptr = udev_device_get_devnode(udev_device);

        assert_eq!(unsafe { CStr::from_ptr(ptr) }.to_str().unwrap(), &devnode);

        udev_device_unref(udev_device);

        Ok(())
    }

    fn test_udev_device_get_devnum(dev: RD) -> Result {
        let udev_device = from_rd(dev.clone());

        let devnum = dev.borrow().get_devnum()?;

        assert_eq!(udev_device_get_devnum(udev_device), devnum);

        Ok(())
    }

    fn test_udev_device_get_devpath(dev: RD) -> Result {
        let udev_device = from_rd(dev.clone());

        let devpath = dev.borrow().get_devpath()?;

        assert_eq!(
            unsafe { CStr::from_ptr(udev_device_get_devpath(udev_device)) }
                .to_str()
                .unwrap(),
            &devpath
        );

        Ok(())
    }

    fn test_udev_device_get_devtype(dev: RD) -> Result {
        let ud = from_rd(dev.clone());

        let devtype = dev.borrow().get_devtype()?;

        assert_eq!(
            unsafe { CStr::from_ptr(udev_device_get_devtype(ud)) }
                .to_str()
                .unwrap(),
            &devtype
        );

        Ok(())
    }

    fn test_udev_device_get_driver(dev: RD) -> Result {
        let ud = from_rd(dev.clone());

        let driver = dev.borrow().get_driver()?;

        assert_eq!(
            unsafe { CStr::from_ptr(udev_device_get_driver(ud)) }
                .to_str()
                .unwrap(),
            &driver
        );

        Ok(())
    }

    fn test_udev_device_get_sysname(dev: RD) -> Result {
        let ud = from_rd(dev.clone());

        let sysname = dev.borrow().get_sysname()?;

        assert_eq!(
            unsafe { CStr::from_ptr(udev_device_get_sysname(ud)) }
                .to_str()
                .unwrap(),
            &sysname
        );

        Ok(())
    }

    fn test_udev_device_get_subsystem(dev: RD) -> Result {
        let ud = from_rd(dev.clone());

        let subsystem = dev.borrow().get_subsystem()?;

        assert_eq!(
            unsafe { CStr::from_ptr(udev_device_get_subsystem(ud)) }
                .to_str()
                .unwrap(),
            &subsystem
        );

        Ok(())
    }

    fn test_udev_device_get_seqnum(dev: RD) -> Result {
        let ud = from_rd(dev);

        let ud_mut: &mut udev_device = unsafe { transmute(&mut *ud) };

        ud_mut.device.set_seqnum(10000);

        assert_eq!(udev_device_get_seqnum(ud), 10000);

        Ok(())
    }

    fn test_udev_device_get_property_value(dev: RD) -> Result {
        let ud = from_rd(dev);
        let ud_mut: &mut udev_device = unsafe { transmute(&mut *ud) };
        ud_mut.device.sealed.replace(true);
        ud_mut.device.add_property("hello", "world").unwrap();

        assert_eq!(
            unsafe {
                CStr::from_ptr(udev_device_get_property_value(
                    ud,
                    "hello\0".as_ptr() as *const i8,
                ))
            }
            .to_str()
            .unwrap(),
            "world"
        );

        Ok(())
    }

    #[test]
    fn test_udev_device_ut() {
        let mut e = DeviceEnumerator::new();
        e.set_enumerator_type(DeviceEnumerationType::All);

        for dev in e.iter() {
            let _ = test_udev_device_new_from_device_id(dev.clone());
            let _ = test_udev_device_new_from_devnum(dev.clone());
            let _ = test_udev_device_new_from_subsystem_sysname(dev.clone());
            let _ = test_udev_device_new_from_syspath(dev.clone());
            let _ = test_udev_device_get_devnode(dev.clone());
            let _ = test_udev_device_get_devnum(dev.clone());
            let _ = test_udev_device_get_devpath(dev.clone());
            let _ = test_udev_device_get_devtype(dev.clone());
            let _ = test_udev_device_get_driver(dev.clone());
            let _ = test_udev_device_get_sysname(dev.clone());
            let _ = test_udev_device_get_subsystem(dev.clone());
        }
    }

    #[test]
    fn test_udev_device_has_tag_ut() {
        let dev = Rc::new(RefCell::new(
            Device::from_subsystem_sysname("net", "lo").unwrap(),
        ));
        let _ = test_udev_device_has_tag(dev);
    }

    #[test]
    fn test_udev_device_fake_from_monitor_ut() {
        let dev = Rc::new(RefCell::new(
            Device::from_subsystem_sysname("net", "lo").unwrap(),
        ));
        let _ = test_udev_device_get_action(dev.clone());
        let _ = test_udev_device_get_seqnum(dev.clone());
        let _ = test_udev_device_get_property_value(dev);
    }
}
