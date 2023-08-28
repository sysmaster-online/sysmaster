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

//! Interfaces related to the unit name.
//!

use nix::errno::Errno;
use std::ffi::CString;

fn unhexchar(c: char) -> Result<i32, Errno> {
    match c {
        '0'..='9' => Ok(c as i32 - '0' as i32),
        'a'..='f' => Ok(c as i32 - 'a' as i32 + 10),
        'A'..='F' => Ok(c as i32 - 'A' as i32 + 10),
        _ => Err(Errno::EINVAL),
    }
}

/// Restore the unit name which is escaped
pub fn unit_name_unescape(s: &str) -> Result<String, Errno> {
    let mut vec = Vec::new();

    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '-' {
            vec.push(b'/');
        } else if c == '\\' {
            if chars.next().unwrap() != 'x' {
                return Err(Errno::EINVAL);
            }

            if let Ok(a) = unhexchar(chars.next().unwrap()) {
                if let Ok(b) = unhexchar(chars.next().unwrap()) {
                    vec.push((((a as u8) << 4) as u8) | (b as u8));
                    continue;
                }
            }

            return Err(Errno::EINVAL);
        } else {
            vec.push(c as u8);
        }
    }

    match String::from_utf8(vec) {
        Ok(ret) => Ok(ret),
        _ => Err(Errno::EINVAL),
    }
}

/// Get the content between the first '@' and the last '.' from unit name.
pub fn unit_name_to_instance(unit_name: &str) -> CString {
    let mut p = match unit_name.find('@') {
        Some(pos) => pos,
        None => return CString::new("").unwrap(),
    };

    p += 1;

    let mut d = match unit_name[p..].rfind('.') {
        Some(pos) => pos,
        None => return CString::new("").unwrap(),
    };

    d += p;

    CString::new(&unit_name[p..d]).unwrap()
}
