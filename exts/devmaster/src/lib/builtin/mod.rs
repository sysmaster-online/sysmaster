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

//! builtin commands
//!

use crate::{
    error::{Error, Result},
    framework::devmaster::Cache,
    rules::exec_unit::ExecuteUnit,
};
use device::Device;
use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::{self, Display},
    rc::Rc,
    str::FromStr,
    sync::{Arc, RwLock},
};

pub mod blkid;
pub mod btrfs;
pub mod example;
pub mod hwdb;
pub mod input_id;
pub mod keyboard;
pub mod kmod;
pub mod net_id;
pub mod net_setup_link;
pub mod path_id;
pub mod uaccess;
pub mod usb_id;

/// temporary struct definition
pub struct Netlink;

/// trait for implementing builtin commands
pub trait Builtin {
    /// builtin command
    fn cmd(
        &self,
        exec_unit: &ExecuteUnit,
        argc: i32,
        argv: Vec<String>,
        test: bool,
    ) -> Result<bool>;

    /// builtin init function
    fn init(&self);

    /// builtin exit function
    fn exit(&self);

    /// check whether builtin command should reload
    fn should_reload(&self) -> bool;

    /// the help of builtin command
    fn help(&self) -> String;

    /// whether the builtin command can only run once
    fn run_once(&self) -> bool {
        false
    }

    /// add property into device
    fn add_property(
        &self,
        device: Rc<RefCell<Device>>,
        test: bool,
        key: &str,
        value: &str,
    ) -> Result<(), Error> {
        device
            .borrow()
            .add_property(key, value)
            .map_err(|e| Error::BuiltinCommandError {
                msg: format!("Failed to add property '{}'='{}': ({})", key, value, e),
            })?;

        if test {
            println!("{}={}", key, value);
        }

        Ok(())
    }
}

/// enumerator of builtin commands
#[derive(Eq, PartialEq, Hash, Debug, Copy, Clone)]
#[allow(missing_docs)]
pub enum BuiltinCommand {
    Blkid = 0,
    Btrfs = 1,
    Hwdb = 2,
    InputId = 3,
    Keyboard = 4,
    Kmod = 5,
    NetId = 6,
    NetSetupLink = 7,
    PathId = 8,
    Uaccess = 9,
    UsbId = 10,
    Example = 11,
    Max = 12,
}

impl FromStr for BuiltinCommand {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let tokens: Vec<&str> = s.split(' ').collect();
        if tokens.is_empty() {
            return Err(Error::BuiltinCommandError {
                msg: "invalid builtin command".to_string(),
            });
        }

        match tokens[0] {
            "blkid" => Ok(BuiltinCommand::Blkid),
            "btrfs" => Ok(BuiltinCommand::Btrfs),
            "hwdb" => Ok(BuiltinCommand::Hwdb),
            "input_id" => Ok(BuiltinCommand::InputId),
            "keyboard" => Ok(BuiltinCommand::Keyboard),
            "kmod" => Ok(BuiltinCommand::Kmod),
            "net_id" => Ok(BuiltinCommand::NetId),
            "net_setup_link" => Ok(BuiltinCommand::NetSetupLink),
            "path_id" => Ok(BuiltinCommand::PathId),
            "uaccess" => Ok(BuiltinCommand::Uaccess),
            "usb_id" => Ok(BuiltinCommand::UsbId),
            "example" => Ok(BuiltinCommand::Example),
            _ => Err(Error::BuiltinCommandError {
                msg: "invalid builtin command".to_string(),
            }),
        }
    }
}

impl Display for BuiltinCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            BuiltinCommand::Blkid => "blkid",
            BuiltinCommand::Btrfs => "btrfs",
            BuiltinCommand::Hwdb => "hwdb",
            BuiltinCommand::InputId => "input_id",
            BuiltinCommand::Keyboard => "keyboard",
            BuiltinCommand::Kmod => "kmod",
            BuiltinCommand::NetId => "net_id",
            BuiltinCommand::NetSetupLink => "net_setup_link",
            BuiltinCommand::PathId => "path_id",
            BuiltinCommand::Uaccess => "uaccess",
            BuiltinCommand::UsbId => "usb_id",
            BuiltinCommand::Example => "example",
            _ => "invalid",
        };
        write!(f, "{}", s)
    }
}

