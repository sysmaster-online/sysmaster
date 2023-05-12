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

//! example to implement a builtin command
//!

use crate::builtin::Builtin;
use crate::builtin::Netlink;
use crate::error::{Error, Result};
use device::Device;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

/// example for implementing Builtin trait
pub struct Example;

impl Builtin for Example {
    /// builtin command
    fn cmd(
        &self,
        device: Arc<Mutex<Device>>,
        ret_rtnl: &mut RefCell<Option<Netlink>>,
        _argc: i32,
        _argv: Vec<String>,
        test: bool,
    ) -> Result<bool> {
        println!("example builtin run");

        let syspath = match device.lock().unwrap().get_syspath() {
            Some(p) => String::from(p),
            None => {
                return Err(Error::BuiltinCommandError {
                    msg: "syspath invalid".to_string(),
                })
            }
        };

        ret_rtnl.replace(Some(Netlink {}));

        self.add_property(device, test, "ID_EXAMPLE_SYSPATH".to_string(), syspath)
            .map_err(|_| Error::BuiltinCommandError {
                msg: "add property failed".to_string(),
            })?;

        Ok(true)
    }

    /// builtin init function
    fn init(&self) {
        println!("example builtin init");
    }

    /// builtin exit function
    fn exit(&self) {
        println!("example builtin exit");
    }

    /// check whether builtin command should reload
    fn should_reload(&self) -> bool {
        false
    }

    /// the help of builtin command
    fn help(&self) -> String {
        "Example implementation for builtin commands".to_string()
    }

    /// whether the builtin command can only run once
    fn run_once(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::Example;
    use crate::builtin::{Builtin, Netlink};
    use device::device_enumerator::DeviceEnumerator;
    use std::cell::RefCell;

    #[test]
    fn test_builtin_example() {
        let mut enumerator = DeviceEnumerator::new();

        for device in enumerator.iter_mut() {
            let mut rtnl = RefCell::<Option<Netlink>>::from(None);

            let builtin = Example {};
            builtin
                .cmd(device.clone(), &mut rtnl, 0, vec![], true)
                .unwrap();

            device
                .lock()
                .unwrap()
                .get_property_value("ID_EXAMPLE_SYSPATH")
                .unwrap();
            rtnl.take().unwrap();
        }
    }
}
