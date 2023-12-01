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
use std::ffi::{CStr, CString};
use std::mem;
use std::os::linux::raw::dev_t;
use std::rc::Rc;

use crate::libudev::*;
use libudev_macro::RefUnref;

#[repr(C)]
#[derive(Debug, Clone, RefUnref)]
/// udev_device
pub struct udev_device {
    pub(crate) udev: *mut udev,
    pub(crate) device: Rc<Device>,
    pub(crate) syspath: CString,
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
            syspath: CString::new("").unwrap(),
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

#[cfg(test)]
mod test {
    use std::cell::RefCell;

    use super::*;
    use device::device_enumerator::*;

    type RCDevice = Rc<RefCell<Device>>;
    pub type Result = std::result::Result<(), device::error::Error>;

    fn test_udev_device_new_from_devnum(device: RCDevice) -> Result {
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

    fn test_udev_device_new_from_subsystem_sysname(device: RCDevice) -> Result {
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

    fn test_udev_device_new_from_device_id(device: RCDevice) -> Result {
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

    fn test_udev_device_new_from_syspath(device: RCDevice) -> Result {
        let syspath = device.borrow().get_syspath().unwrap();
        let c_syspath = CString::new(syspath).unwrap();
        let udev_device = udev_device_new_from_syspath(std::ptr::null_mut(), c_syspath.as_ptr());
        let ret_syspath = unsafe { CStr::from_ptr(udev_device_get_syspath(udev_device)) };

        assert_eq!(c_syspath.to_str().unwrap(), ret_syspath.to_str().unwrap());

        udev_device_unref(udev_device);

        Ok(())
    }

    #[test]
    fn test_udev_device_new() {
        let mut e = DeviceEnumerator::new();
        e.set_enumerator_type(DeviceEnumerationType::All);

        for dev in e.iter() {
            let _ = test_udev_device_new_from_device_id(dev.clone());
            let _ = test_udev_device_new_from_devnum(dev.clone());
            let _ = test_udev_device_new_from_subsystem_sysname(dev.clone());
            let _ = test_udev_device_new_from_syspath(dev.clone());
        }
    }
}
