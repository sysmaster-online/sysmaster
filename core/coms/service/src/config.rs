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
//
#![allow(non_snake_case)]
use super::comm::ServiceUnitComm;
use super::rentry::{NotifyAccess, SectionService, ServiceCommand, ServiceType};
use basic::unit_name::unit_name_to_instance;
use confique::{Config, FileFormat, Partial};
use constants::{USEC_INFINITY, USEC_PER_SEC};
use core::error::*;
use core::exec::ExecCommand;
use core::rel::ReStation;
use core::specifier::{UnitSpecifierData, LONG_LINE_MAX};
use core::unit::KillContext;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::rc::Rc;

pub(super) struct ServiceConfig {
    // associated objects
    comm: Rc<ServiceUnitComm>,

    // owned objects
    data: Rc<RefCell<ServiceConfigData>>,

    // resolved from ServiceConfigData
    kill_context: Rc<KillContext>,
}

impl ReStation for ServiceConfig {
    // no input, no compensate

    // data
    fn db_map(&self, reload: bool) {
        if reload {
            return;
        }
        if let Some(conf) = self.comm.rentry_conf_get() {
            self.data.replace(ServiceConfigData::new(conf));
        }
    }

    fn db_insert(&self) {
        self.comm.rentry_conf_insert(&self.data.borrow().Service);
    }

    // reload: no external connections, no entry
}

impl ServiceConfig {
    pub(super) fn new(commr: &Rc<ServiceUnitComm>) -> Self {
        ServiceConfig {
            comm: Rc::clone(commr),
            data: Rc::new(RefCell::new(ServiceConfigData::default())),
            kill_context: Rc::new(KillContext::default()),
        }
    }

    pub(super) fn load(&self, paths: Vec<PathBuf>, update: bool) -> Result<()> {
        type ConfigPartial = <ServiceConfigData as Config>::Partial;
        let mut partial: ConfigPartial = Partial::from_env().context(ConfiqueSnafu)?;
        /* The first config wins, so add default values at last. */
        log::debug!("Loading service config from: {:?}", paths);
        for path in paths {
            partial = match confique::File::with_format(&path, FileFormat::Toml).load() {
                Err(e) => {
                    log::error!("Failed to load {:?}: {}, skipping", path, e);
                    continue;
                }
                Ok(v) => partial.with_fallback(v),
            }
        }
        partial = partial.with_fallback(ConfigPartial::default_values());
        *self.data.borrow_mut() = match ServiceConfigData::from_partial(partial) {
            Err(e) => {
                /* The error message is pretty readable, just print it out. */
                log::error!("{}", e);
                return Err(Error::Confique { source: e });
            }
            Ok(v) => v,
        };
        self.data.borrow_mut().verify();

        let mut unit_specifier_data = UnitSpecifierData::new();
        unit_specifier_data.instance = unit_name_to_instance(&self.comm.get_owner_id());
        self.data
            .borrow_mut()
            .update_with_specifier_escape(&unit_specifier_data);

        if update {
            self.db_update();
        }

        Ok(())
    }

    pub(super) fn config_data(&self) -> Rc<RefCell<ServiceConfigData>> {
        self.data.clone()
    }

    pub(super) fn get_exec_cmds(&self, cmd_type: ServiceCommand) -> Option<VecDeque<ExecCommand>> {
        self.data.borrow().get_exec_cmds(cmd_type)
    }

    pub(super) fn service_type(&self) -> ServiceType {
        self.data.borrow().Service.Type
    }

    pub(super) fn set_notify_access(&self, v: NotifyAccess) {
        self.data.borrow_mut().set_notify_access(v);
        self.db_update();
    }

    pub(super) fn environments(&self) -> Option<HashMap<String, String>> {
        self.data.borrow().Service.Environment.clone()
    }

    pub(super) fn sockets(&self) -> Option<Vec<String>> {
        self.data
            .borrow()
            .Service
            .Sockets
            .as_ref()
            .map(|v| v.iter().map(|v| v.to_string()).collect())
    }

    pub(super) fn kill_context(&self) -> Rc<KillContext> {
        self.kill_context.clone()
    }

    pub(super) fn flush_timeout(&self) {
        let time_out = self.data.borrow().Service.TimeoutSec;
        if time_out == 0 {
            return;
        }

        self.data.borrow_mut().set_timeout_start(time_out);
        self.data.borrow_mut().set_timeout_stop(time_out);
    }

    pub(super) fn pid_file(&self) -> Option<PathBuf> {
        self.data.borrow().Service.PIDFile.clone()
    }
}

fn specifier_escape_exec_command(
    exec_command: &mut Option<VecDeque<ExecCommand>>,
    max_len: usize,
    unit_specifier_data: &UnitSpecifierData,
) {
    let ret_exec_command = exec_command.clone();

    if let Some(mut commands) = ret_exec_command {
        for cmd in &mut commands {
            cmd.specifier_escape_full(max_len, unit_specifier_data);
        }

        *exec_command = Some(commands);
    }
}

