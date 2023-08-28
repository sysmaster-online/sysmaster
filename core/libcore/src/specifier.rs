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

//! Interfaces related to the unit specifier.
//!

use basic::unit_name::unit_name_unescape;
use nix::errno::Errno;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_void};

type SpecifierCallback = fn(char, *const c_void, &str) -> Result<String, Errno>;

///
pub const LONG_LINE_MAX: usize = 1024 * 1024;
///
pub const PATH_MAX: usize = 4096;
///
pub const UNIT_NAME_MAX: usize = 256;

const POSSIBLE_SPECIFIERS: &str = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ%";

/// Specifier-related unit data.
#[derive(Debug)]
pub struct UnitSpecifierData {
    /// Content between the first '@' and the last '.'.
    pub instance: CString,
    // ...
}

impl UnitSpecifierData {
    /// Create a UnitSpecifierData object.
    pub fn new() -> Self {
        UnitSpecifierData {
            instance: CString::new("").unwrap(),
            // ...
        }
    }
}

impl Default for UnitSpecifierData {
    fn default() -> Self {
        Self::new()
    }
}

struct Specifier {
    specifier: char,
    lookup: SpecifierCallback,
    data: *const c_void,
}

fn specifier_string(_specifier: char, data: *const c_void, _root: &str) -> Result<String, Errno> {
    match unsafe { CStr::from_ptr(data as *const c_char) }.to_str() {
        Ok(data_str) => Ok(data_str.to_string()),
        _ => Err(Errno::EINVAL),
    }
}

fn specifier_instance_unescape(
    _specifier: char,
    data: *const c_void,
    _root: &str,
) -> Result<String, Errno> {
    match unsafe { CStr::from_ptr(data as *const c_char) }.to_str() {
        Ok(data_str) => unit_name_unescape(data_str),
        _ => Err(Errno::EINVAL),
    }
}

fn specifier_escape(
    text: &str,
    max_len: usize,
    table: &[Specifier],
    root: &str,
) -> Result<String, Errno> {
    let mut ret = String::new();
    let mut percent = false;

    for c in text.chars() {
        if percent {
            percent = false;

            if c == '%' {
                ret.push('%');
            } else if let Some(i) = table.iter().find(|i| i.specifier == c) {
                match (i.lookup)(i.specifier, i.data, root) {
                    Ok(w) => ret.push_str(&w),
                    Err(e) => return Err(e),
                }
            } else if POSSIBLE_SPECIFIERS.contains(c) {
                return Err(Errno::EBADSLT);
            } else {
                ret.push('%');
                ret.push(c);
            }
        } else if c == '%' {
            percent = true;
        } else {
            ret.push(c);
        }

        if ret.len() > max_len {
            return Err(Errno::ENAMETOOLONG);
        }
    }

    if percent {
        ret.push('%');
        if ret.len() > max_len {
            return Err(Errno::ENAMETOOLONG);
        }
    }

    Ok(ret)
}

/// Escape the specifier of unit in text.
/// text:                Text to be escaped.
/// max_len:             Maximum length allowed after escape.
/// unit_specifier_data: Specifier-related unit data.
pub fn unit_string_specifier_escape(
    text: &str,
    max_len: usize,
    unit_specifier_data: &UnitSpecifierData,
) -> Result<String, Errno> {
    let table = [
        Specifier {
            specifier: 'i',
            lookup: specifier_string,
            data: unit_specifier_data.instance.as_ptr() as *const c_void,
        },
        Specifier {
            specifier: 'I',
            lookup: specifier_instance_unescape,
            data: unit_specifier_data.instance.as_ptr() as *const c_void,
        },
    ];

    match specifier_escape(text, max_len, &table, "") {
        Ok(ret) => Ok(ret),
        Err(e) => {
            log::error!(
                "Failed to resolve unit specifier in '{}', ignoring: {}",
                text,
                e
            );
            Err(e)
        }
    }
}

/// Escape the specifier of unit in texts.
/// texts:               Texts to be escaped.
/// max_len:             Maximum length allowed after escape.
/// unit_specifier_data: Specifier-related unit data.
pub fn unit_strings_specifier_escape(
    texts: &[String],
    max_len: usize,
    unit_specifier_data: &UnitSpecifierData,
) -> Result<Vec<String>, Errno> {
    let mut result = Vec::new();

    for text in texts {
        match unit_string_specifier_escape(text, max_len, unit_specifier_data) {
            Ok(ret) => result.push(ret),
            Err(e) => return Err(e),
        }
    }

    Ok(result)
}
