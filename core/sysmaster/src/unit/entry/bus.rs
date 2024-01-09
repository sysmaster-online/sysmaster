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

use super::config::UeConfig;
use core::error::*;
use std::rc::Rc;

pub struct UeBus {
    // associated objects
    config: Rc<UeConfig>,
    // owned objects
}

impl UeBus {
    pub(super) fn new(configr: &Rc<UeConfig>) -> UeBus {
        UeBus {
            config: Rc::clone(configr),
        }
    }

    pub(super) fn set_property(&self, key: &str, value: &str) -> Result<()> {
        self.config.set_property(key, value)
    }
}