#[derive(Config, Default, Debug)]
pub(super) struct ServiceConfigData {
    #[config(nested)]
    pub Service: SectionService,
}

impl ServiceConfigData {
    pub(self) fn new(Service: SectionService) -> ServiceConfigData {
        ServiceConfigData { Service }
    }

    pub(self) fn set_notify_access(&mut self, v: NotifyAccess) {
        self.Service.set_notify_access(v)
    }

    // keep consistency with the configuration, so just copy from configuration.
    pub(self) fn get_exec_cmds(&self, cmd_type: ServiceCommand) -> Option<VecDeque<ExecCommand>> {
        match cmd_type {
            ServiceCommand::Condition => self.Service.ExecCondition.clone(),
            ServiceCommand::StartPre => self.Service.ExecStartPre.clone(),
            ServiceCommand::Start => self.Service.ExecStart.clone(),
            ServiceCommand::StartPost => self.Service.ExecStartPost.clone(),
            ServiceCommand::Reload => self.Service.ExecReload.clone(),
            ServiceCommand::Stop => self.Service.ExecStop.clone(),
            ServiceCommand::StopPost => self.Service.ExecStopPost.clone(),
        }
    }

    pub(self) fn set_timeout_start(&mut self, time_out: u64) {
        self.Service.set_timeout_start(time_out);
    }

    pub(self) fn set_timeout_stop(&mut self, time_out: u64) {
        self.Service.set_timeout_stop(time_out);
    }

    pub(self) fn verify(&mut self) {
        if self.Service.WatchdogSec >= USEC_INFINITY / USEC_PER_SEC {
            self.Service.WatchdogSec = 0;
        } else {
            self.Service.WatchdogSec *= USEC_PER_SEC;
        }
        if self.Service.RestartSec >= USEC_INFINITY / USEC_PER_SEC {
            self.Service.RestartSec = USEC_PER_SEC;
        } else {
            self.Service.RestartSec *= USEC_PER_SEC;
        }
    }

    pub(self) fn update_with_specifier_escape(&mut self, unit_specifier_data: &UnitSpecifierData) {
        specifier_escape_exec_command(
            &mut self.Service.ExecStart,
            LONG_LINE_MAX,
            unit_specifier_data,
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::comm::ServiceUnitComm;
    use crate::config::ServiceConfig;
    use crate::rentry::ServiceType;
    use basic::unit_name::unit_name_to_instance;
    use core::exec::ExecCommand;
    use core::specifier::UnitSpecifierData;
    use libtests::get_project_root;
    use std::{collections::VecDeque, rc::Rc};

    #[test]
    fn test_service_parse() {
        let mut file_path = get_project_root().unwrap();
        file_path.push("tests/test_units/config.service");

        let paths = vec![file_path];

        let comm = Rc::new(ServiceUnitComm::new());
        let config = ServiceConfig::new(&comm);

        let result = config.load(paths, false);

        println!("service data: {:?}", config.config_data());

        assert!(result.is_ok());
    }
    #[test]
    fn test_get_service_type() {
        let mut file_path = get_project_root().unwrap();
        file_path.push("tests/test_units/config.service");

        let paths = vec![file_path];

        let comm = Rc::new(ServiceUnitComm::new());
        let config = ServiceConfig::new(&comm);

        assert!(config.load(paths, false).is_ok());
        assert_eq!(config.service_type(), ServiceType::Simple)
    }

    #[test]
    fn test_service_specifier_escape() {
        let comm = Rc::new(ServiceUnitComm::new());
        let config = ServiceConfig::new(&comm);

        // Construct ExecStart="/bin/%i %i %i ; /bin/%I %I %I"
        let mut src = VecDeque::new();
        let tmp_strings = ["%i".to_string(), "%I".to_string()];
        for tmp in tmp_strings.iter() {
            let argv = vec![tmp.to_string(), tmp.to_string()];
            let cmd = ExecCommand::new("/bin/".to_string() + tmp, argv);
            src.push_back(cmd);
        }
        config.data.borrow_mut().Service.ExecStart = Some(src);

        // Construct instance="Hal\\xc3\\xb6-chen"
        let mut unit_specifier_data = UnitSpecifierData::new();
        unit_specifier_data.instance = unit_name_to_instance("config@Hal\\xc3\\xb6-chen.service");

        config
            .data
            .borrow_mut()
            .update_with_specifier_escape(&unit_specifier_data);

        let mut dst = VecDeque::new();
        let tmp_strings = ["Hal\\xc3\\xb6-chen".to_string(), "Halö/chen".to_string()];
        for tmp in tmp_strings.iter() {
            let argv = vec![tmp.to_string(), tmp.to_string()];
            let cmd = ExecCommand::new("/bin/".to_string() + tmp, argv);
            dst.push_back(cmd);
        }

        assert_eq!(config.data.borrow().Service.ExecStart, Some(dst));
    }
}
