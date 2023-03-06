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

//! Convert the command request into the corresponding execution action
use super::{
    mngr_comm, sys_comm, unit_comm, CommandRequest, CommandResponse, MngrComm, RequestData,
    SysComm, UnitComm, UnitFile,
};

use crate::error::*;
use http::StatusCode;
use std::{fmt::Display, rc::Rc};

pub(crate) trait Executer {
    /// deal Command，return Response
    fn execute(
        self,
        manager: Rc<impl ExecuterAction>,
        call_back: Option<fn(&str) -> String>,
    ) -> CommandResponse;
}

/// ExecuterAction
pub trait ExecuterAction {
    #[allow(missing_docs)]
    type Error: Display;
    /// start the unit_name
    fn start(&self, unit_name: &str) -> Result<(), Self::Error>;
    /// stop the unit_name
    fn stop(&self, unit_name: &str) -> Result<(), Self::Error>;
    /// restart the unit_name
    fn restart(&self, unit_name: &str) -> Result<(), Self::Error>;
    /// reload the unit_name
    fn reload(&self, unit_name: &str) -> Result<(), Self::Error>;
    /// show the status of unit_name
    fn status(&self, unit_name: &str) -> Result<String, Self::Error>;
    /// list all units
    fn list_units(&self) -> Result<String, Self::Error>;
    /// suspend host
    fn suspend(&self) -> Result<i32, Self::Error>;
    /// poweroff host
    fn poweroff(&self) -> Result<i32, Self::Error>;
    /// reboot host
    fn reboot(&self) -> Result<i32, Self::Error>;
    /// halt host
    fn halt(&self) -> Result<i32, Self::Error>;
    /// disable unit_name
    fn disable(&self, unit_name: &str) -> Result<(), Self::Error>;
    /// enable unit_name
    fn enable(&self, unit_name: &str) -> Result<(), Self::Error>;
    /// mask unit_name
    fn mask(&self, unit_name: &str) -> Result<(), Self::Error>;
    /// unmask unit_name
    fn unmask(&self, unit_name: &str) -> Result<(), Self::Error>;
    /// daemon-reload
    fn daemon_reload(&self);
    /// daemon-reexec
    fn daemon_reexec(&self);
}

/// Depending on the type of request
pub(crate) fn dispatch<T>(cmd: CommandRequest, manager: Rc<T>) -> CommandResponse
where
    T: ExecuterAction,
{
    println!("commandRequest :{cmd:?}");
    let call_back = |unit_name: &str| {
        // If users didn't specify the unit type, treat it as

        if !unit_name.contains('.') {
            unit_name.to_string() + ".service"
        } else {
            unit_name.to_string()
        }
    };

    let res = match cmd.request_data {
        Some(RequestData::Ucomm(param)) => param.execute(manager, Some(call_back)),
        Some(RequestData::Mcomm(param)) => param.execute(manager, None),
        Some(RequestData::Syscomm(param)) => param.execute(manager, Some(call_back)),
        Some(RequestData::Ufile(param)) => param.execute(manager, Some(call_back)),
        _ => CommandResponse::default(),
    };
    println!("CommandResponse :{res:?}");
    res
}

fn new_line_break(s: &mut String) {
    if !s.is_empty() {
        *s += "\n";
    }
}

impl Executer for UnitComm {
    fn execute(
        self,
        manager: Rc<impl ExecuterAction>,
        call_back: Option<fn(&str) -> String>,
    ) -> CommandResponse {
        let mut reply = String::new();
        let mut units: Vec<String> = Vec::new();
        for unit_name in &self.units {
            if call_back.is_none() {
                units.push(unit_name.to_string());
                continue;
            }

            units.push(call_back.unwrap()(unit_name));
        }

        match self.action() {
            unit_comm::Action::Status => {
                for unit in units {
                    new_line_break(&mut reply);
                    match manager.status(&unit) {
                        Ok(status) => {
                            reply += &status;
                        }
                        Err(e) => {
                            reply = format!("{reply} Failed to show the status of {unit}: {e}");
                        }
                    }
                }
            }
            unit_comm::Action::Start => {
                for unit in units {
                    if let Err(e) = manager.start(&unit) {
                        new_line_break(&mut reply);
                        reply = format!("{reply} Failed to start {unit}: {e}");
                    }
                }
            }
            unit_comm::Action::Stop => {
                for unit in units {
                    if let Err(e) = manager.stop(&unit) {
                        new_line_break(&mut reply);
                        reply = format!("{reply} Failed to stop {unit}: {e}");
                    }
                }
            }
            unit_comm::Action::Restart => {
                for unit in units {
                    if let Err(e) = manager.restart(&unit) {
                        new_line_break(&mut reply);
                        reply = format!("{reply} Failed to restart {unit}: {e}");
                    }
                }
            }
            unit_comm::Action::Reload => {
                for unit in units {
                    if let Err(e) = manager.reload(&unit) {
                        new_line_break(&mut reply);
                        reply = format!("{reply} Failed to reload {unit}: {e}");
                    }
                }
            }
            _ => todo!(),
        }
        CommandResponse {
            status: StatusCode::OK.as_u16() as _,
            message: reply,
        }
    }
}

