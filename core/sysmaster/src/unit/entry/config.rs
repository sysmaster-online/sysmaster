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
use basic::unit_name::unit_name_to_instance;
use core::error::*;
use core::rel::ReStation;
use core::serialize::DeserializeWith;
use core::specifier::{
    unit_string_specifier_escape, unit_strings_specifier_escape, UnitSpecifierData, LONG_LINE_MAX,
    PATH_MAX, UNIT_NAME_MAX,
};
use serde::{Deserialize, Deserializer, Serialize};
use std::cell::RefCell;
use std::rc::Rc;
use unit_parser::prelude::{UnitConfig, UnitEntry};

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

impl UnitEntry for UnitEmergencyAction {
    type Error = basic::error::Error;

    fn parse_from_str<S: AsRef<str>>(input: S) -> std::result::Result<Self, Self::Error> {
        match input.as_ref() {
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
        let unit_conf_frag = files.get_unit_id_fragment_pathbuf(name);
        if unit_conf_frag.is_empty() {
            return Err(format!("{} doesn't have corresponding config file", name).into());
        }

        let mut configer = match UeConfigData::load_config(unit_conf_frag, name) {
            Ok(v) => v,
            Err(e) => {
                log::error!("Invalid Configuration: {}", e);
                return Err(Error::ConfigureError {
                    msg: format!("Invalid Configuration: {}", e),
                });
            }
        };

        // dropin
        for v in files.get_unit_wants_symlink_units(name) {
            configer.Unit.Wants.push(v.to_string_lossy().to_string());
            configer.Unit.After.push(v.to_string_lossy().to_string());
        }

        for v in files.get_unit_requires_symlink_units(name) {
            configer.Unit.Requires.push(v.to_string_lossy().to_string());
            configer.Unit.After.push(v.to_string_lossy().to_string());
        }

        let mut unit_specifier_data = UnitSpecifierData::new();
        unit_specifier_data.instance = unit_name_to_instance(&self.base.id());
        configer.update_with_specifier_escape(&unit_specifier_data);

        *self.data.borrow_mut() = configer;
        self.db_update();

        Ok(())
    }

    pub(crate) fn config_data(&self) -> Rc<RefCell<UeConfigData>> {
        self.data.clone()
    }

    pub(crate) fn set_property(&self, key: &str, value: &str) -> Result<()> {
        let ret = self.data.borrow_mut().set_property(key, value);
        self.db_update();
        ret
    }
}

#[derive(UnitConfig, Default, Debug)]
pub(crate) struct UeConfigData {
    #[section(default)]
    pub Unit: UeConfigUnit,
    #[section(default)]
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

    pub(self) fn update_with_specifier_escape(&mut self, unit_specifier_data: &UnitSpecifierData) {
        if let Ok(ret) =
            unit_string_specifier_escape(&self.Unit.Description, LONG_LINE_MAX, unit_specifier_data)
        {
            self.Unit.Description = ret;
        }
        if let Ok(ret) = unit_string_specifier_escape(
            &self.Unit.ConditionPathExists,
            PATH_MAX - 1,
            unit_specifier_data,
        ) {
            self.Unit.ConditionPathExists = ret;
        }
        if let Ok(ret) = unit_string_specifier_escape(
            &self.Unit.ConditionFileNotEmpty,
            PATH_MAX - 1,
            unit_specifier_data,
        ) {
            self.Unit.ConditionFileNotEmpty = ret;
        }
        if let Ok(ret) =
            unit_strings_specifier_escape(&self.Unit.BindsTo, UNIT_NAME_MAX, unit_specifier_data)
        {
            self.Unit.BindsTo = ret;
        }
        if let Ok(ret) =
            unit_strings_specifier_escape(&self.Unit.After, UNIT_NAME_MAX, unit_specifier_data)
        {
            self.Unit.After = ret;
        }
    }

    pub(self) fn set_property(&mut self, key: &str, value: &str) -> Result<()> {
        let mut ret = self.Unit.set_property(key, value);
        if let Err(Error::NotFound { what: _ }) = ret {
            ret = self.Install.set_property(key, value);
        }
        ret
    }
}

#[cfg(test)]
mod tests {
    use crate::manager::RELI_HISTORY_MAX_DBS;
    use crate::unit::entry::base::UeBase;
    use crate::unit::entry::config::UeConfig;
    use crate::unit::rentry::UnitRe;
    use basic::unit_name::unit_name_to_instance;
    use core::rel::{ReliConf, Reliability};
    use core::specifier::UnitSpecifierData;
    use core::unit::UnitType;
    use libtests::get_project_root;
    use std::rc::Rc;
    use unit_parser::prelude::UnitConfig;

    use crate::unit::entry::config::UeConfigData;
    #[test]
    fn test_unit_parse() {
        let mut file_path = get_project_root().unwrap();
        file_path.push("tests/test_units");
        println!("{:?}", file_path);
        let result = UeConfigData::load_config(vec![file_path], "config.service").unwrap();

        println!("{:?}", result);
    }

    #[test]
    fn test_unit_specifier_escape() {
        let reli = Rc::new(Reliability::new(
            ReliConf::new().set_max_dbs(RELI_HISTORY_MAX_DBS),
        ));
        let rentry = Rc::new(UnitRe::new(&reli));
        let base = Rc::new(UeBase::new(&rentry, String::new(), UnitType::UnitService));
        let config = UeConfig::new(&base);

        /* Description="%i service"
         * BindsTo="a.service %i.service"
         * After="b.service %i.service;%i.service %I.service"
         * ConditionPathExists="/%i %I"
         */
        config.data.borrow_mut().Unit.Description = "%i service".to_string();
        config.data.borrow_mut().Unit.BindsTo = vec!["a.service %i.service".to_string()];
        config.data.borrow_mut().Unit.After = vec![
            "b.service %i.service".to_string(),
            "%i.service %I.service".to_string(),
        ];
        config.data.borrow_mut().Unit.ConditionPathExists = "/%i %I".to_string();

        // Construct instance="Hal\\xc3\\xb6-chen"
        let mut unit_specifier_data = UnitSpecifierData::new();
        unit_specifier_data.instance = unit_name_to_instance("config@Hal\\xc3\\xb6-chen.service");

        config
            .data
            .borrow_mut()
            .update_with_specifier_escape(&unit_specifier_data);

        assert_eq!(
            config.data.borrow().Unit.Description,
            "Hal\\xc3\\xb6-chen service".to_string()
        );
        assert_eq!(
            config.data.borrow().Unit.BindsTo,
            vec!["a.service Hal\\xc3\\xb6-chen.service".to_string()]
        );
        assert_eq!(
            config.data.borrow().Unit.After,
            vec![
                "b.service Hal\\xc3\\xb6-chen.service".to_string(),
                "Hal\\xc3\\xb6-chen.service Halö/chen.service".to_string()
            ]
        );
        assert_eq!(
            config.data.borrow().Unit.ConditionPathExists,
            "/Hal\\xc3\\xb6-chen Halö/chen".to_string()
        );
    }
}
