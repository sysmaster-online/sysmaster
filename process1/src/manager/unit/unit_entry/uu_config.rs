use serde::{Deserialize, Serialize};

use crate::manager::{data::UnitRelations, unit::unit_base::JobMode};
use crate::null_str;
use proc_macro_utils::ConfigParseM;
use std::cell::RefCell;
use std::fmt::{Display, Formatter};
use std::io::{Error as IoError, ErrorKind};
use utils::config_parser::{toml_str_parse, ConfigParse};

fn default_null_str() -> String {
    null_str!("")
}
fn default_false() -> bool {
    false
}
#[derive(Serialize, Deserialize, ConfigParseM)]
#[serdeName("Unit")]
#[serde(rename_all = "PascalCase")]
pub(crate) struct UeConfigUnit {
    #[serde(default = "default_null_str", alias = "Name")]
    name: String,
    #[serde(default = "default_null_str", alias = "Description")]
    desc: String,
    #[serde(default = "default_null_str", alias = "Documentation")]
    documentation: String,
    #[serde(default = "default_false", alias = "AllowIsoLate")]
    allow_isolate: bool,
    #[serde(default = "default_false")]
    ignore_on_isolate: bool,
    #[serde(default = "default_null_str")]
    on_success_job_mode: String,
    #[serde(default = "default_null_str")]
    on_failure_job_mode: String,
    #[serde(default = "default_null_str")]
    wants: String,
    #[serde(default = "default_null_str")]
    requires: String,
    #[serde(default = "default_null_str", alias = "Before")]
    before: String,
    #[serde(default = "default_null_str")]
    after: String,
}

#[derive(Serialize, Deserialize, ConfigParseM)]
#[serdeName("Install")]
#[serde(rename_all = "PascalCase")]
pub(crate) struct UeConfigInstall {
    #[serde(default = "default_null_str")]
    alias: String,
    #[serde(default = "default_null_str")]
    wanted_by: String,
    #[serde(default = "default_null_str")]
    required_by: String,
    #[serde(default = "default_null_str")]
    also: String,
    #[serde(default = "default_null_str")]
    default_instance: String,
    #[serde(default = "default_null_str")]
    install_alias: String,
    #[serde(default = "default_null_str")]
    install_also: String,
    #[serde(default = "default_null_str")]
    install_default_install: String,
}

pub(in crate::manager::unit) enum UnitConfigItem {
    UcItemName(String),
    UcItemDesc(String),
    UcItemDoc(String),
    UcItemInsAlias(String),
    UcItemInsAlso(String),
    UcItemInsDefIns(String),
    UcItemAllowIsolate(bool),
    UcItemIgnoreOnIsolate(bool),
    UcItemOnSucJobMode(JobMode),
    UcItemOnFailJobMode(JobMode),
    UcItemRelation(UnitRelations, String),
    UcItemWantedBy(String),
    UcItemRequiredBy(String),
}

pub(super) enum UnitConfOption {
    Desc,
    Documentation,
    Relation(UnitRelations),
    AllowIsolate,
    IgnoreOnIolate,
    OnSucessJobMode(JobMode),
    OnFailureJobMode(JobMode),
}

impl Display for UnitConfOption {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            UnitConfOption::Desc => write!(f, "Description"),
            UnitConfOption::Documentation => write!(f, "Documentation"),
            UnitConfOption::Relation(relation) => write!(f, "{}", relation),
            UnitConfOption::AllowIsolate => write!(f, "AllowIsolate"),
            UnitConfOption::IgnoreOnIolate => write!(f, "IgnoreOnIolate"),
            UnitConfOption::OnSucessJobMode(_) => write!(f, "OnSucessJobMode"),
            UnitConfOption::OnFailureJobMode(_) => write!(f, "OnFailureJobMode"),
        }
    }
}

