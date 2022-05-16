use super::{
    unit_comm::Action, CommandRequest, CommandResponse, MngrComm, RequestData, SysComm, UnitComm,
};
use crate::manager::Manager;
use http::StatusCode;
use std::rc::Rc;

pub(crate) trait Executer {
    /// 处理 Command，返回 Response
    fn execute(self, manager: Rc<Manager>) -> CommandResponse;
}

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
            Action::Start => manager.start_unit(&self.unitname),
            Action::Stop => manager.stop_unit(&self.unitname),
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
    fn execute(self, _manager: Rc<Manager>) -> CommandResponse {
        todo!()
    }
}
