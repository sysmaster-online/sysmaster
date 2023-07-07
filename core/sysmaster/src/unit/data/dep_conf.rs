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

use core::unit::UnitRelations;
use std::collections::HashMap;

#[derive(Default, Clone)]
pub struct UnitDepConf {
    pub deps: HashMap<UnitRelations, Vec<String>>,
}

impl UnitDepConf {
    pub fn new() -> UnitDepConf {
        UnitDepConf {
            deps: HashMap::new(),
        }
    }
}
