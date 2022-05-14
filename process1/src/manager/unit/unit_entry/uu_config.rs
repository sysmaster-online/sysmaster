use serde::{Serialize, Deserialize};

use crate::null_str;

use std::cell::RefCell;
use std::fmt::{Display, Formatter};
use config_proc_macro::ConfigParseM;
use std::io::{Error as IoError};
use utils::config_parser::{ConfigParse, toml_str_parse};
use utils::unit_conf::{Confs};
use crate::manager::{data::UnitRelations, unit::unit_base::JobMode};

fn default_null_str() -> String{
   null_str!("")
}
fn default_false()->bool{
    false
}
#[derive(Serialize, Deserialize,ConfigParseM)]
#[serdeName("Unit")]
#[serde(rename_all = "camelCase")]
pub (crate) struct UeConfigUnit{
    name: String,
    desc: String,
    #[serde(default = "default_null_str")]
    documentation: String,
    #[serde(default = "default_false")]
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
    #[serde(default = "default_null_str")]
    before: String,
    #[serde(default = "default_null_str")]
    after: String,
}




#[derive(Serialize, Deserialize,ConfigParseM)]
#[serdeName("Install")]
#[serde(rename_all = "camelCase")]
pub (crate) struct UeConfigInstall{
    alias:String,
    wanted_by:String,
    required_by:String,
    also:String,
    default_instance:String,
    install_alias: String,
    install_also: String,
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
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
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
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result<(),Box<dyn std::error::Error>> {
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
            install_conf:RefCell::new(None),
        }
    }

    pub (super) fn set_unitconf(&self,config_unit:UeConfigUnit){
        let _tmp = Some(config_unit);
        self.unit_conf.replace(_tmp);
    }

    pub (super)fn set_installconf(&self,config_install:UeConfigInstall){
        let _tmp = Some(config_install);
        self.install_conf.replace(_tmp);
    }

    pub (super) fn get_install_deps(&self,conf_option: &InstallConfOption) -> InstallConfOption{
        
    }

    pub (super) fn get_unit_deps(&self, conf_option: & UnitConfOption ) -> UnitConfOption{
    }

    pub(super) fn get(&self, item: &UnitConfigItem) -> UnitConfigItem {
        match item {
            UnitConfigItem::UcItemName(_) => {
                UnitConfigItem::UcItemName(self.unit_conf.borrow().as_ref().unwrap().get_name())
            }
            UnitConfigItem::UcItemDesc(_) => UnitConfigItem::UcItemDesc(
                self.unit_conf.borrow().as_ref().map_or_else(||null_str!(""),|_c|_c.get_desc())
            ),
            UnitConfigItem::UcItemDoc(_) => UnitConfigItem::UcItemDoc(self.unit_conf.borrow().as_ref().map_or_else(||null_str!(""),|_c|_c.get_documentation())),
            UnitConfigItem::UcItemInsAlias(_) => {
                UnitConfigItem::UcItemInsAlias(self.install_conf.borrow().as_ref().map_or_else(||null_str!(""),|_c|_c.get_install_alias()))
            }
            UnitConfigItem::UcItemInsAlso(_) => {
                UnitConfigItem::UcItemInsAlso(self.install_conf.borrow().as_ref().map_or_else(||null_str!(""),|_ic|_ic.get_install_also()))
            }
            UnitConfigItem::UcItemInsDefIns(_) => {
                UnitConfigItem::UcItemInsDefIns(self.install_conf.borrow().as_ref().map_or_else(||null_str!(""),|_ic|_ic.get_install_default_install()))
            }
            UnitConfigItem::UcItemAllowIsolate(_) => {
                UnitConfigItem::UcItemAllowIsolate(self.unit_conf.borrow().as_ref().map_or_else(||false,|_c|_c.get_allow_isolate()))
            }
            UnitConfigItem::UcItemIgnoreOnIsolate(_) => {
                UnitConfigItem::UcItemIgnoreOnIsolate(self.unit_conf.borrow().as_ref().map_or_else(||false,|_c|_c.get_ignore_on_isolate()))
            }
            UnitConfigItem::UcItemOnSucJobMode(_) => {
                UnitConfigItem::UcItemOnSucJobMode(self.unit_conf.borrow().as_ref().map_or_else(||JobMode::JobFail,|_c|_c.get_on_success_job_mode().into()))
            }
            UnitConfigItem::UcItemOnFailJobMode(_) => {
                UnitConfigItem::UcItemOnFailJobMode(self.unit_conf.borrow().as_ref().map_or_else(||JobMode::JobFail,|_c|_c.get_on_failure_job_mode().into()))
            }
        }
    }
}