impl Executer for MngrComm {
    fn execute(
        self,
        manager: Rc<impl ExecuterAction>,
        _call_back: Option<fn(&str) -> String>,
    ) -> CommandResponse {
        match self.action() {
            mngr_comm::Action::Reexec => {
                manager.daemon_reexec();
                CommandResponse {
                    status: StatusCode::OK.as_u16() as _,
                    ..Default::default()
                }
            }

            mngr_comm::Action::Reload => {
                manager.daemon_reload();
                CommandResponse {
                    status: StatusCode::OK.as_u16() as _,
                    ..Default::default()
                }
            }

            mngr_comm::Action::Listunits => match manager.list_units() {
                Ok(m) => CommandResponse {
                    status: StatusCode::OK.as_u16() as _,
                    message: m,
                },
                Err(e) => {
                    let error_message = format!("Failed to list all units:{e}");
                    CommandResponse {
                        status: StatusCode::INTERNAL_SERVER_ERROR.as_u16() as _,
                        message: error_message,
                    }
                }
            },
        }
    }
}

impl Executer for SysComm {
    fn execute(
        self,
        manager: Rc<impl ExecuterAction>,
        _call_back: Option<fn(&str) -> String>,
    ) -> CommandResponse {
        let ret = if self.force {
            let unit_name = self.action().to_string() + ".target";
            match manager.start(&unit_name) {
                Ok(_) => Ok(0),
                Err(e) => Err(e),
            }
        } else {
            match self.action() {
                sys_comm::Action::Hibernate => manager.suspend(),
                sys_comm::Action::Suspend => manager.suspend(),
                sys_comm::Action::Halt => manager.halt(),
                sys_comm::Action::Poweroff => manager.poweroff(),
                sys_comm::Action::Shutdown => manager.poweroff(),
                sys_comm::Action::Reboot => manager.reboot(),
            }
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
    fn execute(
        self,
        manager: Rc<impl ExecuterAction>,
        call_back: Option<fn(&str) -> String>,
    ) -> CommandResponse {
        let mut reply = String::new();
        let mut units: Vec<String> = Vec::new();
        for unit_name in &self.unitname {
            if call_back.is_none() {
                units.push(unit_name.to_string());
                continue;
            }

            units.push(call_back.unwrap()(unit_name));
        }
        match self.action() {
            super::unit_file::Action::Enable => {
                for unit in units {
                    if let Err(e) = manager.enable(&unit) {
                        new_line_break(&mut reply);
                        reply = format!("{reply} Failed to enable {unit}: {e}");
                    }
                }
            }
            super::unit_file::Action::Disable => {
                for unit in units {
                    if let Err(e) = manager.disable(&unit) {
                        new_line_break(&mut reply);
                        reply = format!("{reply} Failed to disable {unit}: {e}");
                    }
                }
            }
            super::unit_file::Action::Mask => {
                for unit in units {
                    if let Err(e) = manager.mask(&unit) {
                        new_line_break(&mut reply);
                        reply = format!("{reply} Failed to mask {unit}: {e}");
                    }
                }
            }
            super::unit_file::Action::Unmask => {
                for unit in units {
                    if let Err(e) = manager.unmask(&unit) {
                        new_line_break(&mut reply);
                        reply = format!("{reply} Failed to unmask {unit}: {e}");
                    }
                }
            }
            _ => todo!(),
        };

        CommandResponse {
            status: StatusCode::OK.as_u16() as _,
            message: reply,
        }
    }
}
