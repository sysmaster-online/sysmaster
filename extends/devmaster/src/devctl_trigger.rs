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

//! subcommand for devctl trigger
//!
use libdevice::{device::Device, device_action::DeviceAction};

/// subcommand for trigger a fake device action, then the kernel will report an uevent
pub fn subcommand_trigger(devices: Vec<String>, action: Option<String>) {
    if devices.is_empty() {
        todo!("Currently do not support triggering all devices")
    }

    let action = match action {
        Some(a) => a.parse::<DeviceAction>().unwrap(),
        None => DeviceAction::Change,
    };

    for d in devices {
        let mut device = Device::from_path(d).unwrap();
        device.trigger(action).unwrap();
    }
}
