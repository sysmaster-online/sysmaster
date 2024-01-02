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
//
#![allow(non_snake_case)]
use unit_parser::prelude::*;

#[derive(UnitConfig, Debug, Default)]
pub struct ManagerConfig {
    pub Manager: Config,
}

#[derive(UnitSection, Default, Debug)]
pub struct Config {
    #[entry(default = 90)]
    pub DefaultRestartSec: u64,
    #[entry(default = 90)]
    pub DefaultTimeoutSec: u64,

    #[entry(default = "info".to_string())]
    pub LogLevel: String,
    #[entry(default = "syslog".to_string())]
    pub LogTarget: String,
    #[entry(default = 10240)]
    pub LogFileSize: u32,
    #[entry(default = 10)]
    pub LogFileNumber: u32,

    #[entry(default = 1048576)] // RELI_HISTORY_MAPSIZE_DEFAULT
    pub DbSize: usize,
}
