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
use std::os::{linux::raw::dev_t, raw::c_char};
use std::rc::Rc;

use crate::libudev_list::{udev_list, udev_list_entry};
use crate::{assert_return, libudev::*};
use libudev_macro::append_impl;
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

    pub(crate) properties: Rc<udev_list>,
    pub(crate) properties_read: bool,

    pub(crate) devlinks: Rc<udev_list>,
    pub(crate) devlinks_read: bool,

    pub(crate) sysattrs: Rc<udev_list>,
    pub(crate) sysattrs_read: bool,

    pub(crate) parent: *mut udev_device,
}

impl Drop for udev_device {
    fn drop(&mut self) {
        if !self.parent.is_null() {
            let _ = unsafe { Rc::from_raw(self.parent) };
        }
    }
}

impl udev_device {
    pub(crate) fn new(udev: *mut udev, device: Rc<Device>) -> Self {
        Self {
            udev,
            device,
            syspath: CString::default(),
            devnode: CString::default(),
            devpath: CString::default(),
            devtype: CString::default(),
            driver: CString::default(),
            sysname: CString::default(),
            subsystem: CString::default(),
            properties: Rc::new(udev_list::new(true)),
            properties_read: false,
            devlinks: Rc::new(udev_list::new(true)),
            devlinks_read: false,
            sysattrs: Rc::new(udev_list::new(true)),
            sysattrs_read: false,
            parent: std::ptr::null_mut(),
        }
    }
}

#[no_mangle]
#[append_impl]
/// udev_device_new_from_device_id
pub extern "C" fn udev_device_new_from_device_id(
    udev: *mut udev,
    id: *const ::std::os::raw::c_char,
) -> *mut udev_device {
    assert_return!(!id.is_null(), {
        errno::set_errno(errno::Errno(libc::EINVAL));
        std::ptr::null_mut()
    });

    let id = unsafe { CStr::from_ptr(id as *const c_char) };

    let s = match id.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let device = match Device::from_device_id(s) {
        Ok(d) => Rc::new(d),
        Err(_) => return std::ptr::null_mut(),
    };

    Rc::into_raw(Rc::new(udev_device::new(udev, device))) as *mut udev_device
}

#[no_mangle]
#[append_impl]
/// udev_device_new_from_devnum
pub extern "C" fn udev_device_new_from_devnum(
    udev: *mut udev,
    type_: ::std::os::raw::c_char,
    devnum: dev_t,
) -> *mut udev_device {
    assert_return!(type_ == 'b' as c_char || type_ == 'c' as c_char, {
        errno::set_errno(errno::Errno(libc::EINVAL));
        std::ptr::null_mut()
    });

    let device = match Device::from_devnum(type_ as u8 as char, devnum) {
        Ok(d) => Rc::new(d),
        Err(_) => return std::ptr::null_mut(),
    };

    Rc::into_raw(Rc::new(udev_device::new(udev, device))) as *mut udev_device
}

#[no_mangle]
#[append_impl]
/// udev_device_new_from_subsystem_sysname
pub extern "C" fn udev_device_new_from_subsystem_sysname(
    udev: *mut udev,
    subsystem: *const ::std::os::raw::c_char,
    sysname: *const ::std::os::raw::c_char,
) -> *mut udev_device {
    assert_return!(!subsystem.is_null(), {
        errno::set_errno(errno::Errno(libc::EINVAL));
        std::ptr::null_mut()
    });

    assert_return!(!sysname.is_null(), {
        errno::set_errno(errno::Errno(libc::EINVAL));
        std::ptr::null_mut()
    });

    let subsystem = unsafe { CStr::from_ptr(subsystem as *const c_char) }
        .to_str()
        .unwrap();
    let sysname = unsafe { CStr::from_ptr(sysname as *const c_char) }
        .to_str()
        .unwrap();
    let device = match Device::from_subsystem_sysname(subsystem, sysname) {
        Ok(d) => Rc::new(d),
        Err(_) => {
            return std::ptr::null_mut();
        }
    };

    Rc::into_raw(Rc::new(udev_device::new(udev, device))) as *mut udev_device
}

