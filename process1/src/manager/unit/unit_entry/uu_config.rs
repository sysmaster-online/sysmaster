use crate::manager::data::{JobMode, UnitConfigItem};
use crate::null_str;

#[derive(Debug)]
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
