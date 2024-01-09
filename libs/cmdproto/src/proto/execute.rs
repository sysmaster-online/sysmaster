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
    mngr_comm, sys_comm, transient_unit_comm, unit_comm, CommandRequest, CommandResponse, MngrComm,
    RequestData, SwitchRootComm, SysComm, TransientUnitComm, UnitComm, UnitFile,
};

use crate::error::*;
use http::StatusCode;
use nix::{self, sys::socket::UnixCredentials};
use std::{fmt::Display, rc::Rc};

pub(crate) trait Executer {
    /// deal Command，return Response
    fn execute(
        self,
        manager: Rc<impl ExecuterAction>,
        call_back: Option<fn(&str) -> String>,
        cred: Option<UnixCredentials>,
    ) -> CommandResponse;
}

/// ExecuterAction
pub trait ExecuterAction {
    #[allow(missing_docs)]
    type Error: Display + Into<nix::Error>;
    #[allow(missing_docs)]
    type Status: Display + Into<nix::Error>;
    /// start the unit_name
    fn start(&self, unit_name: &str) -> Result<(), Self::Error>;
    /// stop the unit_name
    fn stop(&self, unit_name: &str) -> Result<(), Self::Error>;
    /// restart the unit_name
    fn restart(&self, unit_name: &str) -> Result<(), Self::Error>;
    /// reload the unit_name
    fn reload(&self, unit_name: &str) -> Result<(), Self::Error>;
    /// isolate the unit_name
    fn isolate(&self, unit_name: &str) -> Result<(), Self::Error>;
    /// reset the failed unit_name
    fn reset_failed(&self, unit_name: &str) -> Result<(), Self::Error>;
    /// show the status of unit_name
    fn status(&self, unit_name: &str) -> Result<Self::Status, Self::Error>;
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
    /// switch root
    fn switch_root(&self, init: &[String]) -> Result<(), Self::Error>;
    /// transient unit
    fn start_transient_unit(
        &self,
        job_mode: &str,
        unit_config: &transient_unit_comm::UnitConfig,
        aux_units: &[transient_unit_comm::UnitConfig],
    ) -> Result<(), Self::Error>;
}

/// Depending on the type of request
pub(crate) fn dispatch<T>(
    cmd: CommandRequest,
    manager: Rc<T>,
    cred: Option<UnixCredentials>,
) -> CommandResponse
where
    T: ExecuterAction,
{
    // log::trace!("commandRequest :{cmd:?}");
    let call_back = |unit_name: &str| {
        // If users didn't specify the unit type, treat it as

        if !unit_name.contains('.') {
            unit_name.to_string() + ".service"
        } else {
            unit_name.to_string()
        }
    };

    match cmd.request_data {
        Some(RequestData::Ucomm(param)) => param.execute(manager, Some(call_back), cred),
        Some(RequestData::Mcomm(param)) => param.execute(manager, None, cred),
        Some(RequestData::Syscomm(param)) => param.execute(manager, Some(call_back), cred),
        Some(RequestData::Ufile(param)) => param.execute(manager, Some(call_back), cred),
        Some(RequestData::Srcomm(param)) => param.execute(manager, None, cred),
        Some(RequestData::Trancomm(param)) => param.execute(manager, None, cred),
        _ => CommandResponse::default(),
    }
}

fn new_line_break(s: &mut String) {
    if !s.is_empty() {
        *s += "\n";
    }
}

fn response_if_credential_dissatisfied(
    cred: Option<UnixCredentials>,
    command_is_allowed_for_nonroot: bool,
) -> Option<CommandResponse> {
    let sender = match cred {
        None => {
            return Some(CommandResponse {
                status: StatusCode::OK.as_u16() as _,
                error_code: 1,
                message: "Failed to execute your command: cannot determine user credentials."
                    .to_string(),
            })
        }
        Some(v) => v.uid(),
    };
    if sender != 0 && !command_is_allowed_for_nonroot {
        return Some(CommandResponse {
            status: StatusCode::OK.as_u16() as _,
            error_code: 1,
            message: "Failed to execute your command: Operation not permitted.".to_string(),
        });
    }
    None
}