impl From<UnitConfOption> for String {
    fn from(unit_conf_opt: UnitConfOption) -> Self {
        match unit_conf_opt {
            UnitConfOption::Desc => "Desc".into(),
            UnitConfOption::Documentation => "Documentation".into(),
            UnitConfOption::Relation(relation) => relation.into(),
            UnitConfOption::AllowIsolate => "AllowIsolate".into(),
            UnitConfOption::IgnoreOnIolate => "IgnoreOnIolate".into(),
            UnitConfOption::OnSucessJobMode(_) => "OnSucessJobMode".into(),
            UnitConfOption::OnFailureJobMode(_) => "OnFailureJobMode".into(),
        }
    }
}

pub(super) enum InstallConfOption {
    Alias,
    WantedBy,
    RequiredBy,
    Also,
    DefaultInstance,
}

impl Display for InstallConfOption {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            InstallConfOption::Alias => write!(fmt, "Alias"),
            InstallConfOption::WantedBy => write!(fmt, "WantedBy"),
            InstallConfOption::RequiredBy => write!(fmt, "RequiredBy"),
            InstallConfOption::Also => write!(fmt, "Also"),
            InstallConfOption::DefaultInstance => write!(fmt, "DefaultInstance"),
        }
    }
}

impl From<InstallConfOption> for String {
    fn from(install_conf_opt: InstallConfOption) -> Self {
        match install_conf_opt {
            InstallConfOption::Alias => "Alias".into(),
            InstallConfOption::WantedBy => "WantedBy".into(),
            InstallConfOption::RequiredBy => "RequiredBy".into(),
            InstallConfOption::Also => "Also".into(),
            InstallConfOption::DefaultInstance => "DefaultInstance".into(),
        }
    }
}

pub(super) struct UeConfig {
    unit_conf: RefCell<Option<UeConfigUnit>>,
    install_conf: RefCell<Option<UeConfigInstall>>,
}

impl UeConfig {
    pub(super) fn new() -> UeConfig {
        UeConfig {
            unit_conf: RefCell::new(None),
            install_conf: RefCell::new(None),
        }
    }

    pub(super) fn set_unitconf(&self, config_unit: UeConfigUnit) {
        let _tmp = Some(config_unit);
        self.unit_conf.replace(_tmp);
    }

    pub(super) fn set_installconf(&self, config_install: UeConfigInstall) {
        let _tmp = Some(config_install);
        self.install_conf.replace(_tmp);
    }

    pub(super) fn get_install_conf_value(&self, conf_key: InstallConfOption) -> UnitConfigItem {
        match conf_key {
            InstallConfOption::Alias => todo!(),
            InstallConfOption::WantedBy => UnitConfigItem::UcItemRelation(
                UnitRelations::UnitWantsBy,
                self.install_conf
                    .borrow()
                    .as_ref()
                    .map_or_else(|| null_str!(""), |_c| _c.get_wanted_by()),
            ),
            InstallConfOption::RequiredBy => UnitConfigItem::UcItemRelation(
                UnitRelations::UnitRequiresBy,
                self.install_conf
                    .borrow()
                    .as_ref()
                    .map_or_else(|| null_str!(""), |_c| _c.get_required_by()),
            ),
            InstallConfOption::Also => todo!(),
            InstallConfOption::DefaultInstance => todo!(),
        }
    }

