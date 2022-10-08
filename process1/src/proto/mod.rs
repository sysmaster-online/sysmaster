//! Provide commands that cli can call

#![allow(missing_docs)]
#[allow(clippy::all)]
pub mod abi;
pub mod execute;
pub mod frame;

use super::manager::Manager;
pub use abi::command_request::RequestData;
pub use abi::*;
pub use frame::ProstClientStream;
pub use frame::ProstServerStream;
pub use http::StatusCode;
// use prost::Message;

impl CommandRequest {
    /// Create a new command request for unit
    pub fn new_unitcomm(action: unit_comm::Action, unitname: impl Into<String>) -> Self {
        Self {
            request_data: Some(RequestData::Ucomm(UnitComm {
                action: action.into(),
                unitname: unitname.into(),
            })),
        }
    }

    /// Create a new command request for manager
    pub fn new_mngrcomm(action: mngr_comm::Action) -> Self {
        Self {
            request_data: Some(RequestData::Mcomm(MngrComm {
                action: action.into(),
            })),
        }
    }

    /// Create a new command request for system
    pub fn new_syscomm(action: sys_comm::Action) -> Self {
        Self {
            request_data: Some(RequestData::Syscomm(SysComm {
                action: action.into(),
            })),
        }
    }
}
