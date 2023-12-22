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

use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::intrinsics::transmute;
use std::rc::Rc;

use crate::libudev_device::udev_device;
use crate::libudev_list::{udev_list, udev_list_entry};
use crate::{assert_return, libudev::*};
use device::device_enumerator::*;
use libudev_macro::append_impl;
use libudev_macro::RefUnref;

#[repr(C)]
#[derive(Clone, RefUnref)]
/// udev_enumerate
pub struct udev_enumerate {
    pub(crate) udev: *mut udev,
    pub(crate) devices_list: Rc<udev_list>,
    pub(crate) up_to_date: bool,

    pub(crate) enumerator: Rc<RefCell<DeviceEnumerator>>,
}

#[no_mangle]
#[append_impl]
/// udev_enumerate_new
pub extern "C" fn udev_enumerate_new(udev: *mut udev) -> *mut udev_enumerate {
    let mut enumerator = DeviceEnumerator::new();
    if let Err(e) = enumerator.allow_uninitialized() {
        errno::set_errno(errno::Errno(e.get_errno() as i32));
        return std::ptr::null_mut();
    }

    Rc::into_raw(Rc::new(udev_enumerate {
        udev,
        devices_list: Rc::new(udev_list::new(true)),
        up_to_date: false,
        enumerator: Rc::new(RefCell::new(enumerator)),
    })) as *mut udev_enumerate
}

#[no_mangle]
#[append_impl]
/// udev_enumerate_scan_devices
pub extern "C" fn udev_enumerate_scan_devices(
    udev_enumerate: *mut udev_enumerate,
) -> ::std::os::raw::c_int {
    assert_return!(!udev_enumerate.is_null(), -libc::EINVAL);

    let udev_enumerate: &mut udev_enumerate = unsafe { transmute(&mut *udev_enumerate) };

    match udev_enumerate.enumerator.borrow_mut().scan_devices() {
        Ok(_) => 0,
        Err(e) => e.get_errno() as i32,
    }
}

#[no_mangle]
#[append_impl]
/// udev_enumerate_get_list_entry
pub extern "C" fn udev_enumerate_get_list_entry(
    udev_enumerate: *mut udev_enumerate,
) -> *mut udev_list_entry {
    assert_return!(!udev_enumerate.is_null(), {
        errno::set_errno(errno::Errno(libc::EINVAL));
        std::ptr::null_mut()
    });

    let udev_enumerate: &mut udev_enumerate = unsafe { transmute(&mut *udev_enumerate) };

    if !udev_enumerate.up_to_date {
        udev_enumerate.devices_list.cleanup();

        for i in udev_enumerate.enumerator.borrow_mut().iter() {
            let syspath = match i.get_syspath() {
                Ok(s) => s,
                Err(e) => {
                    errno::set_errno(errno::Errno(e.get_errno() as i32));
                    return std::ptr::null_mut();
                }
            };

            udev_enumerate
                .devices_list
                .add_entry(CString::new(syspath).unwrap(), CString::default());
        }
    }

    let ret = udev_enumerate.devices_list.get_entry();

    if ret.is_null() {
        errno::set_errno(errno::Errno(libc::ENODATA));
    }

    ret
}

#[no_mangle]
#[append_impl]
/// udev_enumerate_add_match_subsystem
pub extern "C" fn udev_enumerate_add_match_subsystem(
    udev_enumerate: *mut udev_enumerate,
    subsystem: *const ::std::os::raw::c_char,
) -> ::std::os::raw::c_int {
    assert_return!(
        !udev_enumerate.is_null() && !subsystem.is_null(),
        -libc::EINVAL
    );

    if subsystem.is_null() {
        return 0;
    }

    let udev_enumerate: &mut udev_enumerate = unsafe { transmute(&mut *udev_enumerate) };

    let subsystem = unsafe { CStr::from_ptr(subsystem) }.to_str().unwrap();

    if let Err(e) = udev_enumerate
        .enumerator
        .borrow_mut()
        .add_match_subsystem(subsystem, true)
    {
        return e.get_errno() as i32;
    }

    udev_enumerate.up_to_date = false;

    0
}