    pub(super) fn get_unit_conf_value(&self, conf_key: UnitConfOption) -> UnitConfigItem {
        match conf_key {
            UnitConfOption::Desc => todo!(),
            UnitConfOption::Documentation => todo!(),
            UnitConfOption::Relation(v) => match v {
                UnitRelations::UnitRequires => UnitConfigItem::UcItemRelation(
                    UnitRelations::UnitRequires,
                    self.unit_conf
                        .borrow()
                        .as_ref()
                        .map_or_else(|| null_str!(""), |_c| _c.get_requires()),
                ),
                UnitRelations::UnitRequisite => todo!(),
                UnitRelations::UnitWants => UnitConfigItem::UcItemRelation(
                    UnitRelations::UnitRequires,
                    self.unit_conf
                        .borrow()
                        .as_ref()
                        .map_or_else(|| null_str!(""), |_c| _c.get_wants()),
                ),
                UnitRelations::UnitBindsTo => todo!(),
                UnitRelations::UnitPartOf => todo!(),
                UnitRelations::UnitUpHolds => todo!(),
                UnitRelations::UnitRequiresBy => todo!(),
                UnitRelations::UnitRequisiteOf => todo!(),
                UnitRelations::UnitWantsBy => todo!(),
                UnitRelations::UnitBoundBy => todo!(),
                UnitRelations::UnitConsistsOf => todo!(),
                UnitRelations::UnitUpHeldBy => todo!(),
                UnitRelations::UnitConflicts => todo!(),
                UnitRelations::UnitConflictedBy => todo!(),
                UnitRelations::UnitBefore => UnitConfigItem::UcItemRelation(
                    UnitRelations::UnitBefore,
                    self.unit_conf
                        .borrow()
                        .as_ref()
                        .map_or_else(|| null_str!(""), |_c| _c.get_before()),
                ),
                UnitRelations::UnitAfter => UnitConfigItem::UcItemRelation(
                    UnitRelations::UnitAfter,
                    self.unit_conf
                        .borrow()
                        .as_ref()
                        .map_or_else(|| null_str!(""), |_c| _c.get_after()),
                ),
                UnitRelations::UnitOnSuccess => todo!(),
                UnitRelations::UnitOnSuccessOf => todo!(),
                UnitRelations::UnitOnFailure => todo!(),
                UnitRelations::UnitonFailureOf => todo!(),
                UnitRelations::UnitTriggers => todo!(),
                UnitRelations::UnitTriggeredBy => todo!(),
                UnitRelations::UnitPropagatesReloadTo => todo!(),
                UnitRelations::UnitReloadPropagatedFrom => todo!(),
                UnitRelations::UnitPropagatesStopTo => todo!(),
                UnitRelations::UnitStopPropagatedFrom => todo!(),
                UnitRelations::UnitJoinsNameSpaceOf => todo!(),
                UnitRelations::UnitReferences => todo!(),
                UnitRelations::UnitReferencedBy => todo!(),
                UnitRelations::UnitInSlice => todo!(),
                UnitRelations::UnitSliceOf => todo!(),
            },
            UnitConfOption::AllowIsolate => todo!(),
            UnitConfOption::IgnoreOnIolate => todo!(),
            UnitConfOption::OnSucessJobMode(_) => todo!(),
            UnitConfOption::OnFailureJobMode(_) => todo!(),
        }
    }