impl Executer for UnitComm {
    fn execute(
        self,
        manager: Rc<impl ExecuterAction>,
        call_back: Option<fn(&str) -> String>,
        cred: Option<UnixCredentials>,
    ) -> CommandResponse {
        if let Some(v) = response_if_credential_dissatisfied(
            cred,
            [unit_comm::Action::Status].contains(&self.action()),
        ) {
            return v;
        }

        let mut reply = String::new();
        let mut units: Vec<String> = Vec::new();
        let mut error_code: u32 = 0;
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
                            reply += &status.to_string();
                            error_code = status.into() as u32 | ERROR_CODE_MASK_PRINT_STDOUT;
                        }
                        Err(e) => {
                            reply =
                                format!("{}Failed to show the status of {}: {}", reply, unit, e);
                            error_code = e.into() as u32;
                        }
                    }
                }
            }
            unit_comm::Action::Start => {
                for unit in units {
                    if let Err(e) = manager.start(&unit) {
                        new_line_break(&mut reply);
                        reply = format!("{}Failed to start {}: {}", reply, unit, e);
                        error_code = e.into() as u32;
                    }
                }
            }
            unit_comm::Action::Stop => {
                for unit in units {
                    if let Err(e) = manager.stop(&unit) {
                        new_line_break(&mut reply);
                        reply = format!("{}Failed to stop {}: {}", reply, unit, e);
                        error_code = e.into() as u32;
                    }
                }
            }
            unit_comm::Action::Restart => {
                for unit in units {
                    if let Err(e) = manager.restart(&unit) {
                        new_line_break(&mut reply);
                        reply = format!("{}Failed to restart {}: {}", reply, unit, e);
                        error_code = e.into() as u32;
                    }
                }
            }
            unit_comm::Action::Reload => {
                for unit in units {
                    if let Err(e) = manager.reload(&unit) {
                        new_line_break(&mut reply);
                        reply = format!("{}Failed to reload {}: {}", reply, unit, e);
                        error_code = e.into() as u32;
                    }
                }
            }
            unit_comm::Action::Isolate => {
                for unit in units {
                    if let Err(e) = manager.isolate(&unit) {
                        new_line_break(&mut reply);
                        reply = format!("{}Failed to isolate {}: {}", reply, unit, e);
                        error_code = e.into() as u32;
                    }
                }
            }
            unit_comm::Action::Resetfailed => {
                for unit in units {
                    if let Err(e) = manager.reset_failed(&unit) {
                        new_line_break(&mut reply);
                        reply = format!("{}Failed to reset-failed {}: {}", reply, unit, e);
                        error_code = e.into() as u32;
                    }
                }
            }
            _ => todo!(),
        }
        CommandResponse {
            status: StatusCode::OK.as_u16() as _,
            error_code,
            message: reply,
        }
    }
}