#[no_mangle]
#[append_impl]
/// udev_enumerate_add_match_property
pub extern "C" fn udev_enumerate_add_match_property(
    udev_enumerate: *mut udev_enumerate,
    property: *const ::std::os::raw::c_char,
    value: *const ::std::os::raw::c_char,
) -> ::std::os::raw::c_int {
    assert_return!(
        !udev_enumerate.is_null() && !property.is_null(),
        -libc::EINVAL
    );

    if property.is_null() {
        return 0;
    }

    let udev_enumerate: &mut udev_enumerate = unsafe { transmute(&mut *udev_enumerate) };

    let property = unsafe { CStr::from_ptr(property) }.to_str().unwrap();
    let value = if value.is_null() {
        ""
    } else {
        unsafe { CStr::from_ptr(value) }.to_str().unwrap()
    };

    if let Err(e) = udev_enumerate
        .enumerator
        .borrow_mut()
        .add_match_property(property, value)
    {
        return e.get_errno() as i32;
    }

    udev_enumerate.up_to_date = false;

    0
}

#[no_mangle]
#[append_impl]
/// udev_enumerate_add_match_is_initialized
pub fn udev_enumerate_add_match_is_initialized(
    udev_enumerate: *mut udev_enumerate,
) -> ::std::os::raw::c_int {
    assert_return!(!udev_enumerate.is_null(), -libc::EINVAL);

    let udev_enumerate: &mut udev_enumerate = unsafe { transmute(&mut *udev_enumerate) };

    if let Err(e) = udev_enumerate
        .enumerator
        .borrow_mut()
        .add_match_is_initialized(MatchInitializedType::Compat)
    {
        return e.get_errno() as i32;
    }

    udev_enumerate.up_to_date = false;

    0
}

#[no_mangle]
#[append_impl]
/// udev_enumerate_get_udev
pub extern "C" fn udev_enumerate_get_udev(udev_enumerate: *mut udev_enumerate) -> *mut udev {
    assert_return!(!udev_enumerate.is_null(), {
        errno::set_errno(errno::Errno(libc::EINVAL));
        std::ptr::null_mut()
    });

    let e: &mut udev_enumerate = unsafe { transmute(&mut *udev_enumerate) };

    e.udev
}

#[no_mangle]
#[append_impl]
/// udev_enumerate_add_match_tag
pub extern "C" fn udev_enumerate_add_match_tag(
    udev_enumerate: *mut udev_enumerate,
    tag: *const ::std::os::raw::c_char,
) -> ::std::os::raw::c_int {
    assert_return!(!udev_enumerate.is_null(), -libc::EINVAL);

    let e: &mut udev_enumerate = unsafe { transmute(&mut *udev_enumerate) };
    let tag = unsafe { CStr::from_ptr(tag) }.to_str().unwrap();

    if let Err(e) = e.enumerator.borrow_mut().add_match_tag(tag) {
        return e.get_errno() as i32;
    }

    e.up_to_date = false;
    0
}

#[no_mangle]
#[append_impl]
/// udev_enumerate_add_match_parent
pub extern "C" fn udev_enumerate_add_match_parent(
    udev_enumerate: *mut udev_enumerate,
    parent: *mut udev_device,
) -> ::std::os::raw::c_int {
    assert_return!(!udev_enumerate.is_null(), -libc::EINVAL);

    if parent.is_null() {
        return 0;
    }

    let e: &mut udev_enumerate = unsafe { transmute(&mut *udev_enumerate) };
    let p: Rc<udev_device> = unsafe { Rc::from_raw(parent) };

    if let Err(e) = e.enumerator.borrow_mut().add_match_parent(&p.device) {
        return e.get_errno() as i32;
    }

    let _ = Rc::into_raw(p);

    e.up_to_date = false;
    0
}

