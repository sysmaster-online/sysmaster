use confique::{Config, Error};

use crate::manager::unit::uload_util::UnitFile;
use crate::manager::unit::unit_base::JobMode;
use crate::manager::unit::DeserializeWith;

#[derive(Config, Default)]
pub(crate) struct UeConfig {
    #[config(nested)]
    pub Unit: UeConfigUnit,
    #[config(nested)]
    pub Install: UeConfigInstall,
}

#[derive(Config, Default)]
pub(crate) struct UeConfigUnit {
    #[config(default = "")]
    pub desc: String,
    #[config(default = "")]
    pub documentation: String,
    #[config(default = false)]
    pub allow_isolate: bool,
    #[config(default = false)]
    pub ignore_on_isolate: bool,
    #[config(deserialize_with = JobMode::deserialize_with)]
    #[config(default = "replace")]
    pub on_success_job_mode: JobMode,
    #[config(deserialize_with = JobMode::deserialize_with)]
    #[config(default = "replace")]
    pub on_failure_job_mode: JobMode,
    #[config(deserialize_with = Vec::<String>::deserialize_with)]
    #[config(default = "")]
    pub wants: Vec<String>,
    #[config(deserialize_with = Vec::<String>::deserialize_with)]
    #[config(default = "")]
    pub requires: Vec<String>,
    #[config(deserialize_with = Vec::<String>::deserialize_with)]
    #[config(default = "")]
    pub before: Vec<String>,
    #[config(deserialize_with = Vec::<String>::deserialize_with)]
    #[config(default = "")]
    pub after: Vec<String>,
}

#[derive(Config, Default)]
pub(crate) struct UeConfigInstall {
    #[config(default = "")]
    pub alias: String,
    #[config(deserialize_with = Vec::<String>::deserialize_with)]
    #[config(default = "")]
    pub wanted_by: Vec<String>,
    #[config(deserialize_with = Vec::<String>::deserialize_with)]
    #[config(default = "")]
    pub required_by: Vec<String>,
    #[config(default = "")]
    pub also: String,
    #[config(default = "")]
    pub default_instance: String,
    #[config(default = "")]
    pub install_alias: String,
    #[config(default = "")]
    pub install_also: String,
    #[config(default = "")]
    pub install_default_install: String,
}

impl UeConfig {
    pub fn load_fragment_and_dropin(
        &self,
        files: &UnitFile,
        name: &String,
    ) -> Result<UeConfig, Error> {
        let mut builder = UeConfig::builder().env();

        // fragment
        for v in files.get_unit_id_fragment_pathbuf(name) {
            builder = builder.file(&v);
        }

        let mut configer = builder.load()?;

        // dropin
        for v in files.get_unit_id_dropin_wants(name) {
            configer.Unit.wants.push(v.to_string_lossy().to_string());
            configer.Unit.after.push(v.to_string_lossy().to_string());
        }

        for v in files.get_unit_id_dropin_requires(name) {
            configer.Unit.requires.push(v.to_string_lossy().to_string());
            configer.Unit.after.push(v.to_string_lossy().to_string());
        }
        Ok(configer)
    }
}
