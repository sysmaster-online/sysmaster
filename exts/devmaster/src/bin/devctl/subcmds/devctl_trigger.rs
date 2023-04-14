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
use device::{
    device_enumerator::{DeviceEnumerationType, DeviceEnumerator},
    Device, DeviceAction,
};

/// subcommand for trigger a fake device action, then the kernel will report an uevent
pub fn subcommand_trigger(
    devices: Vec<String>,
    r#type: Option<String>,
    verbose: bool,
    action: Option<String>,
    dry_run: bool,
) {
    let action = match action {
        Some(a) => a.parse::<DeviceAction>().unwrap(),
        None => DeviceAction::Change,
    };

    // if no device is declared, enumerate all devices or subsystems and drivers under /sys/
    if devices.is_empty() {
        let etype = match r#type {
            Some(t) => {
                if t == "devices" {
                    DeviceEnumerationType::Devices
                } else if t == "subsystems" {
                    DeviceEnumerationType::Subsystems
                } else {
                    log::error!("invalid events type{}", t);
                    return;
                }
            }
            None => DeviceEnumerationType::Devices,
        };

        let mut enumerator = DeviceEnumerator::new();
        enumerator.set_enumerator_type(etype);
        for device in enumerator.iter_mut() {
            if !dry_run {
                device.lock().unwrap().trigger(action).unwrap();
            }
            if verbose {
                println!(
                    "{}",
                    device.lock().unwrap().get_syspath().unwrap_or_default()
                );
            }
        }
        return;
    }

    for d in devices {
        let mut device = Device::from_path(d).unwrap();
        if !dry_run {
            device.trigger(action).unwrap();
        }
        if verbose {
            println!("{}", device.get_syspath().unwrap_or_default());
        }
    }
}
