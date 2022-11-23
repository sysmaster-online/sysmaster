//!
use std::convert::TryFrom;

pub use api::{reli_debug_enable_switch, reli_debug_get_switch, Reliability};
pub use base::{reli_dir_prepare, ReDb, ReDbRoTxn, ReDbRwTxn, ReDbTable};
use serde::{Deserialize, Serialize};
pub use station::{ReStation, ReStationKind};

// dependency: base -> {enable | last | history | pending | station} -> api
mod api;
mod base;
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
            v => Err(format!("input {} is invalid", v)),
        }
    }
}
