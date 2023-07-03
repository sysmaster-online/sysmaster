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
use confique::{Config, FileFormat, Partial};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::rc::Rc;
use sysmaster::error::*;
use sysmaster::exec::ExecCommand;
use sysmaster::rel::ReStation;
use sysmaster::unit::KillContext;

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
                    log::error!("Failed to load {path:?}: {e}, skipping");
                    continue;
                }
                Ok(v) => partial.with_fallback(v),
            }
        }
        partial = partial.with_fallback(ConfigPartial::default_values());
        *self.data.borrow_mut() = match ServiceConfigData::from_partial(partial) {
            Err(e) => {
                /* The error message is pretty readable, just print it out. */
                log::error!("{e}");
                return Err(Error::Confique { source: e });
            }
            Ok(v) => v,
        };

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
}

#[cfg(test)]
mod tests {
    use crate::comm::ServiceUnitComm;
    use crate::config::ServiceConfig;
    use libtests::get_project_root;
    use std::rc::Rc;

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
}
