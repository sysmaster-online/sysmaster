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

use libudev_macro::append_impl;
use libudev_macro::RefUnref;
use std::ffi::c_void;
use std::rc::Rc;

#[repr(C)]
#[derive(Debug, Clone, RefUnref)]
/// udev
///
/// userdata points to an stored data object, it does not own the lifetime of the object
/// and might be useful to access from callbacks.
pub struct udev {
    pub(crate) userdata: *mut c_void,
}

#[no_mangle]
#[append_impl]
/// udev_new
pub extern "C" fn udev_new() -> *mut udev {
    Rc::into_raw(Rc::new(udev {
        userdata: std::ptr::null_mut(),
    })) as *mut _
}
