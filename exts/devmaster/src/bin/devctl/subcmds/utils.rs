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

//! the utils of devctl

use crate::Result;
use device::Device;
use nix::unistd::{access, AccessFlags};
use std::path::PathBuf;

/// find device by path or unit name
pub fn find_device(id: &str, prefix: &str) -> Result<Device> {
    if id.is_empty() {
        return Err(nix::Error::EINVAL);
    }
    if let Ok(device) = Device::from_path(id) {
        return Ok(device);
    }

    let mut path = PathBuf::from(id);

    if !prefix.is_empty() && !id.starts_with(prefix) {
        path = match PathBuf::from(prefix.to_string() + "/" + id).canonicalize() {
            Ok(path) => path,
            Err(err) => {
                return Err(nix::errno::from_i32(err.raw_os_error().unwrap_or_default()));
            }
        };
        if let Ok(device) = Device::from_path(path.to_str().unwrap()) {
            return Ok(device);
        }
    }

    /* if a path is provided, then it cannot be a unit name. Let's return earlier. */
    if path.to_str().unwrap().contains('/') {
        return Err(nix::Error::ENODEV);
    }

    /* Check if the argument looks like a device unit name. */
    find_device_from_unit(id)
}

/// dbus and device unit is not currently implemented
fn find_device_from_unit(_unit_name: &str) -> Result<Device> {
    todo!()
}

/// check if the queue is empty
pub fn devmaster_queue_is_empty() -> Result<bool> {
    match access("/run/devmaster/queue", AccessFlags::F_OK) {
        Ok(()) => Ok(false),
        Err(err) => {
            if err == nix::Error::ENOENT {
                return Ok(true);
            }
            Err(err)
        }
    }
}
