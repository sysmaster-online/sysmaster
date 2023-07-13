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

//! hwdb builtin
//!

use crate::builtin::Builtin;
use crate::error::Result;
use crate::rules::exec_unit::ExecuteUnit;

/// hwdb builtin command
pub struct Hwdb;

impl Builtin for Hwdb {
    /// builtin command
    fn cmd(
        &self,
        _exec_unit: &ExecuteUnit,
        _argc: i32,
        _argv: Vec<String>,
        _test: bool,
    ) -> Result<bool> {
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
        "Hardware database".to_string()
    }

    /// whether the builtin command can only run once
    fn run_once(&self) -> bool {
        false
    }
}
