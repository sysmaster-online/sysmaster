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

//! the utils of the globbing pathnames
//!

use crate::error::*;
use crate::Error;
use libc::{glob, glob_t, globfree, GLOB_NOSORT};
use nix::errno::Errno;
use std::ffi::CString;

/// Find the first path name that matches the pattern
pub fn glob_first(path: &str) -> Result<String> {
    if path.is_empty() {
        return Err(Error::Nix {
            source: Errno::EINVAL,
        });
    }

    let pattern = CString::new(path).unwrap();
    let mut pglob: glob_t = unsafe { std::mem::zeroed() };
    let mut first = String::new();

    /* use GLOB_NOSORT to speed up. */
    let ret = unsafe { glob(pattern.as_ptr(), GLOB_NOSORT, None, &mut pglob) };
    if 0 != ret {
        match ret {
            libc::GLOB_NOSPACE => {
                return Err(Error::Other {
                    msg: (String::from("running out of memory")),
                });
            }
            libc::GLOB_ABORTED => {
                return Err(Error::Other {
                    msg: (String::from("read error")),
                });
            }
            libc::GLOB_NOMATCH => {
                return Err(Error::Other {
                    msg: (String::from("no found matches")),
                });
            }
            _ => {
                return Err(Error::Other {
                    msg: (String::from("Unknown error")),
                });
            }
        }
    }

    if pglob.gl_pathc > 0 {
        let ptr = unsafe { std::ffi::CStr::from_ptr(*pglob.gl_pathv.offset(0)) };
        first = ptr.to_str().unwrap().to_string();
    }

    unsafe { globfree(&mut pglob) };
    Ok(first)
}
