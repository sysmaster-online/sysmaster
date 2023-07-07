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

//! reliability module
pub use api::{reli_debug_get_switch, ReliConf, Reliability};
pub use base::{reli_dir_prepare, ReDb, ReDbRoTxn, ReDbRwTxn, ReDbTable};
use serde::{Deserialize, Serialize};
pub use station::{ReStation, ReStationKind};
use std::convert::TryFrom;

// dependency: base -> {enable | last | history | pending | station} -> debug -> api
mod api;
mod base;
#[cfg(debug)]
mod debug;
mod enable;
mod history;
mod last;
mod pending;
mod station;

///
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum ReliLastFrame {
    ///
    Queue = 0,
    ///
    JobManager,
    ///
    SigChld,
    ///
    CgEvent,
    ///
    Notify,
    ///
    SubManager,
    ///
    ManagerOp,
    ///
    CmdOp,
    ///
    OtherEvent,
}

impl TryFrom<u32> for ReliLastFrame {
    type Error = String;
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(ReliLastFrame::Queue),
            1 => Ok(ReliLastFrame::JobManager),
            2 => Ok(ReliLastFrame::SigChld),
            3 => Ok(ReliLastFrame::CgEvent),
            4 => Ok(ReliLastFrame::Notify),
            5 => Ok(ReliLastFrame::SubManager),
            6 => Ok(ReliLastFrame::ManagerOp),
            7 => Ok(ReliLastFrame::CmdOp),
            8 => Ok(ReliLastFrame::OtherEvent),
            v => Err(format!("input {v} is invalid")),
        }
    }
}
