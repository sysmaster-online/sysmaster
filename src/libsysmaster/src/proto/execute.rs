//! Convert the command request into the corresponding execution action
use super::{
    sys_comm, unit_comm, CommandRequest, CommandResponse, MngrComm, RequestData, SysComm, UnitComm,
    UnitFile,
};

use http::StatusCode;
use libutils::Result;
use std::io::Error;
use std::rc::Rc;


/// error number of manager
#[derive(Debug)]
pub enum ExecCmdErrno {
    /// invalid input
    Input,
    /// not existed
    NotExisted,
    /// Internal error
    Internal,
    /// not supported
    NotSupported,
}

pub(crate) trait Executer {
    /// deal Command，return Response
    fn execute(self, manager: Rc<impl ExecuterAction>) -> CommandResponse;
}

/// ExecuterAction
pub trait ExecuterAction {
    /// start the unit_name
    fn start(&self, unit_name: &str) -> Result<(), ExecCmdErrno>;
    /// stop the unit_name
    fn stop(&self, unit_name: &str) -> Result<(), ExecCmdErrno>;
    /// suspend host
    fn suspend(&self) -> Result<i32>;
    /// poweroff host
    fn poweroff(&self) -> Result<i32>;
    /// reboot host
    fn reboot(&self) -> Result<i32>;
    /// halt host
    fn halt(&self) -> Result<i32>;
    /// disable unit_name
    fn disable(&self, unit_name: &str) -> Result<(), Error>;
    /// enable unit_name
    fn enable(&self, unit_name: &str) -> Result<(), Error>;
}

/// Depending on the type of request
pub(crate) fn dispatch<T>(cmd: CommandRequest, manager: Rc<T>) -> CommandResponse
where
    T: ExecuterAction,
{
    println!("commandRequest :{:?}", cmd);
    let res = match cmd.request_data {
        Some(RequestData::Ucomm(param)) => param.execute(manager),
        Some(RequestData::Mcomm(param)) => param.execute(manager),
        Some(RequestData::Syscomm(param)) => param.execute(manager),
        Some(RequestData::Ufile(param)) => param.execute(manager),
        _ => CommandResponse::default(),
    };
    println!("CommandResponse :{:?}", res);
    res
}

impl Executer for UnitComm {
    fn execute(self, manager: Rc<impl ExecuterAction>) -> CommandResponse {
        let ret = match self.action() {
            unit_comm::Action::Start => manager.start(&self.unitname),
            unit_comm::Action::Stop => manager.stop(&self.unitname),
            _ => todo!(),
        };
        match ret {
            Ok(_) => CommandResponse {
                status: StatusCode::OK.as_u16() as _,
                ..Default::default()
            },
            Err(_e) => CommandResponse {
                status: StatusCode::INTERNAL_SERVER_ERROR.as_u16() as _,
                message: String::from("error."),
            },
        }
    }
}

impl Executer for MngrComm {
    fn execute(self, _manager: Rc<impl ExecuterAction>) -> CommandResponse {
        todo!()
    }
}

impl Executer for SysComm {
    fn execute(self, manager: Rc<impl ExecuterAction>) -> CommandResponse {
        let ret = match self.action() {
            sys_comm::Action::Hibernate => manager.suspend(),
            sys_comm::Action::Suspend => manager.suspend(),
            sys_comm::Action::Halt => manager.halt(),
            sys_comm::Action::Poweroff => manager.poweroff(),
            sys_comm::Action::Shutdown => manager.poweroff(),
            sys_comm::Action::Reboot => manager.reboot(),
        };
        match ret {
            Ok(_) => CommandResponse {
                status: StatusCode::OK.as_u16() as _,
                ..Default::default()
            },
            Err(_e) => CommandResponse {
                status: StatusCode::INTERNAL_SERVER_ERROR.as_u16() as _,
                message: String::from("error."),
            },
        }
    }
}

impl Executer for UnitFile {
    fn execute(self, manager: Rc<impl ExecuterAction>) -> CommandResponse {
        let ret = match self.action() {
            super::unit_file::Action::Enable => manager.enable(&self.unitname),
            super::unit_file::Action::Disable => manager.disable(&self.unitname),
            _ => todo!(),
        };
        match ret {
            Ok(_) => CommandResponse {
                status: StatusCode::OK.as_u16() as _,
                ..Default::default()
            },
            Err(_e) => CommandResponse {
                status: StatusCode::INTERNAL_SERVER_ERROR.as_u16() as _,
                message: String::from("error."),
            },
        }
    }
}
