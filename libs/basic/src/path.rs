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

//! utilities for parse path string
#![allow(clippy::not_unsafe_ptr_arg_deref)]
use std::ffi::CStr;

/// Check whether a C string is valid and normalized
pub fn path_is_normalized(p: *const ::std::os::raw::c_char) -> bool {
    if p.is_null() {
        return false;
    }

    let p = match unsafe { CStr::from_ptr(p) }.to_str() {
        Ok(p) => p,
        Err(_) => {
            return false;
        }
    };

    if p == "."
        || p.starts_with("./")
        || p.ends_with("/.")
        || p.contains("/./")
        || p.contains("/../")
        || p.contains("//")
    {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_is_normalized() {
        assert!(path_is_normalized("a\0".as_ptr() as *const _));
        assert!(path_is_normalized("a/b\0".as_ptr() as *const _));
        assert!(path_is_normalized("/a\0".as_ptr() as *const _));
        assert!(!path_is_normalized("./a\0".as_ptr() as *const _));
        assert!(!path_is_normalized("a/.\0".as_ptr() as *const _));
        assert!(!path_is_normalized("a/./a\0".as_ptr() as *const _));
        assert!(!path_is_normalized("a//a\0".as_ptr() as *const _));
        assert!(!path_is_normalized("a/../a\0".as_ptr() as *const _));
    }
}
