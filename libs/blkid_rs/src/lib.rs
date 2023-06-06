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

//! innner lib of libblkid
mod libblkid;
mod partition;
mod probe;

use bitflags::bitflags;
use libc::{c_char, c_int};
use std::ffi::CString;
use std::result::Result;

pub use crate::{partition::BlkidPartition, partition::BlkidPartlist, probe::BlkidProbe};

/// the macro for wrapper libblkid return value
#[macro_export]
macro_rules! errno_ret {
    ($ret_expr:expr) => {
        match $ret_expr {
            i if i < 0 => return Err(i),
            i => i,
        }
    };
}

bitflags! {
    /// BlkidSublksFlags
    pub struct BlkidSublksFlags: i32 {
        /// Read label from superblock
        const LABEL = libblkid::BLKID_SUBLKS_LABEL as i32;
        /// Read label from superblock and define `LABEL_RAW` value
        const LABELRAW = libblkid::BLKID_SUBLKS_LABELRAW as i32;
        /// Read UUID from superblock
        const UUID = libblkid::BLKID_SUBLKS_UUID as i32;
        /// Read UUID from superblock and define `UUID_RAW` value
        const UUIDRAW = libblkid::BLKID_SUBLKS_UUID as i32;
        /// Read type from superblock and define `TYPE` value
        const TYPE = libblkid::BLKID_SUBLKS_TYPE as i32;
        /// Read compatible filesystem type from superblock
        const SECTYPE = libblkid::BLKID_SUBLKS_SECTYPE as i32;
        /// Read usage from superblock and define `USAGE` value
        const USAGE = libblkid::BLKID_SUBLKS_USAGE as i32;
        /// Read filesystem version from superblock
        const VERSION = libblkid::BLKID_SUBLKS_VERSION as i32;
        /// Read superblock magic number and define `SBMAGIC` and `SBMAGIC_OFFSET`
        const MAGIC = libblkid::BLKID_SUBLKS_MAGIC as i32;
        /// Allow a bad checksum
        const BADCSUM = libblkid::BLKID_SUBLKS_BADCSUM as i32;
        /// Default flags
        const DEFAULT = libblkid::BLKID_SUBLKS_DEFAULT as i32;
    }

    /// BlkidFltr
    pub struct BlkidFltr: i32 {
        /// Probe for all names that are not in the list that was provided.
        const NOTIN = libblkid::BLKID_FLTR_NOTIN as i32;
        /// Probe for all names that are in the list that was provided.
        const ONLYIN = libblkid::BLKID_FLTR_ONLYIN as i32;
    }

    /// BlkidUsageFlags
    pub struct BlkidUsageFlags: i32 {
        /// filesystemd
        const FILESYSTEM = libblkid::BLKID_USAGE_FILESYSTEM as i32;
        /// raid
        const RAID = libblkid::BLKID_USAGE_RAID as i32;
        /// crypto
        const CRYPTO = libblkid::BLKID_USAGE_CRYPTO as i32;
        /// other
        const OTHER = libblkid::BLKID_USAGE_OTHER as i32;
    }
}

#[cfg(blkid = "libblkid_2_37")]
bitflags! {
        /// BlkidProbPartsFlags
        pub struct BlkidProbPartsFlags: i32 {
            /// force gpt
            const FORCE_GPT = libblkid::BLKID_PARTS_FORCE_GPT as i32;
            /// get entry details info
            const ENTRY_DETAILS= libblkid::BLKID_PARTS_ENTRY_DETAILS as i32;
            /// magic
            const MAGIC = libblkid::BLKID_PARTS_MAGIC as i32;
        }
}

// Shared code for encoding methods
fn string_shared<F>(string: &str, closure: F) -> Result<String, String>
where
    F: Fn(&CString, &mut Vec<u8>) -> c_int,
{
    // Per the documentation, the maximum buffer is 4 times
    // the length of the string.
    let mut buffer = vec![0u8; string.len() * 4];

    let cstring = CString::new(string).unwrap();
    if closure(&cstring, &mut buffer) != 0 {
        return Err("The requested conversion was unsuccessful".to_string());
    }

    let first_null = buffer
        .iter()
        .position(|u| *u == 0)
        .ok_or_else(|| "No null found in C string".to_string())?;
    buffer.truncate(first_null);
    let buffer_cstring = CString::new(buffer).unwrap();
    buffer_cstring.into_string().map_err(|e| format!("{}", e))
}

/// Encode potentially unsafe characters
pub fn encode_string(string: &str) -> Result<String, String> {
    string_shared(string, |cstring, buffer| unsafe {
        libblkid::blkid_encode_string(
            cstring.as_ptr(),
            buffer.as_mut_ptr() as *mut c_char,
            buffer.len(),
        )
    })
}

/// Generate a safe string that allows ascii, hex-escaping, and utf8. Whitespaces become `_`.
pub fn safe_string(string: &str) -> Result<String, String> {
    string_shared(string, |cstring, buffer| unsafe {
        libblkid::blkid_safe_string(
            cstring.as_ptr(),
            buffer.as_mut_ptr() as *mut c_char,
            buffer.len(),
        )
    })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_encode_string() {
        let encoded_string = encode_string("\\test string").unwrap();
        assert_eq!(encoded_string, "\\x5ctest\\x20string");
    }

    #[test]
    fn test_safe_string() {
        let safe_string = safe_string("test string").unwrap();
        assert_eq!(safe_string, "test_string");
    }
}
