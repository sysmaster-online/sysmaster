#![allow(non_snake_case)]
use std::cell::RefCell;
use std::rc::Rc;

use confique::Config;
use std::error::Error as stdError;

use crate::manager::unit::uload_util::UnitFile;
use crate::manager::unit::unit_base::JobMode;
use crate::manager::unit::DeserializeWith;

pub(crate) struct UeConfig {
    data: Rc<RefCell<UeConfigData>>,
}

impl UeConfig {
    pub(crate) fn new() -> Self {
        UeConfig {
            data: Rc::new(RefCell::new(UeConfigData::default())),
        }
    }

    pub(super) fn load_fragment_and_dropin(
        &self,
        files: &UnitFile,
        name: &String,
    ) -> Result<(), Box<dyn stdError>> {
        let mut builder = UeConfigData::builder().env();

        let unit_conf_frag = files.get_unit_id_fragment_pathbuf(name);
        if unit_conf_frag.is_empty() {
            log::error!("config file for {} is not exist", name);
            return Err(format!("config file for {} is not exist", name).into());
        }
        // fragment
        for v in unit_conf_frag {
            if !v.exists() {
                log::error!("config file is not exist");
                return Err(format!("config file is not exist {}", name).into());
            }
            builder = builder.file(&v);
        }

        let mut configer = builder.load()?;

        // dropin
        for v in files.get_unit_id_dropin_wants(name) {
            configer.Unit.Wants.push(v.to_string_lossy().to_string());
            configer.Unit.After.push(v.to_string_lossy().to_string());
        }

        for v in files.get_unit_id_dropin_requires(name) {
            configer.Unit.Requires.push(v.to_string_lossy().to_string());
            configer.Unit.After.push(v.to_string_lossy().to_string());
        }

        *self.data.borrow_mut() = configer;

        Ok(())
    }

    pub(crate) fn config_data(&self) -> Rc<RefCell<UeConfigData>> {
        self.data.clone()
    }
}

#[derive(Config, Default, Debug)]
pub(crate) struct UeConfigData {
    #[config(nested)]
    pub Unit: UeConfigUnit,
    #[config(nested)]
    pub Install: UeConfigInstall,
}

#[derive(Config, Default, Debug)]
pub(crate) struct UeConfigUnit {
    #[config(default = "")]
    pub Description: String,
    #[config(default = "")]
    pub Documentation: String,
    #[config(default = false)]
    pub AllowIsolate: bool,
    #[config(default = false)]
    pub IgnoreOnIsolate: bool,
    #[config(default = true)]
    pub DefaultDependencies: bool,
    // #[config(deserialize_with = JobMode::deserialize_with)]
    // #[config(default = "replace")]
    // pub on_success_job_mode: JobMode,
    #[config(deserialize_with = JobMode::deserialize_with)]
    #[config(default = "replace")]
    pub OnFailureJobMode: JobMode,
    #[config(deserialize_with = Vec::<String>::deserialize_with)]
    #[config(default = "")]
    pub Wants: Vec<String>,
    #[config(deserialize_with = Vec::<String>::deserialize_with)]
    #[config(default = "")]
    pub Requires: Vec<String>,
    #[config(deserialize_with = Vec::<String>::deserialize_with)]
    #[config(default = "")]
    pub Before: Vec<String>,
    #[config(deserialize_with = Vec::<String>::deserialize_with)]
    #[config(default = "")]
    pub After: Vec<String>,
}

#[derive(Config, Default, Debug)]
pub(crate) struct UeConfigInstall {
    #[config(default = "")]
    pub Alias: String,
    #[config(deserialize_with = Vec::<String>::deserialize_with)]
    #[config(default = "")]
    pub WantedBy: Vec<String>,
    #[config(deserialize_with = Vec::<String>::deserialize_with)]
    #[config(default = "")]
    pub RequiredBy: Vec<String>,
    #[config(default = "")]
    pub Also: String,
    #[config(default = "")]
    pub DefaultInstance: String,
    // #[config(default = "")]
    // pub install_alias: String,
    // #[config(default = "")]
    // pub install_also: String,
    // #[config(default = "")]
    // pub install_default_install: String,
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

    use confique::Config;

    use crate::manager::unit::unit_entry::uu_config::UeConfigData;

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

        let mut builder = UeConfigData::builder().env();
        builder = builder.file(&file_path);

        let result = builder.load();

        println!("{:?}", result);
    }
}
