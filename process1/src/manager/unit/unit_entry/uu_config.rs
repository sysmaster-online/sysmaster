use core::fmt::{Display, Formatter, Result};

use crate::manager::data::{JobMode, UnitConfigItem, UnitRelations};
use crate::null_str;

pub(in crate::manager) enum UnitConfOption {
    Desc,
    Documentation,
    Relation(UnitRelations),
    AllowIsolate,
    IgnoreOnIolate,
    OnSucessJobMode,
    OnFailureJobMode,
}

impl Display for UnitConfOption {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            UnitConfOption::Desc => write!(f, "Description"),
            UnitConfOption::Documentation => write!(f, "Documentation"),
            UnitConfOption::Relation(relation) => write!(f, "{}", relation),
            UnitConfOption::AllowIsolate => write!(f, "AllowIsolate"),
            UnitConfOption::IgnoreOnIolate => write!(f, "IgnoreOnIolate"),
            UnitConfOption::OnSucessJobMode => write!(f, "OnSucessJobMode"),
            UnitConfOption::OnFailureJobMode => write!(f, "OnFailureJobMode"),
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
            UnitConfOption::OnSucessJobMode => "OnSucessJobMode".into(),
            UnitConfOption::OnFailureJobMode => "OnFailureJobMode".into(),
        }
    }
}

pub(in crate::manager) enum InstallConfOption {
    Alias,
    WantedBy,
    RequiredBy,
    Also,
    DefaultInstance,
}

impl Display for InstallConfOption {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result {
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
    name: String,
    desc: String,
    documnetation: String,
    allow_isolate: bool,
    ignore_on_isolate: bool,
    on_success_job_mode: JobMode,
    on_failure_job_mode: JobMode,
}

impl UeConfig {
    pub(super) fn new() -> UeConfig {
        UeConfig {
            name: String::from(""),
            desc: String::from(""),
            documnetation: null_str!(""),
            allow_isolate: false,
            ignore_on_isolate: false,
            on_success_job_mode: JobMode::JobFail,
            on_failure_job_mode: JobMode::JobFail,
        }
    }

    pub(super) fn set(&mut self, item: UnitConfigItem) {
        match item {
            UnitConfigItem::UcItemName(name) => self.name = name,
            UnitConfigItem::UcItemDesc(desc) => self.desc = desc,
            UnitConfigItem::UcItemDoc(doc) => self.documnetation = doc,
            UnitConfigItem::UcItemAllowIsolate(allow) => self.allow_isolate = allow,
            UnitConfigItem::UcItemIgnoreOnIsolate(ignore) => self.ignore_on_isolate = ignore,
            UnitConfigItem::UcItemOnSucJobMode(mode) => self.on_success_job_mode = mode,
            UnitConfigItem::UcItemOnFailJobMode(mode) => self.on_failure_job_mode = mode,
        }
    }

    pub(super) fn get(&self, item: &UnitConfigItem) -> UnitConfigItem {
        match item {
            UnitConfigItem::UcItemName(_) => UnitConfigItem::UcItemName(self.name.clone()),
            UnitConfigItem::UcItemDesc(_) => UnitConfigItem::UcItemDesc(self.desc.clone()),
            UnitConfigItem::UcItemDoc(_) => UnitConfigItem::UcItemDoc(self.documnetation.clone()),
            UnitConfigItem::UcItemAllowIsolate(_) => {
                UnitConfigItem::UcItemAllowIsolate(self.allow_isolate)
            }
            UnitConfigItem::UcItemIgnoreOnIsolate(_) => {
                UnitConfigItem::UcItemIgnoreOnIsolate(self.ignore_on_isolate)
            }
            UnitConfigItem::UcItemOnSucJobMode(_) => {
                UnitConfigItem::UcItemOnSucJobMode(self.on_success_job_mode)
            }
            UnitConfigItem::UcItemOnFailJobMode(_) => {
                UnitConfigItem::UcItemOnFailJobMode(self.on_failure_job_mode)
            }
        }
    }
}
