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
//

use std::{ffi::CString, os::unix::io::RawFd, ptr};

use crate::{
    errno_ret, libblkid, partition::BlkidPartlist, BlkidFltr, BlkidSublksFlags, BlkidUsageFlags,
};

#[cfg(blkid = "libblkid_2_37")]
use crate::BlkidProbPartsFlags;

type Result<T> = std::result::Result<T, i32>;

/// A structure for probing block devices.
pub struct BlkidProbe(pub(super) libblkid::blkid_probe);

impl BlkidProbe {
    /// Allocate and create a new libblkid probe.
    pub fn new() -> Option<Self> {
        let p = unsafe { libblkid::blkid_new_probe() };
        if p.is_null() {
            None
        } else {
            Some(BlkidProbe(p))
        }
    }

    /// Assign the device to probe control struct.
    pub fn set_device(
        &mut self,
        fd: RawFd,
        offset: libblkid::blkid_loff_t,
        size: libblkid::blkid_loff_t,
    ) -> Result<()> {
        errno_ret!(unsafe { libblkid::blkid_probe_set_device(self.0, fd, offset, size) });
        Ok(())
    }

    /// Check the device is an entire disk
    pub fn is_wholedisk(&self) -> bool {
        (unsafe { libblkid::blkid_probe_is_wholedisk(self.0) }) > 0
    }

    /// Get the size of of a device.
    pub fn get_size(&self) -> libblkid::blkid_loff_t {
        unsafe { libblkid::blkid_probe_get_size(self.0) }
    }

    /// Get a file descriptor for assigned device.
    pub fn get_fd(&self) -> Result<RawFd> {
        let fd = errno_ret!(unsafe { libblkid::blkid_probe_get_fd(self.0) });
        Ok(fd)
    }

    /// Enable superblock probing.
    pub fn enable_superblocks(&mut self, enable: bool) -> Result<()> {
        errno_ret!(unsafe { libblkid::blkid_probe_enable_superblocks(self.0, enable.into()) });
        Ok(())
    }

    /// Set the superblock probing flags.
    pub fn set_superblock_flags(&mut self, flags: BlkidSublksFlags) -> Result<()> {
        errno_ret!(unsafe { libblkid::blkid_probe_set_superblocks_flags(self.0, flags.bits()) });
        Ok(())
    }

    /// Filter devices based on the usages specified in the `usage` parameter.
    pub fn filter_superblock_usage(
        &mut self,
        flag: BlkidFltr,
        usage: BlkidUsageFlags,
    ) -> Result<()> {
        errno_ret!(unsafe {
            libblkid::blkid_probe_filter_superblocks_usage(self.0, flag.bits(), usage.bits())
        });
        Ok(())
    }

    /// Enable partition probing.
    pub fn enable_partitions(&mut self, enable: bool) -> Result<()> {
        errno_ret!(unsafe { libblkid::blkid_probe_enable_partitions(self.0, enable.into()) });
        Ok(())
    }

    /// Probes all enabled chains and checks for ambiguous results.
    pub fn do_safeprobe(&mut self) -> Result<()> {
        errno_ret!(unsafe { libblkid::blkid_do_safeprobe(self.0) });
        Ok(())
    }

    /// Same as `do_safeprobe` but does not check for collisions.
    pub fn do_fullprobe(&mut self) -> Result<()> {
        errno_ret!(unsafe { libblkid::blkid_do_fullprobe(self.0) });
        Ok(())
    }

    /// Number of values in probe
    pub fn numof_values(&self) -> Result<usize> {
        let size = errno_ret!(unsafe { libblkid::blkid_probe_numof_values(self.0) });
        Ok(size as usize)
    }

    /// Get the tag and value of an entry by the index in the range
    /// `0..(self.numof_values())`.
    pub fn get_value(&self, num: libc::c_uint) -> Result<(String, String)> {
        let num_values = self.numof_values()?;
        if num as usize >= num_values {
            return Err(-1);
        }

        let mut name: *const libc::c_char = ptr::null();
        let mut data: *const libc::c_char = ptr::null();
        let mut size: usize = 0;
        errno_ret!(unsafe {
            libblkid::blkid_probe_get_value(
                self.0,
                num as libc::c_int,
                &mut name as *mut _,
                &mut data as *mut _,
                &mut size as *mut _,
            )
        });
        let name = unsafe { std::ffi::CStr::from_ptr(name) }
            .to_str()
            .unwrap()
            .to_string();
        let data = std::ffi::CStr::from_bytes_with_nul(unsafe {
            std::slice::from_raw_parts(data as *const u8, size)
        })
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
        Ok((name, data))
    }

    /// Get the value for a tag with the given name.
    pub fn lookup_value(&self, name: &str) -> Result<String> {
        let name_cstring = CString::new(name).unwrap();

        let mut data: *const libc::c_char = ptr::null();
        let mut size: usize = 0;
        errno_ret!(unsafe {
            libblkid::blkid_probe_lookup_value(
                self.0,
                name_cstring.as_ptr(),
                &mut data as *mut _,
                &mut size as *mut _,
            )
        });
        let data = std::ffi::CStr::from_bytes_with_nul(unsafe {
            std::slice::from_raw_parts(data as *const u8, size)
        })
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
        Ok(data)
    }

    /// Get list of probed partitions.
    pub fn get_partitions(&mut self) -> Option<BlkidPartlist> {
        let partlist = unsafe { libblkid::blkid_probe_get_partitions(self.0) };
        if partlist.is_null() {
            None
        } else {
            Some(BlkidPartlist::new(partlist))
        }
    }

    /// Set hint
    #[cfg(blkid = "libblkid_2_37")]
    pub fn set_hint(&mut self, name: &str, value: u64) -> Result<()> {
        let name_cstring = CString::new(name).unwrap();
        errno_ret!(unsafe { libblkid::blkid_probe_set_hint(self.0, name_cstring.as_ptr(), value) });
        Ok(())
    }

    ///Set partitions flags
    #[cfg(blkid = "libblkid_2_37")]
    pub fn set_partitions_flags(&mut self, flags: BlkidProbPartsFlags) -> Result<()> {
        errno_ret!(unsafe { libblkid::blkid_probe_set_partitions_flags(self.0, flags.bits()) });
        Ok(())
    }
}

impl Drop for BlkidProbe {
    fn drop(&mut self) {
        unsafe { libblkid::blkid_free_probe(self.0) }
    }
}
