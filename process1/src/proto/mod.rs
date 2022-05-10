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
    pub fn new_unitcomm(action: unit_comm::Action, unitname: impl Into<String>) -> Self {
        Self {
            request_data: Some(RequestData::Ucomm(UnitComm {
                action: action.into(),
                unitname: unitname.into(),
            })),
        }
    }

    pub fn new_mngrcomm(action: mngr_comm::Action) -> Self {
        Self {
            request_data: Some(RequestData::Mcomm(MngrComm {
                action: action.into(),
            })),
        }
    }
}
