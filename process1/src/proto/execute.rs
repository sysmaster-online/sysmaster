use std::{cell::RefCell, rc::Rc};

use super::{CommandRequest, CommandResponse, Manager, MngrComm, RequestData, SysComm, UnitComm};

pub trait Executer {
    /// 处理 Command，返回 Response
    fn execute(self, manager: Rc<RefCell<Manager>>) -> CommandResponse;
}

pub fn dispatch(cmd: CommandRequest, manager: Rc<RefCell<Manager>>) -> CommandResponse {
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
    fn execute(self, _manager: Rc<RefCell<Manager>>) -> CommandResponse {
        todo!()
    }
}

impl Executer for MngrComm {
    fn execute(self, _manager: Rc<RefCell<Manager>>) -> CommandResponse {
        todo!()
    }
}

impl Executer for SysComm {
    fn execute(self, _manager: Rc<RefCell<Manager>>) -> CommandResponse {
        todo!()
    }
}
