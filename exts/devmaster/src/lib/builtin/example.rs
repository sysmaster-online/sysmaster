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
use crate::error::*;
use crate::rules::exec_unit::ExecuteUnit;
use snafu::ResultExt;

/// example for implementing Builtin trait
pub struct Example;

impl Builtin for Example {
    /// builtin command
    fn cmd(
        &self,
        exec_unit: &ExecuteUnit,
        _argc: i32,
        _argv: Vec<String>,
        test: bool,
    ) -> Result<bool> {
        let device = exec_unit.get_device();

        let syspath = device.get_syspath().context(DeviceSnafu)?;

        self.add_property(device, test, "ID_EXAMPLE_SYSPATH", &syspath)
            .map_err(|_| Error::BuiltinCommandError {
                msg: "add property failed".to_string(),
            })?;

        Ok(true)
    }

    /// builtin init function
    fn init(&self) {}

    /// builtin exit function
    fn exit(&self) {}

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
    use crate::{builtin::Builtin, rules::exec_unit::ExecuteUnit};
    use device::device_enumerator::DeviceEnumerator;

    #[test]
    #[ignore]
    fn test_builtin_example() {
        let mut enumerator = DeviceEnumerator::new();

        for device in enumerator.iter() {
            let exec_unit = ExecuteUnit::new(device.clone());

            let builtin = Example {};
            builtin.cmd(&exec_unit, 0, vec![], true).unwrap();

            device.get_property_value("ID_EXAMPLE_SYSPATH").unwrap();
        }
    }
}