#[no_mangle]
#[append_impl]
/// udev_enumerate_add_match_sysattr
pub extern "C" fn udev_enumerate_add_match_sysattr(
    udev_enumerate: *mut udev_enumerate,
    sysattr: *const ::std::os::raw::c_char,
    value: *const ::std::os::raw::c_char,
) -> ::std::os::raw::c_int {
    assert_return!(!udev_enumerate.is_null(), -libc::EINVAL);

    if sysattr.is_null() {
        return 0;
    }

    let e: &mut udev_enumerate = unsafe { transmute(&mut *udev_enumerate) };

    let sysattr = unsafe { CStr::from_ptr(sysattr) }.to_str().unwrap();
    let value = if value.is_null() {
        ""
    } else {
        unsafe { CStr::from_ptr(value) }.to_str().unwrap()
    };

    if let Err(e) = e
        .enumerator
        .borrow_mut()
        .add_match_sysattr(sysattr, value, true)
    {
        return e.get_errno() as i32;
    }

    e.up_to_date = false;
    0
}

#[no_mangle]
#[append_impl]
/// udev_enumerate_add_nomatch_sysattr
pub extern "C" fn udev_enumerate_add_nomatch_sysattr(
    udev_enumerate: *mut udev_enumerate,
    sysattr: *const ::std::os::raw::c_char,
    value: *const ::std::os::raw::c_char,
) -> ::std::os::raw::c_int {
    assert_return!(!udev_enumerate.is_null(), -libc::EINVAL);

    if sysattr.is_null() {
        return 0;
    }

    let e: &mut udev_enumerate = unsafe { transmute(&mut *udev_enumerate) };

    let sysattr = unsafe { CStr::from_ptr(sysattr) }.to_str().unwrap();
    let value = if value.is_null() {
        ""
    } else {
        unsafe { CStr::from_ptr(value) }.to_str().unwrap()
    };

    if let Err(e) = e
        .enumerator
        .borrow_mut()
        .add_match_sysattr(sysattr, value, false)
    {
        return e.get_errno() as i32;
    }

    e.up_to_date = false;
    0
}

#[no_mangle]
#[append_impl]
/// udev_enumerate_add_nomatch_subsystem
pub extern "C" fn udev_enumerate_add_nomatch_subsystem(
    udev_enumerate: *mut udev_enumerate,
    subsystem: *const ::std::os::raw::c_char,
) -> ::std::os::raw::c_int {
    assert_return!(!udev_enumerate.is_null(), -libc::EINVAL);

    if subsystem.is_null() {
        return 0;
    }

    let udev_enumerate: &mut udev_enumerate = unsafe { transmute(&mut *udev_enumerate) };

    let subsystem = unsafe { CStr::from_ptr(subsystem) }.to_str().unwrap();

    if let Err(e) = udev_enumerate
        .enumerator
        .borrow_mut()
        .add_match_subsystem(subsystem, false)
    {
        return e.get_errno() as i32;
    }

    udev_enumerate.up_to_date = false;

    0
}

#[cfg(test)]
mod tests {
    use device::Device;

    use crate::libudev_list::{udev_list_entry_get_name_impl, udev_list_entry_get_next_impl};

    use super::*;

    #[test]
    fn test_enumerator() {
        let e = udev_enumerate_new_impl(std::ptr::null_mut());

        assert_eq!(
            udev_enumerate_add_match_subsystem_impl(e, "block\0".as_ptr() as *const i8),
            0
        );
        assert_eq!(
            udev_enumerate_add_match_property_impl(
                e,
                "ID_TYPE\0".as_ptr() as *const i8,
                "disk\0".as_ptr() as *const i8,
            ),
            0
        );
        assert_eq!(udev_enumerate_add_match_is_initialized_impl(e), 0);
        assert_eq!(udev_enumerate_scan_devices_impl(e), 0);

        let mut entry = udev_enumerate_get_list_entry_impl(e);

        loop {
            if entry.is_null() {
                break;
            }

            let syspath = unsafe { CStr::from_ptr(udev_list_entry_get_name_impl(entry)) };

            let syspath = syspath.to_str().unwrap();

            let device = Device::from_syspath(syspath, true).unwrap();
            assert_eq!(&device.get_subsystem().unwrap(), "block");
            assert_eq!(&device.get_property_value("ID_TYPE").unwrap(), "disk");

            entry = udev_list_entry_get_next_impl(entry);
        }
    }
}
