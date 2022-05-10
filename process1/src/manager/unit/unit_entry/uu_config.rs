use crate::manager::unit::unit_base::JobMode;
use crate::null_str;
use std::cell::RefCell;

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

pub(super) struct UeConfig {
    data: RefCell<UeConfigData>,
}

impl UeConfig {
    pub(super) fn new() -> UeConfig {
        UeConfig {
            data: RefCell::new(UeConfigData::new()),
        }
    }

    pub(super) fn set(&self, item: UnitConfigItem) {
        self.data.borrow_mut().set(item)
    }

    pub(super) fn get(&self, item: &UnitConfigItem) -> UnitConfigItem {
        self.data.borrow().get(item)
    }
}

struct UeConfigData {
    name: String,
    desc: String,
    documnetation: String,
    install_alias: String,
    install_also: String,
    install_default_install: String,
    allow_isolate: bool,
    ignore_on_isolate: bool,
    on_success_job_mode: JobMode,
    on_failure_job_mode: JobMode,
}

// the declaration "pub(self)" is for identification only.
impl UeConfigData {
    pub(self) fn new() -> UeConfigData {
        UeConfigData {
            name: String::from(""),
            desc: String::from(""),
            documnetation: null_str!(""),
            install_alias: null_str!(""),
            install_also: null_str!(""),
            install_default_install: null_str!(""),
            allow_isolate: false,
            ignore_on_isolate: false,
            on_success_job_mode: JobMode::JobFail,
            on_failure_job_mode: JobMode::JobFail,
        }
    }

    pub(self) fn set(&mut self, item: UnitConfigItem) {
        match item {
            UnitConfigItem::UcItemName(name) => self.name = name,
            UnitConfigItem::UcItemDesc(desc) => self.desc = desc,
            UnitConfigItem::UcItemDoc(doc) => self.documnetation = doc,
            UnitConfigItem::UcItemInsAlias(alias) => self.install_alias = alias,
            UnitConfigItem::UcItemInsAlso(also) => self.install_also = also,
            UnitConfigItem::UcItemInsDefIns(def_ins) => self.install_default_install = def_ins,
            UnitConfigItem::UcItemAllowIsolate(allow) => self.allow_isolate = allow,
            UnitConfigItem::UcItemIgnoreOnIsolate(ignore) => self.ignore_on_isolate = ignore,
            UnitConfigItem::UcItemOnSucJobMode(mode) => self.on_success_job_mode = mode,
            UnitConfigItem::UcItemOnFailJobMode(mode) => self.on_failure_job_mode = mode,
        }
    }

    pub(self) fn get(&self, item: &UnitConfigItem) -> UnitConfigItem {
        match item {
            UnitConfigItem::UcItemName(_) => UnitConfigItem::UcItemName(self.name.clone()),
            UnitConfigItem::UcItemDesc(_) => UnitConfigItem::UcItemDesc(self.desc.clone()),
            UnitConfigItem::UcItemDoc(_) => UnitConfigItem::UcItemDoc(self.documnetation.clone()),
            UnitConfigItem::UcItemInsAlias(_) => {
                UnitConfigItem::UcItemInsAlias(self.install_alias.clone())
            }
            UnitConfigItem::UcItemInsAlso(_) => {
                UnitConfigItem::UcItemInsAlso(self.install_also.clone())
            }
            UnitConfigItem::UcItemInsDefIns(_) => {
                UnitConfigItem::UcItemInsDefIns(self.install_default_install.clone())
            }
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
