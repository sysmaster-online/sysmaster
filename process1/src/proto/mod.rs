pub mod abi;
pub mod execute;
pub mod frame;

pub use super::manager::manager::Manager;
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

    /// 转换成 string 做错误处理
    pub fn format(&self) -> String {
        format!("{:?}", self)
    }
}

impl CommandResponse {
    pub fn ok() -> Self {
        CommandResponse {
            status: StatusCode::OK.as_u16() as _,
            ..Default::default()
        }
    }

    pub fn internal_error(msg: String) -> Self {
        CommandResponse {
            status: StatusCode::INTERNAL_SERVER_ERROR.as_u16() as _,
            message: msg,
            // ..Default::default()
        }
    }

    /// 转换成 string 做错误处理
    pub fn format(&self) -> String {
        format!("{:?}", self)
    }
}
