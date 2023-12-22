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

//! This crate provides C ABI compatible with libudev.

/// libudev
pub mod libudev;

/// libudev_device
pub mod libudev_device;
/// libudev_enumerate
pub mod libudev_enumerate;
/// libudev_hwdb
pub mod libudev_hwdb;
/// libudev_list
pub mod libudev_list;
/// libudev_monitor
pub mod libudev_monitor;
/// libudev_queue
pub mod libudev_queue;

#[macro_export]
/// if the expression is not true, return specified value
macro_rules! assert_return {
    ( $x:expr, $r:expr ) => {
        if !($x) {
            return $r;
        }
    };
}
