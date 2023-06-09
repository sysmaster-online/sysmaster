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

//!

use std::fmt::Display;

/// uuid
#[derive(PartialEq, Eq)]
pub struct Uuid(pub(super) [u8; 16]);

fn unhexchar(c: u8) -> Result<u8, ()> {
    if c.is_ascii_digit() {
        return Ok(c - b'0');
    } else if c.is_ascii_hexdigit() && c.is_ascii_lowercase() {
        return Ok(c - b'a' + 10);
    } else if c.is_ascii_hexdigit() && c.is_ascii_uppercase() {
        return Ok(c - b'A' + 10);
    }

    Err(())
}

fn hexchar(c: u8) -> Result<char, ()> {
    let table: &[u8; 16] = b"0123456789abcdef";
    if c > 15 {
        return Err(());
    }
    Ok(table[c as usize] as char)
}

impl Default for Uuid {
    fn default() -> Self {
        Uuid::new()
    }
}

impl Display for Uuid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut ret = String::new();
        for i in 0..self.0.len() {
            if vec![4, 6, 8, 10].contains(&i) {
                ret.push('-');
            }
            ret.push((hexchar(self.0[i] >> 4)).unwrap());
            ret.push((hexchar(self.0[i] & 0xF)).unwrap());
        }
        write!(f, "{ret}")
    }
}

impl Uuid {
    /// create Uuid instance
    pub fn new() -> Self {
        Uuid([0; 16])
    }

    /// structure uuid by format string
    pub fn from_string(str: &str) -> Option<Uuid> {
        let mut id128 = Uuid::new();
        let buf = str.as_bytes();
        let mut i = 0;
        let mut is_guid = false;
        let mut n = 0;
        while n < 16 {
            if i >= buf.len() {
                break;
            }
            if buf[i] == b'-' {
                if i == 8 {
                    is_guid = true;
                } else if i == 13 || i == 18 || i == 23 {
                    if !is_guid {
                        return None;
                    }
                } else {
                    return None;
                }
                i += 1;
                continue;
            }

            let a = match unhexchar(buf[i]) {
                Ok(a) => a,
                Err(_) => return None,
            };
            i += 1;

            let b = match unhexchar(buf[i]) {
                Ok(a) => a,
                Err(_) => return None,
            };
            i += 1;

            id128.0[n] = a << 4 | b;
            n += 1;
        }

        if is_guid && i != 36 {
            return None;
        }

        if !is_guid && i != 32 {
            return None;
        }

        Some(id128)
    }

    /// uuid is null
    pub fn is_null(&self) -> bool {
        let mut sum = 0;
        for i in self.0 {
            sum += i;
        }
        sum == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uuid() {
        let str_uuid1 = "e57446f87c3f4f978a7eca30ff7197d3";
        let uuid1 = Uuid::from_string(str_uuid1).unwrap();
        assert_eq!(
            "e57446f8-7c3f-4f97-8a7e-ca30ff7197d3".to_string(),
            uuid1.to_string()
        );

        let str_uuid2 = "E57446f8-7c3f-4f97-8a7e-ca30ff7197d3";
        let uuid2 = Uuid::from_string(str_uuid2).unwrap();
        assert_eq!(
            "e57446f8-7c3f-4f97-8a7e-ca30ff7197d3".to_string(),
            uuid2.to_string()
        );
    }
}
