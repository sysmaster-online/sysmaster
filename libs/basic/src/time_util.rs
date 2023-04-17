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

//!
use std::time::SystemTime;

const USEC_INFINITY: u128 = u128::MAX;

/// usec per sec
pub const USEC_PER_SEC: u64 = 1000000;

///
pub fn timespec_load(systime: SystemTime) -> u128 {
    match systime.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => d.as_micros(),
        Err(_) => USEC_INFINITY,
    }
}
