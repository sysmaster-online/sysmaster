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

//! the module of devctl subcommands
//!

pub(crate) mod hwdb;
pub(crate) mod info;
pub(crate) mod monitor;
pub(crate) mod settle;
pub(crate) mod test_builtin;
pub(crate) mod trigger;
pub(self) mod utils;

pub(crate) type Result<T> = std::result::Result<T, nix::Error>;
