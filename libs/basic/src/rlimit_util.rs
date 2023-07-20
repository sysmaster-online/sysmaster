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
use nix::sys::resource::{self, Resource};
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

/// Reset rlimit, ensure safe
pub fn rlimit_nofile_safe() {
    let mut limit = match resource::getrlimit(Resource::RLIMIT_NOFILE) {
        Ok(limit) => limit,
        Err(e) => {
            log::warn!("Failed to query RLIMIT_NOFILE: {}", e);
            return;
        }
    };

    log::info!("limit: {}, {}", limit.0, limit.1);
    if limit.0 <= nix::sys::select::FD_SETSIZE as u64 {
        return;
    }

    limit.0 = nix::sys::select::FD_SETSIZE as u64;

    if let Err(e) = resource::setrlimit(Resource::RLIMIT_NOFILE, limit.0, limit.1) {
        log::warn!("Failed to set RLIMIT_NOFILE: {}", e);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_rlimit() {
        let soft = 1100;
        let hard = 2048;
        setrlimit(libc::RLIMIT_NOFILE as u8, soft, hard).unwrap();
        rlimit_nofile_safe();
        let limit = getrlimit(libc::RLIMIT_NOFILE as u8).unwrap();
        assert_eq!(limit.0, nix::sys::select::FD_SETSIZE as u64);
        assert_eq!(limit.1, hard);

        let soft = 1022;
        setrlimit(libc::RLIMIT_NOFILE as u8, soft, hard).unwrap();
        rlimit_nofile_safe();
        let limit = getrlimit(libc::RLIMIT_NOFILE as u8).unwrap();
        assert_eq!(limit.0, soft);
        assert_eq!(limit.1, hard);

        let soft = 1024;
        setrlimit(libc::RLIMIT_NOFILE as u8, soft, hard).unwrap();
        rlimit_nofile_safe();
        let limit = getrlimit(libc::RLIMIT_NOFILE as u8).unwrap();
        assert_eq!(limit.0, soft);
        assert_eq!(limit.1, hard);
    }
}