/// manage builtin commands
pub struct BuiltinManager {
    builtins: HashMap<BuiltinCommand, Box<dyn Builtin>>,
}

impl BuiltinManager {
    /// create builtin manager
    pub fn new(cache: Arc<RwLock<Cache>>) -> Self {
        let mut builtins = HashMap::<BuiltinCommand, Box<dyn Builtin>>::with_capacity(
            BuiltinCommand::Max as usize,
        );

        builtins.insert(BuiltinCommand::Blkid, Box::new(blkid::Blkid {}));
        builtins.insert(BuiltinCommand::Btrfs, Box::new(btrfs::Btrfs {}));
        builtins.insert(BuiltinCommand::Hwdb, Box::new(hwdb::Hwdb {}));
        builtins.insert(BuiltinCommand::InputId, Box::new(input_id::InputId {}));
        builtins.insert(BuiltinCommand::Keyboard, Box::new(keyboard::Keyboard {}));
        builtins.insert(BuiltinCommand::Kmod, Box::new(kmod::Kmod::new()));
        builtins.insert(BuiltinCommand::NetId, Box::new(net_id::NetId {}));
        builtins.insert(
            BuiltinCommand::NetSetupLink,
            Box::new(net_setup_link::NetSetupLink { cache }),
        );
        builtins.insert(BuiltinCommand::PathId, Box::new(path_id::PathId {}));
        builtins.insert(BuiltinCommand::Uaccess, Box::new(uaccess::Uaccess {}));
        builtins.insert(BuiltinCommand::UsbId, Box::new(usb_id::UsbId {}));
        builtins.insert(BuiltinCommand::Example, Box::new(example::Example {}));

        BuiltinManager { builtins }
    }

    /// initialize all builtin commands
    #[allow(dead_code)]
    pub fn init(&self) {
        for (_, v) in self.builtins.iter() {
            v.init();
        }
    }

    /// execute exit method for each builtin command
    #[allow(dead_code)]
    pub fn exit(&self) {
        for (_, v) in self.builtins.iter() {
            v.exit();
        }
    }

    /// check whether builtin commands should reload
    #[allow(dead_code)]
    pub fn should_reload(&self) -> bool {
        for (_, v) in self.builtins.iter() {
            if v.should_reload() {
                return true;
            }
        }

        false
    }

    /// list all builtin commands
    #[allow(dead_code)]
    pub fn list(&self) {
        for (k, v) in self.builtins.iter() {
            eprintln!("    {:<14}  {}", k, v.help())
        }
    }

    /// check whether the builtin command run once
    #[allow(dead_code)]
    pub fn run_once(&self, cmd: BuiltinCommand) -> bool {
        match self.builtins.get(&cmd) {
            Some(builtin) => builtin.run_once(),
            None => false,
        }
    }

    /// run builtin command
    #[allow(dead_code)]
    pub fn run(
        &self,
        exec_unit: &ExecuteUnit,
        cmd: BuiltinCommand,
        argc: i32,
        argv: Vec<String>,
        test: bool,
    ) -> Result<bool> {
        self.builtins
            .get(&cmd)
            .unwrap()
            .cmd(exec_unit, argc, argv, test)
    }
}

#[cfg(test)]
mod tests {
    use super::BuiltinManager;
    use crate::{framework::devmaster::Cache, rules::exec_unit::ExecuteUnit};
    use device::device_enumerator::DeviceEnumerator;
    use std::sync::{Arc, RwLock};

    #[test]
    fn test_builtin_manager() {
        let cache = Arc::new(RwLock::new(Cache::new(vec![], vec![])));
        let mgr = BuiltinManager::new(cache);
        let mut enumerator = DeviceEnumerator::new();

        mgr.list();

        mgr.init();

        for device in enumerator.iter() {
            let exec_unit = ExecuteUnit::new(device.clone());

            for (_, v) in mgr.builtins.iter() {
                let _ = v.cmd(&exec_unit, 0, vec![], true);
            }
        }

        mgr.exit();
    }
}
