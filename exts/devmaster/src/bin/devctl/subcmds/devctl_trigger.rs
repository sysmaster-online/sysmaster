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
use std::{cell::RefCell, rc::Rc};

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
    let mut devlist = vec![];

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
        for device in enumerator.iter() {
            devlist.push(device);
        }
    } else {
        for d in devices {
            match Device::from_path(&d) {
                Ok(dev) => {
                    devlist.push(Rc::new(RefCell::new(dev)));
                }
                Err(e) => {
                    eprintln!("Invalid device path '{}': {}", d, e);
                }
            }
        }
    }

    for d in devlist {
        if !dry_run {
            if let Err(e) = d.borrow().trigger(action) {
                if ![nix::Error::ENOENT, nix::Error::ENODEV].contains(&e.get_errno()) {
                    eprintln!(
                        "Failed to trigger '{}': {}",
                        d.borrow().get_syspath().unwrap_or_default(),
                        e
                    );
                } else {
                    println!(
                        "Ignore to trigger '{}': {}",
                        d.borrow().get_syspath().unwrap_or_default(),
                        e
                    );
                }
            }
        }
        if verbose {
            println!("{}", d.borrow().get_syspath().unwrap_or_default());
        }
    }
}
