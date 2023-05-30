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

use std::ffi::CStr;

use crate::{errno_ret, libblkid, Result};

/// A handle for working with a probed partition.
pub struct BlkidPartition(libblkid::blkid_partition);

impl BlkidPartition {
    /// Get the partition name or `None` if it can't be represented.
    pub fn get_name(&self) -> Option<String> {
        let char_ptr = unsafe { libblkid::blkid_partition_get_name(self.0) };
        if char_ptr.is_null() {
            return None;
        }

        match unsafe { CStr::from_ptr(char_ptr) }.to_str() {
            Ok(s) => Some(s.to_string()),
            Err(_) => None,
        }
    }

    /// Get the partition UUID or `None` if the partition table doesn't support it.
    pub fn get_uuid(&self) -> Option<String> {
        let char_ptr = unsafe { libblkid::blkid_partition_get_uuid(self.0) };
        if char_ptr.is_null() {
            return None;
        }

        match unsafe { CStr::from_ptr(char_ptr) }.to_str() {
            Ok(s) => Some(s.to_string()),
            Err(_) => None,
        }
    }

    /// Get the string representation of the partition type.
    pub fn get_type_string(&self) -> Option<String> {
        let char_ptr = unsafe { libblkid::blkid_partition_get_type_string(self.0) };
        match unsafe { CStr::from_ptr(char_ptr) }.to_str() {
            Ok(s) => Some(s.to_string()),
            Err(_) => None,
        }
    }

    /// Get the flags for the given partition.
    pub fn get_flags(&self) -> libc::c_ulonglong {
        unsafe { libblkid::blkid_partition_get_flags(self.0) }
    }

    /// Check whether the given partition is logical.
    pub fn is_logical(&self) -> bool {
        (unsafe { libblkid::blkid_partition_is_logical(self.0) }) != 0
    }

    /// Check whether the given partition is an extended partition.
    pub fn is_extended(&self) -> bool {
        (unsafe { libblkid::blkid_partition_is_extended(self.0) }) != 0
    }

    /// Check whether the given partition is a primary partition.
    pub fn is_primary(&self) -> bool {
        (unsafe { libblkid::blkid_partition_is_primary(self.0) }) != 0
    }
}

/// A handle for traversing a list of partitions.
pub struct BlkidPartlist(libblkid::blkid_partlist);

impl BlkidPartlist {
    pub(crate) fn new(partlist: libblkid::blkid_partlist) -> BlkidPartlist {
        BlkidPartlist(partlist)
    }

    /// Get the number of partitions in the list.
    pub fn numof_partitions(&mut self) -> Result<libc::c_int, i32> {
        let ret = errno_ret!(unsafe { libblkid::blkid_partlist_numof_partitions(self.0) });
        Ok(ret)
    }

    /// Get a partition at the given index of the list.
    pub fn get_partition(&mut self, index: libc::c_int) -> Option<BlkidPartition> {
        let part = unsafe { libblkid::blkid_partlist_get_partition(self.0, index) };
        if part.is_null() {
            None
        } else {
            Some(BlkidPartition(part))
        }
    }
}