impl Executer for MngrComm {
    fn execute(
        self,
        manager: Rc<impl ExecuterAction>,
        _call_back: Option<fn(&str) -> String>,
        cred: Option<UnixCredentials>,
    ) -> CommandResponse {
        if let Some(v) = response_if_credential_dissatisfied(
            cred,
            [mngr_comm::Action::Listunits].contains(&self.action()),
        ) {
            return v;
        }

        match self.action() {
            mngr_comm::Action::Reexec => {
                manager.daemon_reexec();
                CommandResponse {
                    status: StatusCode::OK.as_u16() as _,
                    error_code: 0,
                    ..Default::default()
                }
            }

            mngr_comm::Action::Reload => {
                manager.daemon_reload();
                CommandResponse {
                    status: StatusCode::OK.as_u16() as _,
                    error_code: 0,
                    ..Default::default()
                }
            }

            mngr_comm::Action::Listunits => match manager.list_units() {
                Ok(m) => CommandResponse {
                    status: StatusCode::OK.as_u16() as _,
                    error_code: 0,
                    message: m,
                },
                Err(e) => {
                    let error_message = format!("Failed to list all units:{}", e);
                    CommandResponse {
                        status: StatusCode::INTERNAL_SERVER_ERROR.as_u16() as _,
                        error_code: e.into() as u32,
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
        cred: Option<UnixCredentials>,
    ) -> CommandResponse {
        if let Some(v) = response_if_credential_dissatisfied(cred, false) {
            return v;
        }

        let ret = if !self.force {
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
                error_code: 0,
                ..Default::default()
            },
            Err(e) => CommandResponse {
                status: StatusCode::INTERNAL_SERVER_ERROR.as_u16() as _,
                error_code: e.into() as u32,
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
        cred: Option<UnixCredentials>,
    ) -> CommandResponse {
        if let Some(v) = response_if_credential_dissatisfied(cred, false) {
            return v;
        }

        let mut reply = String::new();
        let mut units: Vec<String> = Vec::new();
        let mut error_code: u32 = 0;
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
                        reply = format!("{} Failed to enable {}: {}", reply, unit, e);
                        error_code = e.into() as u32;
                    }
                }
            }
            super::unit_file::Action::Disable => {
                for unit in units {
                    if let Err(e) = manager.disable(&unit) {
                        new_line_break(&mut reply);
                        reply = format!("{} Failed to disable {}: {}", reply, unit, e);
                        error_code = e.into() as u32;
                    }
                }
            }
            super::unit_file::Action::Mask => {
                for unit in units {
                    if let Err(e) = manager.mask(&unit) {
                        new_line_break(&mut reply);
                        reply = format!("{} Failed to mask {}: {}", reply, unit, e);
                        error_code = e.into() as u32;
                    }
                }
            }
            super::unit_file::Action::Unmask => {
                for unit in units {
                    if let Err(e) = manager.unmask(&unit) {
                        new_line_break(&mut reply);
                        reply = format!("{} Failed to unmask {}: {}", reply, unit, e);
                        error_code = e.into() as u32;
                    }
                }
            }
            _ => todo!(),
        };

        CommandResponse {
            status: StatusCode::OK.as_u16() as _,
            error_code,
            message: reply,
        }
    }
}

impl Executer for SwitchRootComm {
    fn execute(
        self,
        manager: Rc<impl ExecuterAction>,
        _call_back: Option<fn(&str) -> String>,
        cred: Option<UnixCredentials>,
    ) -> CommandResponse {
        if let Some(v) = response_if_credential_dissatisfied(cred, false) {
            return v;
        }

        match manager.switch_root(&self.init) {
            Ok(_) => CommandResponse {
                status: StatusCode::OK.as_u16() as _,
                error_code: 0,
                ..Default::default()
            },
            Err(e) => CommandResponse {
                status: StatusCode::INTERNAL_SERVER_ERROR.as_u16() as _,
                error_code: e.into() as u32,
                message: String::from("error."),
            },
        }
    }
}

impl Executer for TransientUnitComm {
    fn execute(
        self,
        manager: Rc<impl ExecuterAction>,
        _call_back: Option<fn(&str) -> String>,
        cred: Option<UnixCredentials>,
    ) -> CommandResponse {
        if let Some(v) = response_if_credential_dissatisfied(cred, false) {
            return v;
        }

        if self.unit_config.is_none() {
            return CommandResponse {
                status: StatusCode::INTERNAL_SERVER_ERROR.as_u16() as _,
                error_code: 1,
                message: String::from("error."),
            };
        }

        match manager.start_transient_unit(
            &self.job_mode,
            &self.unit_config.unwrap(),
            &self.aux_units,
        ) {
            Ok(_) => CommandResponse {
                status: StatusCode::OK.as_u16() as _,
                error_code: 0,
                ..Default::default()
            },
            Err(e) => CommandResponse {
                status: StatusCode::INTERNAL_SERVER_ERROR.as_u16() as _,
                message: e.to_string(),
                error_code: e.into() as u32,
            },
        }
    }
}
