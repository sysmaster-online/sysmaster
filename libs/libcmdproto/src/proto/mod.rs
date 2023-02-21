//! Provide commands that cli can call
#[allow(missing_docs)]
#[allow(clippy::all)]
pub mod abi;
pub mod execute;
pub mod frame;

pub use abi::command_request::RequestData;
pub use abi::*;
pub use frame::ProstClientStream;
pub use frame::ProstServerStream;
pub use http::StatusCode;
use std::fmt;

impl CommandRequest {
    /// Create a new command request for unit
    pub fn new_unitcomm(action: unit_comm::Action, units: Vec<String>) -> Self {
        Self {
            request_data: Some(RequestData::Ucomm(UnitComm {
                action: action.into(),
                units,
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
    pub fn new_syscomm(action: sys_comm::Action, force: bool) -> Self {
        Self {
            request_data: Some(RequestData::Syscomm(SysComm {
                action: action.into(),
                force,
            })),
        }
    }

    /// Create a new command request for unit file
    pub fn new_unitfile(action: unit_file::Action, unitfile: Vec<String>) -> Self {
        Self {
            request_data: Some(RequestData::Ufile(UnitFile {
                action: action.into(),
                unitname: unitfile,
            })),
        }
    }
}

impl fmt::Display for sys_comm::Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", format!("{self:?}").to_lowercase())
    }
}
