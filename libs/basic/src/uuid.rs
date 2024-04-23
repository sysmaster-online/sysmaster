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
use bitflags::bitflags;
use std::fmt::Display;

bitflags! {
    /// GPT attribute
    pub struct GptAttribute: u64 {
        /// growfs
        const GPT_FLAG_GROWFS = 1 << 59;
        /// read only
        const GPT_FLAG_READ_ONLY = 1 << 60;
        /// no auto mount
        const GPT_FLAG_NO_AUTO = 1 << 63;
    }
}

/// GPT_ESP UUID type
pub const GPT_ESP: Uuid = Uuid([
    0xc1, 0x2a, 0x73, 0x28, 0xf8, 0x1f, 0x11, 0xd2, 0xba, 0x4b, 0x00, 0xa0, 0xc9, 0x3e, 0xc9, 0x3b,
]);
/// GPT_XBOOTLDR UUID type
pub const GPT_XBOOTLDR: Uuid = Uuid([
    0xbc, 0x13, 0xc2, 0xff, 0x59, 0xe6, 0x42, 0x62, 0xa3, 0x52, 0xb2, 0x75, 0xfd, 0x6f, 0x71, 0x72,
]);
#[cfg(target_arch = "aarch64")]
/// GPT_ROOT_NATIVE UUID type
pub const GPT_ROOT_NATIVE: Uuid = Uuid([
    0xb9, 0x21, 0xb0, 0x45, 0x1d, 0xf0, 0x41, 0xc3, 0xaf, 0x44, 0x4c, 0x6f, 0x28, 0x0d, 0x3f, 0xae,
]);
#[cfg(target_arch = "x86_64")]
/// GPT_ROOT_NATIVE UUID type
pub const GPT_ROOT_NATIVE: Uuid = Uuid([
    0x69, 0xda, 0xd7, 0x10, 0x2c, 0xe4, 0x4e, 0x3c, 0xb1, 0x6c, 0x21, 0xa1, 0xd4, 0x9a, 0xbe, 0xd3,
]);
#[cfg(target_arch = "riscv64")]
/// GPT_ROOT_NATIVE UUID type
pub const GPT_ROOT_NATIVE: Uuid = Uuid([
    0x72, 0xec, 0x70, 0xa6, 0xcf, 0x74, 0x40, 0xe6, 0xbd, 0x49, 0x4b, 0xda, 0x08, 0xe8, 0xf2, 0x24,
]);

/// uuid
#[derive(PartialEq, Eq, Debug, Clone, Hash)]
pub struct Uuid(pub(super) [u8; 16]);

fn unhexchar(c: u8) -> Result<u8, ()> {
    // If the character is a digit, subtract the ASCII value of '0' to get the decimal value
    if c.is_ascii_digit() {
        return Ok(c - b'0');
    }
    // If the character is a lowercase hex digit, subtract the ASCII value of 'a' and add 10 to get the decimal value
    else if c.is_ascii_hexdigit() && c.is_ascii_lowercase() {
        return Ok(c - b'a' + 10);
    }
    // If the character is an uppercase hex digit, subtract the ASCII value of 'A' and add 10 to get the decimal value
    else if c.is_ascii_hexdigit() && c.is_ascii_uppercase() {
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
            // Convert the byte to hexadecimal characters, push empty string when ti is not hex
            ret.push((hexchar(self.0[i] >> 4)).unwrap());
            ret.push((hexchar(self.0[i] & 0xF)).unwrap());
        }
        write!(f, "{}", ret)
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

        if i != str.len() {
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

/// get random uuid
pub fn randomize() -> Result<Uuid, nix::Error> {
    let mut id = Uuid::new();

    crate::random::random_bytes(&mut id.0);

    /* Turn this into a valid v4 UUID, to be nice. Note that we
     * only guarantee this for newly generated UUIDs, not for
     * pre-existing ones. */

    Ok(id128_make_v4_uuid(id))
}

fn id128_make_v4_uuid(uuid: Uuid) -> Uuid {
    /* Stolen from generate_random_uuid() of drivers/char/random.c
     * in the kernel sources */

    /* Set UUID version to 4 --- truly random generation */
    let mut id = uuid;
    id.0[6] = (id.0[6] & 0x0F) | 0x40;

    /* Set the UUID variant to DCE */
    id.0[8] = (id.0[8] & 0x3F) | 0x80;

    id
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

        assert_eq!(
            Uuid::from_string("01020304-0506-0708-090a-0b0c0d0e0f101"),
            None
        );
        assert_eq!(
            Uuid::from_string("01020304-0506-0708-090a-0b0c0d0e0f10-"),
            None
        );
        assert_eq!(
            Uuid::from_string("01020304-0506-0708-090a0b0c0d0e0f10"),
            None
        );
        assert_eq!(
            Uuid::from_string("010203040506-0708-090a-0b0c0d0e0f10"),
            None
        );

        assert!(Uuid::default().is_null());
    }
}
