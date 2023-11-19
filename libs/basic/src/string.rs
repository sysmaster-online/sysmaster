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

//! Common used string functions

use fnmatch_sys::fnmatch;
use libc::{c_char, c_int};
use std::ffi::CString;

/// Add "\n" to s.
/// This can be used when generating a multi-line string.
/// Use this function before you write a new line.
pub fn new_line_break(s: &mut String) {
    if !s.is_empty() {
        *s += "\n";
    }
}

/// Pattern match based on glob style pattern
/// The flags argument modifies the behavior; it is the bitwise OR of zero
/// or more of the following flags:
///
///     `FNM_NOMATCH`
///     `FNM_NOESCAPE`
///     `FNM_PATHNAME`
///     `FNM_PERIOD`
///
/// return:
///     true: if string matches.
///     false: 1.no match 2.pattern or value is error 3.fnmatch is error
pub fn pattern_match(pattern: &str, value: &str, flags: c_int) -> bool {
    let cpattern = match CString::new(pattern) {
        Ok(cpattern) => cpattern,
        Err(_err) => return false,
    };
    let cvalue = match CString::new(value) {
        Ok(cvalue) => cvalue,
        Err(_err) => return false,
    };

    unsafe {
        fnmatch(
            cpattern.as_ptr() as *const c_char,
            cvalue.as_ptr() as *const c_char,
            flags,
        ) == 0
    }
}

/// Pattern match
/// return:
///     true: 1.if string matches 2.pattern is empty
///     false: pattern is not empty and pattern_match return false
pub fn fnmatch_or_empty(pattern: &str, value: &str, flags: c_int) -> bool {
    pattern.is_empty() || pattern_match(pattern, value, flags)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fnmatch_sys;

    #[test]
    fn test_new_line_break() {
        let mut s = String::from("");
        new_line_break(&mut s);
        assert_eq!(s, "");

        let mut s = String::from("abc");
        new_line_break(&mut s);
        assert_eq!(s, "abc\n");
    }

    #[test]
    fn test_pattern_match() {
        assert!(pattern_match("hello*", "hello world", 0));
        assert!(!pattern_match("hello*", "world", 0));
        assert!(pattern_match("hello*", "hello world", unsafe {
            fnmatch_sys::FNM_NOMATCH
        }));
        assert!(!pattern_match("hello*", "world", unsafe {
            fnmatch_sys::FNM_NOMATCH
        }));

        assert!(pattern_match("hello\\*", "hello\\ world", unsafe {
            fnmatch_sys::FNM_NOESCAPE
        }));

        assert!(pattern_match("foo/*", "foo/bar.txt", unsafe {
            fnmatch_sys::FNM_PATHNAME
        }));
        assert!(!pattern_match("foo/*", "foo/subdir/bar.txt", unsafe {
            fnmatch_sys::FNM_PATHNAME
        }));

        assert!(pattern_match("*.txt", "bar.txt", unsafe {
            fnmatch_sys::FNM_PERIOD
        }));
        assert!(!pattern_match("*.txt", ".txt", unsafe {
            fnmatch_sys::FNM_PERIOD
        }));
    }

    #[test]
    fn test_fnmatch_or_empty() {
        assert!(fnmatch_or_empty("", "hello world", 0));
        assert!(fnmatch_or_empty("hello*", "hello world", 0));
        assert!(!fnmatch_or_empty("hello*", "world", 0));
    }
}
