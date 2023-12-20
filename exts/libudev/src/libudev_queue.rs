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

use crate::libudev::*;
use libudev_macro::append_impl;
use libudev_macro::RefUnref;
use std::path::Path;
use std::rc::Rc;

#[repr(C)]
#[derive(Clone, RefUnref)]
/// udev_queue
pub struct udev_queue {
    pub(crate) udev: *mut udev,
    pub(crate) fd: i32,
}

#[no_mangle]
#[append_impl]
/// udev_queue_new
pub extern "C" fn udev_queue_new(udev: *mut udev) -> *mut udev_queue {
    Rc::into_raw(Rc::new(udev_queue {
        udev,
        fd: -libc::EBADF,
    })) as *mut _
}

#[no_mangle]
#[append_impl]
/// udev_queue_get_udev_is_active
///
/// This function detects whether devmaster is running rather than udevd.
pub extern "C" fn udev_queue_get_udev_is_active(
    _udev_queue: *mut udev_queue,
) -> ::std::os::raw::c_int {
    Path::new("/run/devmaster/control").exists() as i32
}
