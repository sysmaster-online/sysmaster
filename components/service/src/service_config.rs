#![allow(non_snake_case)]
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use super::service_base::ServiceType;
use crate::service_base::NotifyAccess;
use crate::service_base::ServiceCommand;
use confique::Config;
use confique::Error;
use process1::manager::DeserializeWith;
use process1::manager::ExecCommand;

pub(super) struct ServiceConfig {
    data: Rc<RefCell<ServiceConfigData>>,
}

impl ServiceConfig {
    pub(super) fn new() -> Self {
        ServiceConfig {
            data: Rc::new(RefCell::new(ServiceConfigData::default())),
        }
    }

    pub(super) fn load(&self, paths: &Vec<PathBuf>) -> Result<(), Error> {
        let mut builder = ServiceConfigData::builder().env();

        log::debug!("service load path: {:?}", paths);
        // fragment
        for v in paths {
            builder = builder.file(&v);
        }

        *self.data.borrow_mut() = builder.load()?;
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
        self.data.borrow_mut().set_notify_access(v)
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
    pub(super) fn set_notify_access(&mut self, v: NotifyAccess) {
        self.Service.set_notify_access(v)
    }
}

#[derive(Config, Default, Debug)]
pub(super) struct SectionService {
    #[config(deserialize_with = ServiceType::deserialize_with)]
    #[config(default = "simple")]
    pub Type: ServiceType,
    #[config(deserialize_with = Vec::<ExecCommand>::deserialize_with)]
    pub ExecStart: Option<Vec<ExecCommand>>,
    #[config(deserialize_with = Vec::<ExecCommand>::deserialize_with)]
    pub ExecStartPre: Option<Vec<ExecCommand>>,
    #[config(deserialize_with = Vec::<ExecCommand>::deserialize_with)]
    pub ExecStartPost: Option<Vec<ExecCommand>>,
    #[config(deserialize_with = Vec::<ExecCommand>::deserialize_with)]
    pub ExecStop: Option<Vec<ExecCommand>>,
    #[config(deserialize_with = Vec::<ExecCommand>::deserialize_with)]
    pub ExecStopPost: Option<Vec<ExecCommand>>,
    #[config(deserialize_with = Vec::<ExecCommand>::deserialize_with)]
    pub ExecReload: Option<Vec<ExecCommand>>,
    #[config(deserialize_with = Vec::<ExecCommand>::deserialize_with)]
    pub ExecCondition: Option<Vec<ExecCommand>>,
    #[config(deserialize_with = Vec::<String>::deserialize_with)]
    pub Sockets: Option<Vec<String>>,
    pub WatchdogUSec: Option<u64>,
    pub PIDFile: Option<String>,
    #[config(default = false)]
    pub RemainAfterExit: bool,
    pub NotifyAccess: Option<NotifyAccess>,
    #[config(deserialize_with = Vec::<String>::deserialize_with)]
    pub Environment: Option<Vec<String>>,
}

impl SectionService {
    pub(super) fn set_notify_access(&mut self, v: NotifyAccess) {
        self.NotifyAccess = Some(v);
    }
}

impl ServiceConfigData {
    pub fn get_exec_cmds(&self, cmd_type: ServiceCommand) -> Option<Vec<ExecCommand>> {
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
    use crate::service_config::ServiceConfig;
    use tests::get_project_root;

    #[test]
    fn test_service_parse() {
        let mut file_path = get_project_root().unwrap();
        file_path.push("test_units/config.service.toml");
        let paths = vec![file_path];
        let config = ServiceConfig::new();

        let result = config.load(&paths);

        println!("service data: {:?}", config.config_data());

        assert!(result.is_ok());
    }
}
