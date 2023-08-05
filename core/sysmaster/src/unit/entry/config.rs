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
use super::base::UeBase;
use crate::unit::rentry::{UeConfigInstall, UeConfigUnit};
use crate::unit::util::UnitFile;
use confique::{Config, FileFormat, Partial};
use core::error::*;
use core::rel::ReStation;
use core::serialize::DeserializeWith;
use serde::{Deserialize, Deserializer, Serialize};
use std::cell::RefCell;
use std::rc::Rc;

pub(crate) struct UeConfig {
    // associated objects
    base: Rc<UeBase>,

    // owned objects
    data: Rc<RefCell<UeConfigData>>,
}

#[allow(missing_docs)]
#[derive(Clone, Debug, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnitEmergencyAction {
    #[serde(alias = "none")]
    None,
    #[serde(alias = "reboot")]
    Reboot,
    #[serde(alias = "reboot-force")]
    RebootForce,
    #[serde(alias = "reboot-immediate")]
    RebootImmediate,
    #[serde(alias = "poweroff")]
    Poweroff,
    #[serde(alias = "poweroff-force")]
    PoweroffForce,
    #[serde(alias = "poweroff-immediate")]
    PoweroffImmediate,
    #[serde(alias = "exit")]
    Exit,
    #[serde(alias = "exit-force")]
    ExitForce,
}

impl Default for UnitEmergencyAction {
    fn default() -> Self {
        Self::None
    }
}

impl DeserializeWith for UnitEmergencyAction {
    type Item = Self;
    fn deserialize_with<'de, D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(de)?;

        match s.as_ref() {
            "none" => Ok(UnitEmergencyAction::None),
            "reboot" => Ok(UnitEmergencyAction::Reboot),
            "reboot-force" => Ok(UnitEmergencyAction::RebootForce),
            "reboot-immediate" => Ok(UnitEmergencyAction::RebootImmediate),
            "poweroff" => Ok(UnitEmergencyAction::Poweroff),
            "poweroff-force" => Ok(UnitEmergencyAction::PoweroffForce),
            "poweroff-immediate" => Ok(UnitEmergencyAction::PoweroffImmediate),
            "exit" => Ok(UnitEmergencyAction::Exit),
            "exit-force" => Ok(UnitEmergencyAction::ExitForce),
            &_ => Ok(UnitEmergencyAction::None),
        }
    }
}

impl From<String> for UnitEmergencyAction {
    fn from(action: String) -> Self {
        match action.as_ref() {
            "none" => UnitEmergencyAction::None,
            "reboot" => UnitEmergencyAction::Reboot,
            "reboot-force" => UnitEmergencyAction::RebootForce,
            "reboot-immediate" => UnitEmergencyAction::RebootImmediate,
            "poweroff" => UnitEmergencyAction::Poweroff,
            "poweroff-force" => UnitEmergencyAction::PoweroffForce,
            "poweroff-immediate" => UnitEmergencyAction::PoweroffImmediate,
            "exit" => UnitEmergencyAction::Exit,
            "exit-force" => UnitEmergencyAction::ExitForce,
            _ => UnitEmergencyAction::None,
        }
    }
}

impl ReStation for UeConfig {
    // no input, no compensate

    // data
    fn db_map(&self, reload: bool) {
        if reload {
            return;
        }
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

    pub(super) fn load_fragment_and_dropin(&self, files: &UnitFile, name: &str) -> Result<()> {
        type ConfigPartial = <UeConfigData as Config>::Partial;
        let mut partial: ConfigPartial = Partial::from_env().context(ConfiqueSnafu)?;
        /* The first config wins, so add default values at last. */
        let unit_conf_frag = files.get_unit_id_fragment_pathbuf(name);
        if unit_conf_frag.is_empty() {
            return Err(format!("{} doesn't have corresponding config file", name).into());
        }
        // fragment
        for path in unit_conf_frag {
            if !path.exists() {
                return Err(format!("Config file {:?} of {} doesn't exist", path, name).into());
            }
            partial = match confique::File::with_format(&path, FileFormat::Toml).load() {
                Err(e) => {
                    log::error!("Failed to load {:?}: {}, skipping", path, e);
                    continue;
                }
                Ok(v) => partial.with_fallback(v),
            };
        }
        partial = partial.with_fallback(ConfigPartial::default_values());
        let mut configer = UeConfigData::from_partial(partial).context(ConfiqueSnafu)?;

        // dropin
        for v in files.get_unit_wants_symlink_units(name) {
            configer.Unit.Wants.push(v.to_string_lossy().to_string());
            configer.Unit.After.push(v.to_string_lossy().to_string());
        }

        for v in files.get_unit_requires_symlink_units(name) {
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
    use libtests::get_project_root;

    use crate::unit::entry::config::UeConfigData;
    #[test]
    fn test_unit_parse() {
        let mut file_path = get_project_root().unwrap();
        file_path.push("test_units/config.service");

        let mut builder = UeConfigData::builder().env();
        builder = builder.file(&file_path);

        let result = builder.load();

        println!("{:?}", result);
    }
}
