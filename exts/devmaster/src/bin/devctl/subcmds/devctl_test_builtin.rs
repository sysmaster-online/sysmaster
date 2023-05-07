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

//! subcommand for testing builtin command
//!

use device::Device;
use libdevmaster::builtin::{BuiltinCommand, BuiltinManager, Netlink};
use std::{
    cell::RefCell,
    sync::{Arc, Mutex},
};

/// test builtin command on processing a device
/// Commands:
///     blkid           probe file system and partitions of a block device
///     btrfs           btrfs volume management
///     hwdb            queuery properties in hardware database
///     input_id        get unique properties of input device
///     keyboard        scan code of keyboard device for key mapping
///     kmod            load kernel modules
///     net_id          get unique properties of network device
///     net_setup_link  configure network link
///     path_id         generate persistent device path
///     usb_id          get unique properties of usb device
pub fn subcommand_test_builtin(action: Option<String>, builtin_cmd: String, device: String) {
    println!("Builtin command: '{}'", builtin_cmd);
    println!("Device: '{}'", device);
    println!(
        "Action: '{}'",
        action.clone().unwrap_or_else(|| "change".to_string())
    );

    let mgr = BuiltinManager::new();
    mgr.init();

    let d = Arc::new(Mutex::new(match Device::from_path(device.clone()) {
        Ok(ret) => ret,
        Err(_) => Device::from_path(format!("/sys{}", device)).expect("invalid device path."),
    }));

    if let Err(e) = d.lock().unwrap().add_property(
        "ACTION".to_string(),
        action.unwrap_or_else(|| "change".to_string()),
    ) {
        eprintln!("{:?}", e);
    }

    let mut rtnl = RefCell::<Option<Netlink>>::from(None);

    let argv = builtin_cmd
        .split_ascii_whitespace()
        .map(|s| s.to_string())
        .collect::<Vec<String>>();

    if let Err(e) = mgr.run(
        d,
        &mut rtnl,
        builtin_cmd
            .parse::<BuiltinCommand>()
            .expect("invalid builtin command."),
        argv.len() as i32,
        argv,
        true,
    ) {
        eprintln!("{:?}", e);
    }
}
