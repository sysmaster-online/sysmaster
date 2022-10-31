#![allow(non_snake_case)]
use super::service_comm::ServiceUnitComm;
use super::service_rentry::{NotifyAccess, SectionService, ServiceCommand, ServiceType};
use confique::Config;
use confique::Error;
use libsysmaster::manager::ExecCommand;
use libsysmaster::ReStation;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

pub(super) struct ServiceConfig {
    // associated objects
    comm: Rc<ServiceUnitComm>,

    // owned objects
    data: Rc<RefCell<ServiceConfigData>>,
}

impl ReStation for ServiceConfig {
    // no input, no compensate

    // data
    fn db_map(&self) {
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
        }
    }

    pub(super) fn load(&self, paths: Vec<PathBuf>, update: bool) -> Result<(), Error> {
        let mut builder = ServiceConfigData::builder().env();

        log::debug!("service load path: {:?}", paths);
        // fragment
        for v in paths {
            builder = builder.file(&v);
        }

        *self.data.borrow_mut() = builder.load()?;
        if update {
            self.db_update();
        }
        Ok(())
    }

    pub(super) fn config_data(&self) -> Rc<RefCell<ServiceConfigData>> {
        self.data.clone()
    }

    pub(super) fn get_exec_cmds(&self, cmd_type: ServiceCommand) -> Option<Vec<ExecCommand>> {
        self.data.borrow().get_exec_cmds(cmd_type)
    }

    pub(super) fn service_type(&self) -> ServiceType {
        self.data.borrow().Service.Type
    }

    pub(super) fn set_notify_access(&self, v: NotifyAccess) {
        self.data.borrow_mut().set_notify_access(v);
        self.db_update();
    }

    pub(super) fn environments(&self) -> Option<Vec<String>> {
        self.data
            .borrow()
            .Service
            .Environment
            .as_ref()
            .map(|v| v.iter().map(|v| v.to_string()).collect())
    }

    pub(super) fn sockets(&self) -> Option<Vec<String>> {
        self.data
            .borrow()
            .Service
            .Sockets
            .as_ref()
            .map(|v| v.iter().map(|v| v.to_string()).collect())
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

    pub(self) fn get_exec_cmds(&self, cmd_type: ServiceCommand) -> Option<Vec<ExecCommand>> {
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
}

#[cfg(test)]
mod tests {
    use crate::service_comm::ServiceUnitComm;
    use crate::service_config::ServiceConfig;
    use std::rc::Rc;
    use tests::get_project_root;

    #[test]
    fn test_service_parse() {
        let mut file_path = get_project_root().unwrap();
        file_path.push("test_units/config.service.toml");
        let paths = vec![file_path];

        let comm = Rc::new(ServiceUnitComm::new());
        let config = ServiceConfig::new(&comm);

        let result = config.load(paths, false);

        println!("service data: {:?}", config.config_data());

        assert!(result.is_ok());
    }
}
