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

//! the utils of the rlimit operation
//!

use libc;
use std::io::Error;
use std::mem;

/// indicate no limit
pub const INFINITY: u64 = libc::RLIM_INFINITY;

/// get the rlimit value
pub fn getrlimit(resource: u8) -> Result<(u64, u64), Error> {
    let mut rlimit = unsafe { mem::zeroed() };

    let ret = unsafe { libc::getrlimit(resource as _, &mut rlimit) };

    if ret == 0 {
        return Ok((rlimit.rlim_cur, rlimit.rlim_max));
    }

    Err(Error::last_os_error())
}

/// set resource limit
pub fn setrlimit(resource: u8, soft: u64, hard: u64) -> Result<(), Error> {
    let rlimit = libc::rlimit {
        rlim_cur: soft as _,
        rlim_max: hard as _,
    };

    let ret = unsafe { libc::setrlimit(resource as _, &rlimit) };
    if ret == 0 {
        Ok(())
    } else {
        Err(Error::last_os_error())
    }
}
