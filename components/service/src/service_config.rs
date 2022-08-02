#![allow(non_snake_case)]
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use super::service_base::ServiceType;
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
        self.data
            .borrow()
            .Service
            .Type
            .map_or(ServiceType::Simple, |e| e)
    }
}

#[derive(Config, Default, Debug)]
pub(super) struct ServiceConfigData {
    #[config(nested)]
    pub Service: SectionService,
}

#[derive(Config, Default, Debug)]
pub(super) struct SectionService {
    pub Type: Option<ServiceType>,
    pub BusName: Option<String>,
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
    pub Sockets: Option<String>,
    // #[config(deserialize_with = Vec::<ExecCommand>::deserialize_with)]
    // pub Restart: Option<Vec<ExecCommand>>,
    pub RestrictRealtime: Option<String>,
    pub RebootArgument: Option<String>,
    pub OOMScoreAdjust: Option<String>,
    pub RestartSec: Option<u64>,
    pub WatchdogUSec: Option<u64>,
    pub Slice: Option<String>,
    pub MemoryLimit: Option<u64>,
    pub MemoryLow: Option<u64>,
    pub MemoryMin: Option<u64>,
    pub MemoryMax: Option<u64>,
    pub MemoryHigh: Option<u64>,
    pub MemorySwapMax: Option<u64>,
    pub PIDFile: Option<String>,
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
            ServiceCommand::CommandMax => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        env,
        ffi::OsString,
        fs::read_dir,
        io::{self, ErrorKind},
        path::PathBuf,
    };

    use crate::service_config::ServiceConfig;

    fn get_project_root() -> io::Result<PathBuf> {
        let path = env::current_dir()?;
        let mut path_ancestors = path.as_path().ancestors();

        while let Some(p) = path_ancestors.next() {
            let has_cargo = read_dir(p)?
                .into_iter()
                .any(|p| p.unwrap().file_name() == OsString::from("Cargo.lock"));
            if has_cargo {
                return Ok(PathBuf::from(p));
            }
        }
        Err(io::Error::new(
            ErrorKind::NotFound,
            "Ran out of places to find Cargo.toml",
        ))
    }

    #[test]
    fn test_service_parse() {
        let mut file_path = get_project_root().unwrap();
        file_path.push("libutils/examples/config.service.toml");
        let mut paths = Vec::new();
        paths.push(file_path);

        let config = ServiceConfig::new();

        let result = config.load(&paths);

        println!("{:?}", result);
    }
}