#[no_mangle]
#[append_impl]
/// udev_device_new_from_syspath
pub extern "C" fn udev_device_new_from_syspath(
    udev: *mut udev,
    syspath: *const ::std::os::raw::c_char,
) -> *mut udev_device {
    assert_return!(!syspath.is_null(), {
        errno::set_errno(errno::Errno(libc::EINVAL));
        std::ptr::null_mut()
    });

    let syspath = unsafe { CStr::from_ptr(syspath as *const c_char) }
        .to_str()
        .unwrap();
    let device = match Device::from_syspath(syspath, true) {
        Ok(d) => Rc::new(d),
        Err(_) => {
            return std::ptr::null_mut();
        }
    };

    Rc::into_raw(Rc::new(udev_device::new(udev, device))) as *mut udev_device
}

#[no_mangle]
#[append_impl]
/// udev_device_new_from_environment
pub fn udev_device_new_from_environment(udev: *mut udev) -> *mut udev_device {
    let device = match Device::from_environment() {
        Ok(d) => Rc::new(d),
        Err(e) => {
            errno::set_errno(errno::Errno(e.get_errno() as i32));
            return std::ptr::null_mut();
        }
    };

    Rc::into_raw(Rc::new(udev_device::new(udev, device))) as *mut _
}

#[no_mangle]
#[append_impl]
/// udev_device_get_syspath
pub extern "C" fn udev_device_get_syspath(
    udev_device: *mut udev_device,
) -> *const ::std::os::raw::c_char {
    assert_return!(!udev_device.is_null(), {
        errno::set_errno(errno::Errno(libc::EINVAL));
        std::ptr::null()
    });

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
#[append_impl]
/// udev_device_has_tag
pub extern "C" fn udev_device_has_tag(
    udev_device: *mut udev_device,
    tag: *const ::std::os::raw::c_char,
) -> ::std::os::raw::c_int {
    assert_return!(!udev_device.is_null() && !tag.is_null(), {
        errno::set_errno(errno::Errno(libc::EINVAL));
        0
    });

    let udev_device_mut: &mut udev_device = unsafe { mem::transmute(&mut *udev_device) };

    let tag = unsafe { CStr::from_ptr(tag) }.to_str().unwrap();

    match udev_device_mut.device.has_tag(tag) {
        Ok(true) => 1,
        _ => 0,
    }
}

#[no_mangle]
#[append_impl]
/// udev_device_has_current_tag
pub extern "C" fn udev_device_has_current_tag(
    udev_device: *mut udev_device,
    tag: *const ::std::os::raw::c_char,
) -> ::std::os::raw::c_int {
    assert_return!(!udev_device.is_null() && !tag.is_null(), {
        errno::set_errno(errno::Errno(libc::EINVAL));
        0
    });

    let udev_device_mut: &mut udev_device = unsafe { mem::transmute(&mut *udev_device) };

    let tag = unsafe { CStr::from_ptr(tag) }.to_str().unwrap();

    match udev_device_mut.device.has_current_tag(tag) {
        Ok(true) => 1,
        _ => 0,
    }
}

#[no_mangle]
#[append_impl]
/// udev_device_get_action
pub extern "C" fn udev_device_get_action(
    udev_device: *mut udev_device,
) -> *const ::std::os::raw::c_char {
    assert_return!(!udev_device.is_null(), {
        errno::set_errno(errno::Errno(libc::EINVAL));
        std::ptr::null()
    });

    let udev_device_mut: &mut udev_device = unsafe { mem::transmute(&mut *udev_device) };

    if let Ok(action) = udev_device_mut.device.get_action() {
        let ret = ACTION_STRING_TABLE[action as usize];
        return ret.as_ptr() as *const ::std::os::raw::c_char;
    }

    std::ptr::null()
}

#[no_mangle]
#[append_impl]
/// udev_device_get_devnode
pub extern "C" fn udev_device_get_devnode(
    udev_device: *mut udev_device,
) -> *const ::std::os::raw::c_char {
    assert_return!(!udev_device.is_null(), {
        errno::set_errno(errno::Errno(libc::EINVAL));
        std::ptr::null()
    });

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
#[append_impl]
/// udev_device_get_devnum
pub extern "C" fn udev_device_get_devnum(udev_device: *mut udev_device) -> dev_t {
    assert_return!(!udev_device.is_null(), {
        errno::set_errno(errno::Errno(libc::EINVAL));
        0
    });

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
#[append_impl]
/// udev_device_get_devpath
pub extern "C" fn udev_device_get_devpath(
    udev_device: *mut udev_device,
) -> *const ::std::os::raw::c_char {
    assert_return!(!udev_device.is_null(), {
        errno::set_errno(errno::Errno(libc::EINVAL));
        std::ptr::null()
    });

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
#[append_impl]
/// udev_device_get_devtype
pub extern "C" fn udev_device_get_devtype(
    udev_device: *mut udev_device,
) -> *const ::std::os::raw::c_char {
    assert_return!(!udev_device.is_null(), {
        errno::set_errno(errno::Errno(libc::EINVAL));
        std::ptr::null()
    });

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
#[append_impl]
/// udev_device_get_driver
pub extern "C" fn udev_device_get_driver(
    udev_device: *mut udev_device,
) -> *const ::std::os::raw::c_char {
    assert_return!(!udev_device.is_null(), {
        errno::set_errno(errno::Errno(libc::EINVAL));
        std::ptr::null()
    });

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
#[append_impl]
/// udev_device_get_sysname
pub extern "C" fn udev_device_get_sysname(
    udev_device: *mut udev_device,
) -> *const ::std::os::raw::c_char {
    assert_return!(!udev_device.is_null(), {
        errno::set_errno(errno::Errno(libc::EINVAL));
        std::ptr::null()
    });

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
#[append_impl]
/// udev_device_get_subsystem
pub extern "C" fn udev_device_get_subsystem(
    udev_device: *mut udev_device,
) -> *const ::std::os::raw::c_char {
    assert_return!(!udev_device.is_null(), {
        errno::set_errno(errno::Errno(libc::EINVAL));
        std::ptr::null()
    });

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
#[append_impl]
/// udev_device_get_seqnum
pub extern "C" fn udev_device_get_seqnum(
    udev_device: *mut udev_device,
) -> ::std::os::raw::c_ulonglong {
    assert_return!(!udev_device.is_null(), {
        errno::set_errno(errno::Errno(libc::EINVAL));
        0
    });

    let udev_device_mut: &mut udev_device = unsafe { mem::transmute(&mut *udev_device) };

    udev_device_mut.device.get_seqnum().unwrap_or_default() as ::std::os::raw::c_ulonglong
}

#[no_mangle]
#[append_impl]
/// udev_device_get_property_value
pub extern "C" fn udev_device_get_property_value(
    udev_device: *mut udev_device,
    key: *const ::std::os::raw::c_char,
) -> *const ::std::os::raw::c_char {
    assert_return!(!udev_device.is_null() && !key.is_null(), {
        errno::set_errno(errno::Errno(libc::EINVAL));
        std::ptr::null_mut()
    });

    let key = unsafe { CStr::from_ptr(key) };
    let udev_device_mut: &mut udev_device = unsafe { mem::transmute(&mut *udev_device) };

    if !udev_device_mut.properties_read {
        for (k, v) in &udev_device_mut.device.property_iter() {
            let key = CString::new(k.as_str()).unwrap();
            let value = CString::new(v.as_str()).unwrap();
            udev_device_mut.properties.add_entry(key, value);
        }
        udev_device_mut.properties_read = true;
    }

    match udev_device_mut.properties.unique_entries.borrow().get(key) {
        Some(v) => v.value.as_ptr(),
        None => {
            errno::set_errno(errno::Errno(libc::ENODATA as i32));
            std::ptr::null()
        }
    }
}

#[no_mangle]
#[append_impl]
/// udev_device_get_properties_list_entry
pub extern "C" fn udev_device_get_properties_list_entry(
    udev_device: *mut udev_device,
) -> *mut udev_list_entry {
    assert_return!(!udev_device.is_null(), {
        errno::set_errno(errno::Errno(libc::EINVAL));
        std::ptr::null_mut()
    });

    let d: &mut udev_device = unsafe { mem::transmute(&mut *udev_device) };

    if !d.properties_read {
        for (k, v) in &d.device.property_iter() {
            let key = CString::new(k.as_str()).unwrap();
            let value = CString::new(v.as_str()).unwrap();
            d.properties.add_entry(key, value);
        }
        d.properties_read = true;
    }

    d.properties.get_entry()
}

fn device_new_from_parent(child: *mut udev_device) -> *mut udev_device {
    assert_return!(!child.is_null(), {
        errno::set_errno(errno::Errno(libc::EINVAL));
        std::ptr::null_mut()
    });

    let ud: &mut udev_device = unsafe { mem::transmute(&mut *child) };

    match ud.device.get_parent() {
        Ok(p) => Rc::into_raw(Rc::new(udev_device::new(ud.udev, p))) as *mut udev_device,
        Err(e) => {
            errno::set_errno(errno::Errno(e.get_errno() as i32));
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
#[append_impl]
/// udev_device_get_parent
///
/// return the reference of the innter parent field, thus don't drop it
pub extern "C" fn udev_device_get_parent(udev_device: *mut udev_device) -> *mut udev_device {
    assert_return!(!udev_device.is_null(), {
        errno::set_errno(errno::Errno(libc::EINVAL));
        std::ptr::null_mut()
    });

    let ud: &mut udev_device = unsafe { mem::transmute(&mut *udev_device) };

    if ud.parent.is_null() {
        ud.parent = device_new_from_parent(udev_device);
    }

    ud.parent
}

#[no_mangle]
#[append_impl]
/// udev_device_get_parent_with_subsystem_devtype
pub extern "C" fn udev_device_get_parent_with_subsystem_devtype(
    udev_device: *mut udev_device,
    subsystem: *const ::std::os::raw::c_char,
    devtype: *const ::std::os::raw::c_char,
) -> *mut udev_device {
    assert_return!(!udev_device.is_null(), {
        errno::set_errno(errno::Errno(libc::EINVAL));
        std::ptr::null_mut()
    });
    assert_return!(!subsystem.is_null(), {
        errno::set_errno(errno::Errno(libc::EINVAL));
        std::ptr::null_mut()
    });

    let ud: &mut udev_device = unsafe { mem::transmute(&mut *udev_device) };

    let subsystem = unsafe { CStr::from_ptr(subsystem).to_str().unwrap_or_default() };

    let devtype = if devtype.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(devtype).to_str().ok() }
    };

    let p = match ud
        .device
        .get_parent_with_subsystem_devtype(subsystem, devtype)
    {
        Ok(p) => p,
        Err(e) => {
            errno::set_errno(errno::Errno(e.get_errno() as i32));
            return std::ptr::null_mut();
        }
    };

    #[allow(clippy::never_loop)]
    loop {
        let udev_device = udev_device_get_parent_impl(udev_device);

        if udev_device.is_null() {
            break;
        }

        let ud: &mut udev_device = unsafe { mem::transmute(&mut *udev_device) };

        if ud.device == p {
            return udev_device;
        }
    }

    errno::set_errno(errno::Errno(libc::ENOENT as i32));
    std::ptr::null_mut()
}

#[no_mangle]
#[append_impl]
/// udev_device_get_is_initialized
pub extern "C" fn udev_device_get_is_initialized(
    udev_device: *mut udev_device,
) -> ::std::os::raw::c_int {
    assert_return!(!udev_device.is_null(), {
        errno::set_errno(errno::Errno(libc::EINVAL));
        -libc::EINVAL
    });

    let ud: &mut udev_device = unsafe { mem::transmute(&mut *udev_device) };

    match ud.device.get_is_initialized() {
        Ok(r) => {
            if r {
                1
            } else {
                0
            }
        }
        Err(e) => {
            errno::set_errno(errno::Errno(e.get_errno() as i32));
            0
        }
    }
}

#[no_mangle]
#[append_impl]
/// udev_device_get_devlinks_list_entry
pub extern "C" fn udev_device_get_devlinks_list_entry(
    udev_device: *mut udev_device,
) -> *mut udev_list_entry {
    assert_return!(!udev_device.is_null(), {
        errno::set_errno(errno::Errno(libc::EINVAL));
        std::ptr::null_mut()
    });

    let ud: &mut udev_device = unsafe { mem::transmute(&mut *udev_device) };

    if !ud.devlinks_read {
        ud.devlinks.cleanup();

        for devlink in &ud.device.devlink_iter() {
            let devlink_cstr = CString::new(devlink.as_str()).unwrap();
            ud.devlinks.add_entry(devlink_cstr, CString::default());
        }

        ud.devlinks_read = true;
    }

    ud.devlinks.get_entry()
}

#[no_mangle]
#[append_impl]
/// udev_device_get_sysattr_value
pub extern "C" fn udev_device_get_sysattr_value(
    udev_device: *mut udev_device,
    sysattr: *const ::std::os::raw::c_char,
) -> *const ::std::os::raw::c_char {
    assert_return!(!udev_device.is_null() && !sysattr.is_null(), {
        errno::set_errno(errno::Errno(libc::EINVAL));
        std::ptr::null_mut()
    });

    let sysattr = unsafe { CStr::from_ptr(sysattr) };
    let udev_device_mut: &mut udev_device = unsafe { mem::transmute(&mut *udev_device) };

    let sysattr_string: String = sysattr.to_str().unwrap().to_string();
    match udev_device_mut
        .device
        .get_sysattr_value(sysattr_string.as_str())
    {
        Ok(v) => {
            let entry = udev_device_mut.sysattrs.add_entry(
                CString::new(sysattr_string.as_str()).unwrap(),
                CString::new(v.as_str()).unwrap(),
            );
            entry.value.as_ptr()
        }
        Err(e) => {
            errno::set_errno(errno::Errno(e.get_errno() as i32));
            std::ptr::null()
        }
    }
}

#[cfg(test)]
mod test {
    use std::{intrinsics::transmute, os::raw::c_char};

    use crate::libudev_list::{udev_list_entry_get_name_impl, udev_list_entry_get_next_impl};

    use super::*;
    use device::device_enumerator::*;

    type RD = Rc<Device>;
    pub type Result = std::result::Result<(), device::error::Error>;

    fn test_udev_device_new_from_devnum(device: RD) -> Result {
        let devnum = device.get_devnum()?;

        let t = if &device.get_subsystem().unwrap() == "block" {
            'b'
        } else {
            'c'
        };

        let raw_udev_device = udev_device_new_from_devnum_impl(
            std::ptr::null_mut(),
            t as ::std::os::raw::c_char,
            devnum,
        );

        let syspath = unsafe { CStr::from_ptr(udev_device_get_syspath_impl(raw_udev_device)) };

        assert_eq!(syspath.to_str().unwrap(), &device.get_syspath().unwrap());

        udev_device_unref_impl(raw_udev_device);

        Ok(())
    }

    fn test_udev_device_new_from_subsystem_sysname(device: RD) -> Result {
        let subsystem = device.get_subsystem().unwrap();
        let sysname = device.get_sysname().unwrap();

        /* If subsystem is 'drivers', sysname should use '<driver subsystem>:<sysname>'. */
        let name = if subsystem == "drivers" {
            format!("{}:{}", device.driver_subsystem.borrow(), sysname)
        } else {
            sysname
        };

        let c_subsystem = CString::new(subsystem).unwrap();
        let c_name = CString::new(name).unwrap();
        let raw_udev_device = udev_device_new_from_subsystem_sysname_impl(
            std::ptr::null_mut(),
            c_subsystem.as_ptr(),
            c_name.as_ptr(),
        );

        let syspath = unsafe { CStr::from_ptr(udev_device_get_syspath_impl(raw_udev_device)) };

        assert_eq!(syspath.to_str().unwrap(), &device.get_syspath().unwrap());

        udev_device_unref_impl(raw_udev_device);

        Ok(())
    }

    fn test_udev_device_new_from_device_id(device: RD) -> Result {
        let id = device.get_device_id().unwrap();
        let c_id = CString::new(id).unwrap();
        let udev_device = udev_device_new_from_device_id_impl(std::ptr::null_mut(), c_id.as_ptr());
        let syspath = unsafe { CStr::from_ptr(udev_device_get_syspath_impl(udev_device)) };

        assert_eq!(
            device.get_syspath().unwrap(),
            syspath.to_str().unwrap().to_string()
        );

        udev_device_unref_impl(udev_device);

        Ok(())
    }

    fn test_udev_device_new_from_syspath(device: RD) -> Result {
        let syspath = device.get_syspath().unwrap();
        let c_syspath = CString::new(syspath).unwrap();
        let udev_device =
            udev_device_new_from_syspath_impl(std::ptr::null_mut(), c_syspath.as_ptr());
        let ret_syspath = unsafe { CStr::from_ptr(udev_device_get_syspath_impl(udev_device)) };

        assert_eq!(c_syspath.to_str().unwrap(), ret_syspath.to_str().unwrap());

        udev_device_unref_impl(udev_device);

        Ok(())
    }

    fn from_rd(device: RD) -> *mut udev_device {
        let id = device.get_device_id().unwrap();
        let c_id = CString::new(id).unwrap();
        udev_device_new_from_device_id_impl(std::ptr::null_mut(), c_id.as_ptr())
    }

    fn test_udev_device_has_tag(device: RD) -> Result {
        let _ = device.read_db_internal(true);
        device.add_tag("test_udev_device_has_tag", true);
        device.update_db()?;

        let udev_device = from_rd(device.clone());

        assert!(
            udev_device_has_tag_impl(
                udev_device,
                "test_udev_device_has_tag\0".as_ptr() as *const c_char
            ) > 0
        );

        assert!(
            udev_device_has_current_tag_impl(
                udev_device,
                "test_udev_device_has_tag\0".as_ptr() as *const c_char
            ) > 0
        );

        udev_device_unref_impl(udev_device);

        device
            .all_tags
            .borrow_mut()
            .remove("test_udev_device_has_tag");
        device.remove_tag("test_udev_device_has_tag");
        device.update_db()?;

        Ok(())
    }

    fn test_udev_device_get_action(dev: RD) -> Result {
        let udev_device = from_rd(dev);

        let udev_device_mut: &mut udev_device = unsafe { transmute(&mut *udev_device) };

        udev_device_mut
            .device
            .set_action_from_string("change")
            .unwrap();

        let ptr = udev_device_get_action_impl(udev_device);

        let action = unsafe { CStr::from_ptr(ptr) };

        assert_eq!(action.to_str().unwrap(), "change");

        udev_device_unref_impl(udev_device);

        Ok(())
    }

    fn test_udev_device_get_devnode(dev: RD) -> Result {
        let udev_device = from_rd(dev.clone());

        let devnode = dev.get_devname()?;

        let ptr = udev_device_get_devnode_impl(udev_device);

        assert_eq!(unsafe { CStr::from_ptr(ptr) }.to_str().unwrap(), &devnode);

        udev_device_unref_impl(udev_device);

        Ok(())
    }

    fn test_udev_device_get_devnum(dev: RD) -> Result {
        let udev_device = from_rd(dev.clone());

        let devnum = dev.get_devnum()?;

        assert_eq!(udev_device_get_devnum_impl(udev_device), devnum);

        Ok(())
    }

    fn test_udev_device_get_devpath(dev: RD) -> Result {
        let udev_device = from_rd(dev.clone());

        let devpath = dev.get_devpath()?;

        assert_eq!(
            unsafe { CStr::from_ptr(udev_device_get_devpath_impl(udev_device)) }
                .to_str()
                .unwrap(),
            &devpath
        );

        Ok(())
    }

    fn test_udev_device_get_devtype(dev: RD) -> Result {
        let ud = from_rd(dev.clone());

        let devtype = dev.get_devtype()?;

        assert_eq!(
            unsafe { CStr::from_ptr(udev_device_get_devtype_impl(ud)) }
                .to_str()
                .unwrap(),
            &devtype
        );

        Ok(())
    }

    fn test_udev_device_get_driver(dev: RD) -> Result {
        let ud = from_rd(dev.clone());

        let driver = dev.get_driver()?;

        assert_eq!(
            unsafe { CStr::from_ptr(udev_device_get_driver_impl(ud)) }
                .to_str()
                .unwrap(),
            &driver
        );

        Ok(())
    }

    fn test_udev_device_get_sysname(dev: RD) -> Result {
        let ud = from_rd(dev.clone());

        let sysname = dev.get_sysname()?;

        assert_eq!(
            unsafe { CStr::from_ptr(udev_device_get_sysname_impl(ud)) }
                .to_str()
                .unwrap(),
            &sysname
        );

        Ok(())
    }

    fn test_udev_device_get_subsystem(dev: RD) -> Result {
        let ud = from_rd(dev.clone());

        let subsystem = dev.get_subsystem()?;

        assert_eq!(
            unsafe { CStr::from_ptr(udev_device_get_subsystem_impl(ud)) }
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

        assert_eq!(udev_device_get_seqnum_impl(ud), 10000);

        Ok(())
    }

    fn test_udev_device_get_property_value(dev: RD) -> Result {
        let ud = from_rd(dev);
        let ud_mut: &mut udev_device = unsafe { transmute(&mut *ud) };
        ud_mut.device.sealed.replace(true);
        ud_mut.device.add_property("hello", "world").unwrap();

        assert_eq!(
            unsafe {
                CStr::from_ptr(udev_device_get_property_value_impl(
                    ud,
                    "hello\0".as_ptr() as *const c_char,
                ))
            }
            .to_str()
            .unwrap(),
            "world"
        );

        Ok(())
    }

    fn test_udev_device_get_parent(dev: RD) -> Result {
        if &dev.get_devtype()? != "partition" {
            return Ok(());
        }

        let ud = from_rd(dev.clone());

        let p = udev_device_get_parent_impl(ud);

        assert!(!p.is_null());

        let p_rc = dev.get_parent().unwrap();

        assert_eq!(
            unsafe { CStr::from_ptr(udev_device_get_syspath_impl(p)) }
                .to_str()
                .unwrap(),
            &p_rc.get_syspath().unwrap()
        );

        Ok(())
    }

    fn test_udev_device_get_parent_with_subsystem_devtype(dev: RD) -> Result {
        if &dev.get_devtype()? == "partition" {
            let p = dev
                .get_parent_with_subsystem_devtype("block", Some("disk"))
                .unwrap();

            assert_eq!(&p.get_devtype()?, "disk");

            let ud = from_rd(dev);
            let pud = udev_device_get_parent_with_subsystem_devtype_impl(
                ud,
                "block\0".as_ptr() as *const c_char,
                "disk\0".as_ptr() as *const c_char,
            );

            assert!(!pud.is_null());
            let ud_mut: &mut udev_device = unsafe { transmute(&mut *ud) };
            let p1 = ud_mut
                .device
                .get_parent_with_subsystem_devtype("block", Some("disk"))
                .unwrap();
            let pud_mut: &mut udev_device = unsafe { transmute(&mut *pud) };
            let p2: Rc<Device> = pud_mut.device.clone();

            // The parent device object of ud should be the device object of pud.
            let r1 = Rc::into_raw(p1);
            let r2 = Rc::into_raw(p2);
            assert_eq!(r1, r2);

            let _ = unsafe { Rc::from_raw(r1) };
            let _ = unsafe { Rc::from_raw(r2) };
        }

        Ok(())
    }

    fn test_udev_device_get_is_initialized(dev: RD) -> Result {
        let ud = from_rd(dev);
        let ud_mut: &mut udev_device = unsafe { transmute(&mut *ud) };

        ud_mut.device.set_is_initialized();

        assert!(udev_device_get_is_initialized_impl(ud) > 0);

        Ok(())
    }

    fn test_udev_device_get_devlinks_list_entry(dev: RD) -> Result {
        dev.read_db_internal(true)?;
        let ud = from_rd(dev.clone());

        let mut entry = udev_device_get_devlinks_list_entry_impl(ud);

        loop {
            if entry.is_null() {
                break;
            }

            let link_c = unsafe { CStr::from_ptr(udev_list_entry_get_name_impl(entry)) };

            let link = link_c.to_str().unwrap();
            assert!(dev.has_devlink(link));

            entry = udev_list_entry_get_next_impl(entry);
        }

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
            let _ = test_udev_device_get_is_initialized(dev.clone());
        }
    }

    #[test]
    fn test_udev_device_has_tag_ut() {
        let dev = Rc::new(Device::from_subsystem_sysname("net", "lo").unwrap());
        let _ = test_udev_device_has_tag(dev);
    }

    #[test]
    fn test_udev_device_fake_from_monitor_ut() {
        let dev = Rc::new(Device::from_subsystem_sysname("net", "lo").unwrap());
        let _ = test_udev_device_get_action(dev.clone());
        let _ = test_udev_device_get_seqnum(dev.clone());
        let _ = test_udev_device_get_property_value(dev);
    }

    #[test]
    fn test_enumerate_block() {
        let mut e = DeviceEnumerator::new();
        e.set_enumerator_type(DeviceEnumerationType::Devices);
        e.add_match_subsystem("block", true).unwrap();

        for dev in e.iter() {
            let _ = test_udev_device_get_parent_with_subsystem_devtype(dev.clone());
            let _ = test_udev_device_get_parent(dev.clone());
            let _ = test_udev_device_get_devlinks_list_entry(dev.clone());
        }
    }
}
