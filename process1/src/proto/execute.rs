//! Convert the command request into the corresponding execution action

use super::{
    sys_comm, unit_comm, CommandRequest, CommandResponse, MngrComm, RequestData, SysComm, UnitComm,
};
use crate::manager::Manager;
use http::StatusCode;
use nix::sys::reboot::RebootMode;
use std::rc::Rc;

pub(crate) trait Executer {
    /// deal Command，return Response
    fn execute(self, manager: Rc<Manager>) -> CommandResponse;
}

/// Depending on the type of request
pub(crate) fn dispatch(cmd: CommandRequest, manager: Rc<Manager>) -> CommandResponse {
    println!("commandRequest :{:?}", cmd);
    let res = match cmd.request_data {
        Some(RequestData::Ucomm(param)) => param.execute(manager),
        Some(RequestData::Mcomm(param)) => param.execute(manager),
        Some(RequestData::Syscomm(param)) => param.execute(manager),
        _ => CommandResponse::default(),
    };
    println!("CommandResponse :{:?}", res);
    res
}

impl Executer for UnitComm {
    fn execute(self, manager: Rc<Manager>) -> CommandResponse {
        let ret = match self.action() {
            unit_comm::Action::Start => manager.start_unit(&self.unitname),
            unit_comm::Action::Stop => manager.stop_unit(&self.unitname),
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
    fn execute(self, _manager: Rc<Manager>) -> CommandResponse {
        todo!()
    }
}

impl Executer for SysComm {
    fn execute(self, manager: Rc<Manager>) -> CommandResponse {
        let ret = match self.action() {
            sys_comm::Action::Hibernate => manager.reboot(RebootMode::RB_SW_SUSPEND),
            sys_comm::Action::Suspend => manager.reboot(RebootMode::RB_SW_SUSPEND),
            sys_comm::Action::Halt => manager.reboot(RebootMode::RB_HALT_SYSTEM),
            sys_comm::Action::Poweroff => manager.reboot(RebootMode::RB_POWER_OFF),
            sys_comm::Action::Shutdown => manager.reboot(RebootMode::RB_POWER_OFF),
            sys_comm::Action::Reboot => manager.reboot(RebootMode::RB_AUTOBOOT),
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
