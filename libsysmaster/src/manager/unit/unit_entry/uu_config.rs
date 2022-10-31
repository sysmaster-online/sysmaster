#![allow(non_snake_case)]
use super::uu_base::UeBase;
use crate::manager::unit::uload_util::UnitFile;
use crate::manager::unit::unit_rentry::{UeConfigInstall, UeConfigUnit};
use crate::reliability::ReStation;
use confique::Config;
use std::cell::RefCell;
use std::error::Error as stdError;
use std::rc::Rc;

pub(crate) struct UeConfig {
    // associated objects
    base: Rc<UeBase>,

    // owned objects
    data: Rc<RefCell<UeConfigData>>,
}

impl ReStation for UeConfig {
    // no input, no compensate

    // data
    fn db_map(&self) {
        if let Some((unit, install)) = self.base.rentry_conf_get() {
            let conf = UeConfigData::new(unit, install);
            *self.data.borrow_mut() = conf;
        }
    }

    fn db_insert(&self) {
        self.base
            .rentry_conf_insert(&self.data.borrow().Unit, &self.data.borrow().Install);
    }

    // reload: no external connections, no entry
}

impl UeConfig {
    pub(super) fn new(baser: &Rc<UeBase>) -> UeConfig {
        let conf = UeConfig {
            base: Rc::clone(baser),
            data: Rc::new(RefCell::new(UeConfigData::default())),
        };
        conf.db_insert();
        conf
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
        self.db_update();

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

// the declaration "pub(self)" is for identification only.
impl UeConfigData {
    pub(self) fn new(unit: UeConfigUnit, install: UeConfigInstall) -> UeConfigData {
        UeConfigData {
            Unit: unit,
            Install: install,
        }
    }
}

#[cfg(test)]
mod tests {
    use confique::Config;
    use tests::get_project_root;

    use crate::manager::unit::unit_entry::uu_config::UeConfigData;
    #[test]
    fn test_service_parse() {
        let mut file_path = get_project_root().unwrap();
        file_path.push("test_units/config.service.toml");

        let mut builder = UeConfigData::builder().env();
        builder = builder.file(&file_path);

        let result = builder.load();

        println!("{:?}", result);
    }
}
