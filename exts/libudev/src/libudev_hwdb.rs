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

use hwdb::SdHwdb;
use libudev_macro::append_impl;
use libudev_macro::RefUnref;
use std::{
    cell::RefCell,
    ffi::{CStr, CString},
    intrinsics::transmute,
    rc::Rc,
};

use crate::assert_return;
use crate::{
    libudev::udev,
    libudev_list::{udev_list, udev_list_entry},
};

#[repr(C)]
#[derive(Clone, RefUnref)]
/// udev_hwdb
pub struct udev_hwdb {
    pub(crate) hwdb: Rc<RefCell<SdHwdb>>,
    pub(crate) properties: Rc<udev_list>,
}

#[no_mangle]
#[append_impl]
/// udev_hwdb_new
pub extern "C" fn udev_hwdb_new(_udev: *mut udev) -> *mut udev_hwdb {
    let hwdb = match SdHwdb::new() {
        Ok(h) => h,
        Err(e) => {
            errno::set_errno(errno::Errno(e as i32));
            return std::ptr::null_mut();
        }
    };

    Rc::into_raw(Rc::new(udev_hwdb {
        hwdb: Rc::new(RefCell::new(hwdb)),
        properties: Rc::new(udev_list::new(true)),
    })) as *mut _
}

#[no_mangle]
#[append_impl]
/// udev_hwdb_get_properties_list_entry
pub extern "C" fn udev_hwdb_get_properties_list_entry(
    hwdb: *mut udev_hwdb,
    modalias: *const ::std::os::raw::c_char,
    _flags: ::std::os::raw::c_uint,
) -> *mut udev_list_entry {
    assert_return!(!hwdb.is_null() && !modalias.is_null(), {
        errno::set_errno(errno::Errno(libc::EINVAL));
        std::ptr::null_mut()
    });

    let h: &mut udev_hwdb = unsafe { transmute(&mut *hwdb) };

    h.properties.cleanup();

    let modalias = unsafe { CStr::from_ptr(modalias) }.to_str().unwrap();

    if let Ok(properties) = h.hwdb.borrow_mut().get_properties(modalias.to_string()) {
        for (k, v) in properties.iter() {
            h.properties.add_entry(
                CString::new(k.as_str()).unwrap(),
                CString::new(v.as_str()).unwrap(),
            );
        }
    }

    let e = h.properties.get_entry();

    if e.is_null() {
        errno::set_errno(errno::Errno(libc::ENODATA));
    }

    e
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::libudev_list::*;

    #[test]
    fn test_udev_hwdb() {
        let hwdb = udev_hwdb_new_impl(std::ptr::null_mut());
        let mut i = 0;
        loop {
            let mut list = udev_hwdb_get_properties_list_entry_impl(
                hwdb,
                "evdev:input:b0003v0458p07081\0".as_ptr() as *const _,
                0,
            );
            loop {
                if list.is_null() {
                    break;
                }
                let name = udev_list_entry_get_name_impl(list);
                let value = udev_list_entry_get_value_impl(list);
                let name = unsafe { CStr::from_ptr(name) };
                let value = unsafe { CStr::from_ptr(value) };
                println!("{:?}={:?}", name, value);
                list = udev_list_entry_get_next_impl(list);
            }
            i += 1;
            if i == 10 {
                break;
            }
        }
        udev_hwdb_unref_impl(hwdb);
    }
}
