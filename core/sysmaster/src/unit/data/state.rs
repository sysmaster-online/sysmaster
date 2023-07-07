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

use core::unit::{UnitActiveState, UnitNotifyFlags};

#[derive(Debug, Clone)]
pub(crate) struct UnitState {
    pub(crate) os: UnitActiveState,
    pub(crate) ns: UnitActiveState,
    pub(crate) flags: UnitNotifyFlags,
}

impl UnitState {
    pub(crate) fn new(
        os: UnitActiveState,
        ns: UnitActiveState,
        flags: UnitNotifyFlags,
    ) -> UnitState {
        UnitState { os, ns, flags }
    }
}
