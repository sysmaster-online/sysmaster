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

use crate::assert_return;
use crate::libudev::*;
use crate::libudev_device::udev_device;
use device::device_monitor::*;
use libudev_macro::append_impl;
use libudev_macro::RefUnref;
use std::cell::RefCell;
use std::ffi::CStr;
use std::intrinsics::transmute;
use std::rc::Rc;

#[repr(C)]
#[derive(Clone, RefUnref)]
/// udev_monitor
pub struct udev_monitor {
    pub(crate) udev: *mut udev,
    pub(crate) monitor: Rc<RefCell<DeviceMonitor>>,
}

#[no_mangle]
#[append_impl]
/// udev_monitor_new_from_netlink
pub extern "C" fn udev_monitor_new_from_netlink(
    udev: *mut udev,
    name: *const ::std::os::raw::c_char,
) -> *mut udev_monitor {
    assert_return!(!name.is_null(), {
        errno::set_errno(errno::Errno(libc::EINVAL));
        std::ptr::null_mut()
    });

    let name = unsafe { CStr::from_ptr(name) }.to_str().unwrap();

    let g = match name {
        "kernel" => MonitorNetlinkGroup::Kernel,
        "udev" => MonitorNetlinkGroup::Userspace,
        _ => {
            if name.is_empty() {
                MonitorNetlinkGroup::None
            } else {
                errno::set_errno(errno::Errno(libc::EINVAL));
                return std::ptr::null_mut();
            }
        }
    };

    let monitor = Rc::new(RefCell::new(DeviceMonitor::new(g, None)));

    Rc::into_raw(Rc::new(udev_monitor { udev, monitor })) as *mut udev_monitor
}

#[no_mangle]
#[append_impl]
/// udev_monitor_enable_receiving
pub extern "C" fn udev_monitor_enable_receiving(
    udev_monitor: *mut udev_monitor,
) -> ::std::os::raw::c_int {
    assert_return!(!udev_monitor.is_null(), -libc::EINVAL);

    let m: &mut udev_monitor = unsafe { transmute(&mut *udev_monitor) };

    match m.monitor.borrow_mut().bpf_filter_update() {
        Ok(_) => 0,
        Err(e) => e.get_errno() as i32,
    }
}

#[no_mangle]
#[append_impl]
/// udev_monitor_filter_add_match_subsystem_devtype
pub extern "C" fn udev_monitor_filter_add_match_subsystem_devtype(
    udev_monitor: *mut udev_monitor,
    subsystem: *const ::std::os::raw::c_char,
    devtype: *const ::std::os::raw::c_char,
) -> ::std::os::raw::c_int {
    assert_return!(
        !udev_monitor.is_null() && !subsystem.is_null(),
        -libc::EINVAL
    );

    let m: &mut udev_monitor = unsafe { transmute(&mut *udev_monitor) };

    let subsystem = unsafe { CStr::from_ptr(subsystem) }
        .to_str()
        .unwrap_or_default();

    let devtype = if devtype.is_null() {
        ""
    } else {
        unsafe { CStr::from_ptr(devtype) }
            .to_str()
            .unwrap_or_default()
    };

    if let Err(e) = m
        .monitor
        .borrow_mut()
        .filter_add_match_subsystem_devtype(subsystem, devtype)
    {
        return e.get_errno() as i32;
    }

    0
}

#[no_mangle]
#[append_impl]
/// udev_monitor_filter_add_match_tag
pub extern "C" fn udev_monitor_filter_add_match_tag(
    udev_monitor: *mut udev_monitor,
    tag: *const ::std::os::raw::c_char,
) -> ::std::os::raw::c_int {
    assert_return!(!udev_monitor.is_null() && !tag.is_null(), -libc::EINVAL);

    let m: &mut udev_monitor = unsafe { transmute(&mut *udev_monitor) };

    let tag = unsafe { CStr::from_ptr(tag) }.to_str().unwrap_or_default();

    if let Err(e) = m.monitor.borrow_mut().filter_add_match_tag(tag) {
        return e.get_errno() as i32;
    }

    0
}

#[no_mangle]
#[append_impl]
/// udev_monitor_get_fd
pub extern "C" fn udev_monitor_get_fd(udev_monitor: *mut udev_monitor) -> ::std::os::raw::c_int {
    assert_return!(!udev_monitor.is_null(), -libc::EINVAL);

    let m: &mut udev_monitor = unsafe { transmute(&mut *udev_monitor) };

    m.monitor.borrow().fd()
}

#[no_mangle]
#[append_impl]
/// udev_monitor_get_udev
pub extern "C" fn udev_monitor_get_udev(udev_monitor: *mut udev_monitor) -> *mut udev {
    let m: &mut udev_monitor = unsafe { transmute(&mut *udev_monitor) };
    m.udev
}

#[no_mangle]
#[append_impl]
/// udev_monitor_receive_device
pub extern "C" fn udev_monitor_receive_device(udev_monitor: *mut udev_monitor) -> *mut udev_device {
    let m: &mut udev_monitor = unsafe { transmute(&mut *udev_monitor) };

    loop {
        match m.monitor.borrow().receive_device() {
            Ok(Some(d)) => {
                return Rc::into_raw(Rc::new(udev_device::new(m.udev, Rc::new(d))))
                    as *mut udev_device
            }
            Err(e) => {
                errno::set_errno(errno::Errno(e.get_errno() as i32));
                return std::ptr::null_mut();
            }
            _ => continue,
        }
    }
}

#[no_mangle]
#[append_impl]
/// udev_monitor_set_receive_buffer_size
pub extern "C" fn udev_monitor_set_receive_buffer_size(
    udev_monitor: *mut udev_monitor,
    size: ::std::os::raw::c_int,
) -> ::std::os::raw::c_int {
    let m: &mut udev_monitor = unsafe { transmute(&mut *udev_monitor) };

    if let Err(e) = basic::socket::set_receive_buffer(m.monitor.borrow().fd(), size as usize) {
        return e.get_errno() as i32;
    }

    0
}
