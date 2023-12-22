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
use std::{
    cell::RefCell,
    cmp::Ordering,
    collections,
    ffi::{CStr, CString},
    rc::{Rc, Weak},
};

#[repr(C)]
#[derive(Debug, Clone, RefUnref)]
/// udev_list_entry
pub struct udev_list_entry {
    pub(crate) list: Weak<udev_list>,
    pub(crate) name: CString,
    pub(crate) value: CString,
}

#[repr(C)]
#[derive(Debug, Clone)]
/// udev_list_entry
pub struct udev_list {
    pub(crate) unique_entries: RefCell<collections::HashMap<CString, Rc<udev_list_entry>>>,
    pub(crate) entries: RefCell<Vec<Rc<udev_list_entry>>>,
    pub(crate) idx: RefCell<usize>,

    pub(crate) unique: bool,
    pub(crate) up_to_date: RefCell<bool>,
}

impl udev_list {
    pub(crate) fn new(unique: bool) -> Self {
        Self {
            unique_entries: RefCell::new(collections::HashMap::default()),
            entries: RefCell::new(Vec::default()),
            idx: RefCell::new(0),
            unique,
            up_to_date: RefCell::new(false),
        }
    }

    pub(crate) fn add_entry(self: &Rc<Self>, name: CString, value: CString) -> Rc<udev_list_entry> {
        let entry = Rc::new(udev_list_entry {
            list: Rc::downgrade(self),
            name: name.clone(),
            value,
        });

        if self.unique {
            self.unique_entries.borrow_mut().insert(name, entry.clone());
            self.up_to_date.replace(false);
        } else {
            self.entries.borrow_mut().push(entry.clone());
        }

        entry
    }

    pub(crate) fn cleanup(&self) {
        if self.unique {
            self.up_to_date.replace(false);
            self.unique_entries.borrow_mut().clear();
        } else {
            self.entries.borrow_mut().clear();
        }
    }
}

impl Ord for udev_list_entry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}

impl PartialOrd for udev_list_entry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for udev_list_entry {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for udev_list_entry {}

impl udev_list {
    pub(crate) fn get_entry(self: &udev_list) -> *mut udev_list_entry {
        if self.unique && !*self.up_to_date.borrow() {
            self.entries.borrow_mut().clear();
            for (_, v) in self.unique_entries.borrow().iter() {
                self.entries.borrow_mut().push(v.clone());
            }
            self.entries.borrow_mut().sort();
            self.up_to_date.replace(true);
        }

        self.idx.replace(0);
        self.get_entry_next()
    }

    pub(crate) fn get_entry_next(self: &udev_list) -> *mut udev_list_entry {
        let idx = *self.idx.borrow();
        self.idx.replace(idx + 1);
        self.entries
            .borrow()
            .get(idx)
            .map(|e| Rc::as_ptr(e) as *mut udev_list_entry)
            .unwrap_or_else(std::ptr::null_mut)
    }
}

#[no_mangle]
#[append_impl]
/// udev_list_entry_get_next
pub extern "C" fn udev_list_entry_get_next(
    list_entry: *mut udev_list_entry,
) -> *mut udev_list_entry {
    if list_entry.is_null() {
        return std::ptr::null_mut();
    }

    /* Take the ownership of udev_list_entry */
    let entry = unsafe { Rc::from_raw(list_entry) };

    let list = Rc::into_raw(entry.list.clone().upgrade().unwrap()) as *mut udev_list;
    let list_mut: &mut udev_list = unsafe { std::mem::transmute(&mut *list) };
    let ret = list_mut.get_entry_next();
    let _ = unsafe { Rc::from_raw(list) };

    /* Return the ownership of udev_list_entry */
    let _ = Rc::into_raw(entry);

    ret
}

#[no_mangle]
#[append_impl]
/// udev_list_entry_get_by_name
pub extern "C" fn udev_list_entry_get_by_name(
    list_entry: *mut udev_list_entry,
    name: *const ::std::os::raw::c_char,
) -> *mut udev_list_entry {
    if list_entry.is_null() || name.is_null() {
        return std::ptr::null_mut();
    }

    let name = unsafe { CStr::from_ptr(name) }.to_owned();
    /* Take the ownership of udev_list_entry */
    let entry = unsafe { Rc::from_raw(list_entry) };
    let list = entry.list.clone().upgrade().unwrap();

    if !list.unique || !*list.up_to_date.borrow() {
        return std::ptr::null_mut();
    }

    let ret = list
        .unique_entries
        .borrow()
        .get(&name)
        .map(|v| Rc::as_ptr(v) as *mut udev_list_entry)
        .unwrap_or_else(std::ptr::null_mut);

    /* Return the ownership of udev_list_entry */
    let _ = Rc::into_raw(entry);

    ret
}

#[no_mangle]
#[append_impl]
/// udev_list_entry_get_name
pub extern "C" fn udev_list_entry_get_name(
    list_entry: *mut udev_list_entry,
) -> *const ::std::os::raw::c_char {
    if list_entry.is_null() {
        return std::ptr::null();
    }

    /* Take the ownership of udev_list_entry */
    let entry = unsafe { Rc::from_raw(list_entry) };
    let ret = entry.name.as_ptr();
    /* Return the ownership of udev_list_entry */
    let _ = Rc::into_raw(entry);
    ret
}

#[no_mangle]
#[append_impl]
/// udev_list_entry_get_value
pub extern "C" fn udev_list_entry_get_value(
    list_entry: *mut udev_list_entry,
) -> *const ::std::os::raw::c_char {
    if list_entry.is_null() {
        return std::ptr::null();
    }

    /* Take the ownership of udev_list_entry */
    let entry = unsafe { Rc::from_raw(list_entry) };
    let ret = entry.value.as_ptr();
    /* Return the ownership of udev_list_entry */
    let _ = Rc::into_raw(entry);
    ret
}