    pub(super) fn get(&self, item: &UnitConfigItem) -> UnitConfigItem {
        match item {
            UnitConfigItem::UcItemName(_) => {
                UnitConfigItem::UcItemName(self.unit_conf.borrow().as_ref().unwrap().get_name())
            }
            UnitConfigItem::UcItemDesc(_) => UnitConfigItem::UcItemDesc(
                self.unit_conf
                    .borrow()
                    .as_ref()
                    .map_or_else(|| null_str!(""), |_c| _c.get_desc()),
            ),
            UnitConfigItem::UcItemDoc(_) => UnitConfigItem::UcItemDoc(
                self.unit_conf
                    .borrow()
                    .as_ref()
                    .map_or_else(|| null_str!(""), |_c| _c.get_documentation()),
            ),
            UnitConfigItem::UcItemInsAlias(_) => UnitConfigItem::UcItemInsAlias(
                self.install_conf
                    .borrow()
                    .as_ref()
                    .map_or_else(|| null_str!(""), |_c| _c.get_install_alias()),
            ),
            UnitConfigItem::UcItemInsAlso(_) => UnitConfigItem::UcItemInsAlso(
                self.install_conf
                    .borrow()
                    .as_ref()
                    .map_or_else(|| null_str!(""), |_ic| _ic.get_install_also()),
            ),
            UnitConfigItem::UcItemInsDefIns(_) => UnitConfigItem::UcItemInsDefIns(
                self.install_conf
                    .borrow()
                    .as_ref()
                    .map_or_else(|| null_str!(""), |_ic| _ic.get_install_default_install()),
            ),
            UnitConfigItem::UcItemAllowIsolate(_) => UnitConfigItem::UcItemAllowIsolate(
                self.unit_conf
                    .borrow()
                    .as_ref()
                    .map_or_else(|| false, |_c| _c.get_allow_isolate()),
            ),
            UnitConfigItem::UcItemIgnoreOnIsolate(_) => UnitConfigItem::UcItemIgnoreOnIsolate(
                self.unit_conf
                    .borrow()
                    .as_ref()
                    .map_or_else(|| false, |_c| _c.get_ignore_on_isolate()),
            ),
            UnitConfigItem::UcItemOnSucJobMode(_) => {
                UnitConfigItem::UcItemOnSucJobMode(self.unit_conf.borrow().as_ref().map_or_else(
                    || JobMode::JobFail,
                    |_c| _c.get_on_success_job_mode().into(),
                ))
            }
            UnitConfigItem::UcItemOnFailJobMode(_) => {
                UnitConfigItem::UcItemOnFailJobMode(self.unit_conf.borrow().as_ref().map_or_else(
                    || JobMode::JobFail,
                    |_c| _c.get_on_failure_job_mode().into(),
                ))
            }
            UnitConfigItem::UcItemRelation(_, _) => todo!(),
            UnitConfigItem::UcItemWantedBy(_) => todo!(),
            UnitConfigItem::UcItemRequiredBy(_) => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {

    use utils::logger;

    use super::UeConfigUnit;
    use utils::config_parser::ConfigParse;

    use crate::manager::unit::unit_entry::uu_config::UeConfigInstall;
    #[test]
    fn test_ueconfig_load() {
        logger::init_log_with_console("test", 4);
        let config_str = r####"
[Install]
WantedBy = "dbus.service"

[Service]
ExecCondition = ["/usr/bin/sleep 5"]
ExecStart = ["/usr/bin/echo 'test'"]
ExecStop = ["/usr/bin/kill $MAINPID"]

[Unit]
Description = "CN"
Documentation = "192.168.1.1"
Requires = "c.service"
Wants = "b.service""####;

        let v = utils::config_parser::toml_str_parse(config_str);
        if v.is_err() {
            if let Err(r) = v {
                log::debug!("error {}", r.to_string());
            }
        } else {
            log::debug!("toml str parse sucesssful,{}", v.unwrap().to_string());
        }
        log::debug!("begin test for unit  conf section parse");
        let unit_parser = UeConfigUnit::builder_parser();
        let config_unit = unit_parser.conf_file_parse(&config_str);
        if config_unit.is_ok() {
            log::debug!("parse for unit sucesssful");
            let unit = config_unit.unwrap();
            log::debug!("{},{}", unit.get_wants(), unit.get_requires());
            assert_eq!("b.service", unit.get_wants());
            assert_eq!("c.service", unit.get_requires());
        } else {
            if let Err(r) = config_unit {
                log::debug!("error {}", r.to_string());
                panic!("{}", r.to_string());
            }
        }
        log::debug!("begin test for install conf section parse");
        let install_parser = UeConfigInstall::builder_parser();
        let unit_install = install_parser.conf_file_parse(&config_str);
        let _ret = unit_install.map(|install| {
            log::debug!("parse for unit install sucesssful");
            log::debug!("install conf {}", install.get_wanted_by());
            assert_eq!("dbus.service", install.get_wanted_by());
        });
    }
}
