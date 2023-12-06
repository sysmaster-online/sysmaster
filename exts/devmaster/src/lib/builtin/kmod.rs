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

//! kmod builtin
//!

use crate::builtin::Builtin;
use crate::error::Result;
use crate::rules::exec_unit::ExecuteUnit;
use kmod_rs;
use kmod_rs::KmodResources;
use std::cell::RefCell;
use std::rc::Rc;

/// kmod builtin command
pub(crate) struct Kmod {
    /// kmod struct
    kernel_module: Option<Rc<RefCell<kmod_rs::LibKmod>>>,
}

impl Kmod {
    /// create Kmod
    pub(crate) fn new() -> Kmod {
        Kmod {
            kernel_module: kmod_rs::LibKmod::new().map(|inner| Rc::new(RefCell::new(inner))),
        }
    }
}

impl Default for Kmod {
    fn default() -> Self {
        Self::new()
    }
}

impl Builtin for Kmod {
    /// builtin command
    fn cmd(
        &self,
        exec_unit: &ExecuteUnit,
        argc: i32,
        argv: Vec<String>,
        _test: bool,
    ) -> Result<bool> {
        let device = exec_unit.get_device();

        if self.kernel_module.is_none() {
            log::error!("Kmod context is not loaded.");
            return Ok(true);
        }

        if argc < 2 || argv[1] != *"load" {
            return Err(crate::error::Error::BuiltinCommandError {
                msg: "Too few argument".to_string(),
            });
        }

        if argc == 2 {
            let modalias = device
                .get_property_value("MODALIAS")
                .map_or(String::new(), |e| e);

            if !modalias.is_empty() {
                if let Err(e) = self
                    .kernel_module
                    .as_ref()
                    .unwrap()
                    .borrow_mut()
                    .module_load_and_warn(&modalias, false)
                {
                    log::debug!("Load module {} failed: {}", modalias, e);
                }
            }
        } else {
            for i in 2..argc {
                if let Err(e) = self
                    .kernel_module
                    .as_ref()
                    .unwrap()
                    .borrow_mut()
                    .module_load_and_warn(&argv[i as usize], false)
                {
                    log::error!("Load module {} failed, {}", argv[i as usize], e);
                }
            }
        }
        Ok(true)
    }

    /// builtin init function
    fn init(&self) {
        if let Err(e) = self
            .kernel_module
            .as_ref()
            .unwrap()
            .borrow_mut()
            .load_resources()
        {
            log::error!("Load resources failed! {}", e);
        }
    }

    /// builtin exit function
    fn exit(&self) {}

    /// check whether builtin command should reload
    fn should_reload(&self) -> bool {
        self.kernel_module
            .as_ref()
            .unwrap()
            .borrow_mut()
            .validate_resources()
            .map_or(false, |e| {
                log::debug!("Kernel module index needs reloading.");
                e != KmodResources::KmodResourceOk
            })
    }

    /// the help of builtin command
    fn help(&self) -> String {
        "Kernel module loader".to_string()
    }

    /// whether the builtin command can only run once
    fn run_once(&self) -> bool {
        false
    }
}
