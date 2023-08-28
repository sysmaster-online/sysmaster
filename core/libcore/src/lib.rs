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

//! innner lib of sysmaster
//! libcore
/// null_str macro
#[macro_export]
macro_rules! null_str {
    ($name:expr) => {
        String::from($name)
    };
}

pub mod exec;
pub use unit::UmIf;
pub mod error;
pub mod rel;
pub mod serialize;
pub mod specifier;
pub mod unit;
pub mod utils;
pub use error::*;
