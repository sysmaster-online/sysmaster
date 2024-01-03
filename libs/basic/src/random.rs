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

//! Random functions

/// Get random data from getrandom() or '/dev/urandom'
pub fn random_bytes(data: &mut [u8]) {
    let mut have_grndinsecure = true;

    if data.is_empty() {
        return;
    }

    let mut l: usize = 0;
    loop {
        let flag = match have_grndinsecure {
            true => libc::GRND_INSECURE,
            false => libc::GRND_NONBLOCK,
        };
        let size = unsafe {
            libc::getrandom(
                data[l..].as_mut_ptr() as *mut libc::c_void,
                data.len() - l,
                flag,
            )
        };

        if size > 0 {
            l += size as usize;
            if l as usize == data.len() {
                /* Done reading, success. */
                return;
            }
            continue;
        } else if size == 0
            || crate::error::errno_is_not_supported(
                nix::errno::Errno::from_i32(nix::errno::errno()),
            )
            || nix::errno::errno() == libc::EAGAIN && !have_grndinsecure
        {
            /* Weird  or No syscall or Will block, but no GRND_INSECURE. Fallback to /dev/urandom. */
            break;
        } else if nix::errno::errno() == libc::EINVAL && have_grndinsecure {
            /* No GRND_INSECURE; fallback to GRND_NONBLOCK. */
            have_grndinsecure = false;
            continue;
        }

        /* Unexpected, so just give up and fallback to /dev/urandom. */
        break;
    }

    match std::fs::OpenOptions::new().read(true).open("/dev/urandom") {
        Err(err) => {
            log::error!("Failed to open /dev/urandom, err:{}", err);
        }
        Ok(mut file) => {
            let _ = crate::io::loop_read_exact(&mut file, data);
        }
    };
}
