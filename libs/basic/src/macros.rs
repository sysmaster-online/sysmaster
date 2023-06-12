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

//! macros

/// syscall
#[macro_export]
macro_rules! syscall {
    ($fn: ident ( $($arg: expr),* $(,)* ) ) => {{
        let res = unsafe { libc::$fn($($arg, )*) };
        if res < 0 {
            basic::Result::Err(basic::Error::Syscall { syscall: stringify!($fn), errno: unsafe { *libc::__errno_location() }, ret: res })
        } else {
            basic::Result::Ok(res)
        }
    }};
}

/// IN_SET
#[macro_export]
macro_rules! IN_SET {
    ($ov:expr, $($nv:expr),+) => {
        {
            let mut found = false;
            $(
                if $ov == $nv {
                    found = true;
                }
            )+

            found
        }
    };
}

/// generate /proc/self/fd/{fd}
#[macro_export]
macro_rules! format_proc_fd_path {
    ($f:expr) => {
        format!("/proc/self/fd/{}", $f)
    };
}
