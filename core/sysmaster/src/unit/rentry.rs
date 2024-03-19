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

use crate::manager::rentry::{
    RELI_DB_HUM_NOTIFY, RELI_DB_HUNIT_BASE, RELI_DB_HUNIT_CGROUP, RELI_DB_HUNIT_CHILD,
    RELI_DB_HUNIT_CONFIG, RELI_DB_HUNIT_DEP, RELI_DB_HUNIT_LOAD, RELI_DB_HUNIT_PPS,
};
use crate::unit::entry::UnitEmergencyAction;
use bitflags::bitflags;
use core::error::*;
use core::rel::{ReDb, Reliability};
use core::serialize::DeserializeWith;
use core::unit::{UnitRelations, UnitType};
use nix::unistd::Pid;
use serde::{de, Deserialize, Deserializer, Serialize};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::str::FromStr;
use unit_parser::prelude::{UnitEntry, UnitSection};

pub(crate) fn unit_name_to_type(unit_name: &str) -> UnitType {
    let words: Vec<&str> = unit_name.split('.').collect();
    if words.is_empty() {
        return UnitType::UnitTypeInvalid;
    }
    UnitType::from_str(words[words.len() - 1]).unwrap_or(UnitType::UnitTypeInvalid)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct UnitReBase {
    unit_type: UnitType,
}

impl UnitReBase {
    fn new(unit_type: UnitType) -> UnitReBase {
        UnitReBase { unit_type }
    }
}

#[allow(clippy::enum_variant_names)]
#[derive(PartialEq, Debug, Eq, Copy, Clone, Serialize, Deserialize)]
pub enum UnitLoadState {
    Stub = 0,
    Loaded,
    NotFound,
    Error,
    BadSetting,
    Merged,
    Masked,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct UnitReLoad {
    load_state: UnitLoadState,
    transient: bool,
    paths: Vec<PathBuf>,
    transient_file: Option<PathBuf>,
    last_section_private: i8,
}

impl UnitReLoad {
    fn new(
        load_state: UnitLoadState,
        transient: bool,
        paths: Vec<PathBuf>,
        transient_file: Option<PathBuf>,
        last_section_private: i8,
    ) -> UnitReLoad {
        UnitReLoad {
            load_state,
            transient,
            paths,
            transient_file,
            last_section_private,
        }
    }
}

/// jobMode why in UnitRentry? change the name?
#[derive(Clone, Debug, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum JobMode {
    #[serde(alias = "fail")]
    Fail,
    #[serde(alias = "replace")]
    Replace,
    #[serde(alias = "replace_irreversible")]
    ReplaceIrreversible,
    #[serde(alias = "isolate")]
    Isolate,
    #[serde(alias = "flush")]
    Flush,
    #[serde(alias = "ignore_dependencies")]
    IgnoreDependencies,
    #[serde(alias = "ignore_requirements")]
    IgnoreRequirements,
    #[serde(alias = "trigger")]
    Trigger,
}

impl Default for JobMode {
    fn default() -> Self {
        Self::Replace
    }
}

impl FromStr for JobMode {
    type Err = basic::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "fail" => Ok(JobMode::Fail),
            "replace" => Ok(JobMode::Replace),
            "replace_irreversible" => Ok(JobMode::ReplaceIrreversible),
            "isolate" => Ok(JobMode::Isolate),
            "flush" => Ok(JobMode::Flush),
            "ignore_dependencies" => Ok(JobMode::IgnoreDependencies),
            "ignore_requirements" => Ok(JobMode::IgnoreRequirements),
            "trigger" => Ok(JobMode::Trigger),
            &_ => Ok(JobMode::Replace),
        }
    }
}

impl DeserializeWith for JobMode {
    type Item = Self;
    fn deserialize_with<'de, D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(de)?;
        JobMode::from_str(&s).map_err(de::Error::custom)
    }
}

impl UnitEntry for JobMode {
    type Error = basic::error::Error;

    fn parse_from_str<S: AsRef<str>>(input: S) -> std::result::Result<Self, Self::Error> {
        let job_mode = JobMode::from_str(input.as_ref())?;
        Ok(job_mode)
    }
}

#[derive(UnitSection, Default, Clone, Debug, Serialize, Deserialize)]
pub(crate) struct UeConfigUnit {
    #[entry(default = String::new())]
    pub Description: String,
    #[entry(default = String::new())]
    pub Documentation: String,
    //When set to true, the unit will not be stopped when systemctl isolate is executed. For service, target, socket timer and path, the default value is false. For other units, the default value is true
    #[entry(default = false)]
    pub IgnoreOnIsolate: bool,
    #[entry(default = true)]
    pub DefaultDependencies: bool,
    #[entry(default = false)]
    pub RefuseManualStart: bool,
    #[entry(default = false)]
    pub RefuseManualStop: bool,
    #[entry(default = JobMode::Replace)]
    #[entry(default = JobMode::Replace)]
    pub OnFailureJobMode: JobMode,
    #[entry(default = JobMode::Replace)]
    pub OnSuccessJobMode: JobMode,
    #[entry(append)]
    pub Wants: Vec<String>,
    #[entry(append)]
    pub Requires: Vec<String>,
    #[entry(append)]
    pub BindsTo: Vec<String>,
    #[entry(append)]
    pub Requisite: Vec<String>,
    #[entry(append)]
    pub PartOf: Vec<String>,
    #[entry(append)]
    pub OnFailure: Vec<String>,
    #[entry(append)]
    pub OnSuccess: Vec<String>,
    #[entry(append)]
    pub Before: Vec<String>,
    #[entry(append)]
    pub After: Vec<String>,
    #[entry(append)]
    pub Conflicts: Vec<String>,

    /* Conditions */
    #[entry()]
    pub ConditionACPower: Option<bool>,
    #[entry(default = String::new())]
    pub ConditionCapability: String,
    #[entry(default = String::new())]
    pub ConditionDirectoryNotEmpty: String,
    #[entry(default = String::new())]
    pub ConditionFileIsExecutable: String,
    #[entry(default = String::new())]
    pub ConditionFileNotEmpty: String,
    #[entry()]
    pub ConditionFirstBoot: Option<bool>,
    #[entry(default = String::new())]
    pub ConditionKernelCommandLine: String,
    #[entry(default = String::new())]
    pub ConditionNeedsUpdate: String,
    #[entry(default = String::new())]
    pub ConditionPathExists: String,
    #[entry(default = String::new())]
    pub ConditionPathExistsGlob: String,
    #[entry(default = String::new())]
    pub ConditionPathIsDirectory: String,
    #[entry(default = String::new())]
    pub ConditionPathIsMountPoint: String,
    #[entry(default = String::new())]
    pub ConditionPathIsReadWrite: String,
    #[entry(default = String::new())]
    pub ConditionPathIsSymbolicLink: String,
    #[entry(default = String::new())]
    pub ConditionSecurity: String,
    #[entry(default = String::new())]
    pub ConditionUser: String,

    /* Asserts */
    #[entry(default = String::new())]
    pub AssertPathExists: String,
    #[entry(default = 10)]
    pub StartLimitInterval: u64,
    #[entry(default = 10)]
    pub StartLimitIntervalSec: u64,
    #[entry(default = 5)]
    pub StartLimitBurst: u32,
    #[entry(default = UnitEmergencyAction::None)]
    pub SuccessAction: UnitEmergencyAction,
    #[entry(default = UnitEmergencyAction::None)]
    pub FailureAction: UnitEmergencyAction,
    #[entry(default = UnitEmergencyAction::None)]
    pub StartLimitAction: UnitEmergencyAction,
    #[entry(default = 0)]
    pub JobTimeoutSec: u64,
    #[entry(default = UnitEmergencyAction::None)]
    pub JobTimeoutAction: UnitEmergencyAction,
}

impl UeConfigUnit {
    pub(crate) fn set_property(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "Description" => self.Description = value.to_string(),
            "Documentation" => self.Documentation = value.to_string(),
            "IgnoreOnIsolate" => self.IgnoreOnIsolate = basic::config::parse_boolean(value)?,
            "DefaultDependencies" => {
                self.DefaultDependencies = basic::config::parse_boolean(value)?
            }
            "RefuseManualStart" => self.RefuseManualStart = basic::config::parse_boolean(value)?,
            "RefuseManualStop" => self.RefuseManualStop = basic::config::parse_boolean(value)?,
            "OnFailureJobMode" => self.OnFailureJobMode = JobMode::parse_from_str(value)?,
            "OnSuccessJobMode" => self.OnSuccessJobMode = JobMode::parse_from_str(value)?,
            "Wants" => self.Wants = vec_str_2_string(value),
            "Requires" => self.Requires = vec_str_2_string(value),
            "BindsTo" => self.BindsTo = vec_str_2_string(value),
            "Requisite" => self.Requisite = vec_str_2_string(value),
            "PartOf" => self.PartOf = vec_str_2_string(value),
            "OnFailure" => self.OnFailure = vec_str_2_string(value),
            "OnSuccess" => self.OnSuccess = vec_str_2_string(value),
            "Before" => self.Before = vec_str_2_string(value),
            "After" => self.After = vec_str_2_string(value),
            "Conflicts" => self.Conflicts = vec_str_2_string(value),

            /* Conditions */
            "ConditionACPower" => {
                self.ConditionACPower = Some(basic::config::parse_boolean(value)?)
            }
            "ConditionCapability" => self.ConditionCapability = value.to_string(),
            "ConditionDirectoryNotEmpty" => self.ConditionDirectoryNotEmpty = value.to_string(),
            "ConditionFileIsExecutable" => self.ConditionFileIsExecutable = value.to_string(),
            "ConditionFileNotEmpty" => self.ConditionFileNotEmpty = value.to_string(),
            "ConditionFirstBoot" => {
                self.ConditionFirstBoot = Some(basic::config::parse_boolean(value)?)
            }
            "ConditionKernelCommandLine" => self.ConditionKernelCommandLine = value.to_string(),
            "ConditionNeedsUpdate" => self.ConditionNeedsUpdate = value.to_string(),
            "ConditionPathExists" => self.ConditionPathExists = value.to_string(),
            "ConditionPathExistsGlob" => self.ConditionPathExistsGlob = value.to_string(),
            "ConditionPathIsDirectory" => self.ConditionPathIsDirectory = value.to_string(),
            "ConditionPathIsMountPoint" => self.ConditionPathIsMountPoint = value.to_string(),
            "ConditionPathIsReadWrite" => self.ConditionPathIsReadWrite = value.to_string(),
            "ConditionPathIsSymbolicLink" => self.ConditionPathIsSymbolicLink = value.to_string(),
            "ConditionSecurity" => self.ConditionSecurity = value.to_string(),
            "ConditionUser" => self.ConditionUser = value.to_string(),

            /* Asserts */
            "AssertPathExists" => self.AssertPathExists = value.to_string(),

            "StartLimitInterval" => self.StartLimitInterval = value.parse::<u64>()?,
            "StartLimitIntervalSec" => self.StartLimitIntervalSec = value.parse::<u64>()?,
            "StartLimitBurst" => self.StartLimitBurst = value.parse::<u32>()?,
            "SuccessAction" => self.SuccessAction = UnitEmergencyAction::parse_from_str(value)?,
            "FailureAction" => self.FailureAction = UnitEmergencyAction::parse_from_str(value)?,
            "StartLimitAction" => {
                self.StartLimitAction = UnitEmergencyAction::parse_from_str(value)?
            }
            "JobTimeoutSec" => self.JobTimeoutSec = value.parse::<u64>()?,
            "JobTimeoutAction" => {
                self.JobTimeoutAction = UnitEmergencyAction::parse_from_str(value)?
            }
            str_key => {
                return Err(Error::NotFound {
                    what: format!("set property:{}", str_key),
                })
            }
        };
        Ok(())
    }
}

#[derive(UnitSection, Default, Clone, Debug, Serialize, Deserialize)]
pub struct UeConfigInstall {
    #[entry(append)]
    pub Alias: Vec<String>,
    #[entry(append)]
    pub WantedBy: Vec<String>,
    #[entry(append)]
    pub RequiredBy: Vec<String>,
    #[entry(append)]
    pub Also: Vec<String>,
}

impl UeConfigInstall {
    pub(crate) fn set_property(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "Alias" => self.Alias = vec_str_2_string(value),
            "WantedBy" => self.WantedBy = vec_str_2_string(value),
            "RequiredBy" => self.RequiredBy = vec_str_2_string(value),
            "Also" => self.Also = vec_str_2_string(value),
            str_key => {
                return Err(Error::NotFound {
                    what: format!("set property:{}", str_key),
                })
            }
        };
        Ok(())
    }
}

fn vec_str_2_string(str: &str) -> Vec<String> {
    str.split_whitespace().map(|s| s.to_string()).collect()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct UnitReConfig {
    unit: UeConfigUnit,
    install: UeConfigInstall,
}

impl UnitReConfig {
    fn new(unitr: &UeConfigUnit, installr: &UeConfigInstall) -> UnitReConfig {
        UnitReConfig {
            unit: unitr.clone(),
            install: installr.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct UnitReCgroup {
    cg_path: PathBuf,
}

impl UnitReCgroup {
    fn new(cg_pathr: &Path) -> UnitReCgroup {
        UnitReCgroup {
            cg_path: cg_pathr.to_path_buf(),
        }
    }
}

bitflags! {
    #[derive(Serialize, Deserialize)]
    pub(super) struct UnitRePps: u32 {
        // queue
        const QUEUE_CG_EMPTY      = 0x00000001;
        const QUEUE_CG_OOM        = 0x00000002;
        const QUEUE_CG_REALIZE    = 0x00000004;
        const QUEUE_LOAD          = 0x00000008;
        const QUEUE_TARGET_DEPS   = 0x00000010;
        const QUEUE_GC            = 0x00000020; // ignore without last
        const QUEUE_CLEAN         = 0x00000040;
        const QUEUE_DBUS          = 0x00000080;
        const QUEUE_STOP_UNNEEDED = 0x00000100;
        const QUEUE_START_UPHELD  = 0x00000200;
        const QUEUE_STOP_BOUND    = 0x00000400;

        // event
        const EVENT_REWATCH_PIDS  = 0x00010000;
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct UnitReChild {
    pids: Vec<i32>, // i32 ==> nix::unistd::Pid ==> libc::pid_t
}

impl UnitReChild {
    fn new(pids: Vec<i32>) -> UnitReChild {
        UnitReChild { pids }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct UnitReDep {
    deps: Vec<(UnitRelations, String)>,
}

impl UnitReDep {
    fn new() -> UnitReDep {
        UnitReDep { deps: Vec::new() }
    }
}

const RELI_LAST_KEY: u32 = 0; // singleton

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct UmReNotify {
    fd: i32,
}

impl UmReNotify {
    fn new(fd: i32) -> UmReNotify {
        UmReNotify { fd }
    }
}

pub(crate) struct UnitRe {
    // database: multi-instance(N)
    base: Rc<ReDb<String, UnitReBase>>, // RELI_DB_HUNIT_BASE; key: unit_id, data: unit_type;
    load: Rc<ReDb<String, UnitReLoad>>, // RELI_DB_HUNIT_LOAD; key: unit_id, data: load_state+transient+paths;
    conf: Rc<ReDb<String, UnitReConfig>>, // RELI_DB_HUNIT_CONFIG; key: unit_id, data: unit_conf+install_conf;
    cgroup: Rc<ReDb<String, UnitReCgroup>>, // RELI_DB_HUNIT_CGROUP; key: unit_id, data: cg_path;
    child: Rc<ReDb<String, UnitReChild>>, // RELI_DB_HUNIT_CHILD; key: unit_id, data: pid[s];
    pps: Rc<ReDb<String, UnitRePps>>,     // RELI_DB_HUNIT_PPS; key: unit_id, data: pipeline[s];
    dep: Rc<ReDb<String, UnitReDep>>, // RELI_DB_HUNIT_DEP; key: unit_id, data: {UnitRelation+unit_id}[s]

    // database: singleton(1)
    notify: Rc<ReDb<u32, UmReNotify>>, // RELI_DB_HUM_NOTIFY; key: RELI_LAST_KEY, data: UmReNotify;
}

impl UnitRe {
    pub fn new(relir: &Rc<Reliability>) -> UnitRe {
        let base = Rc::new(ReDb::new(relir, RELI_DB_HUNIT_BASE));
        let load = Rc::new(ReDb::new(relir, RELI_DB_HUNIT_LOAD));
        let conf = Rc::new(ReDb::new(relir, RELI_DB_HUNIT_CONFIG));
        let cgroup = Rc::new(ReDb::new(relir, RELI_DB_HUNIT_CGROUP));
        let child = Rc::new(ReDb::new(relir, RELI_DB_HUNIT_CHILD));
        let pps = Rc::new(ReDb::new(relir, RELI_DB_HUNIT_PPS));
        let dep = Rc::new(ReDb::new(relir, RELI_DB_HUNIT_DEP));
        let notify = Rc::new(ReDb::new(relir, RELI_DB_HUM_NOTIFY));
        let rentry = UnitRe {
            base,
            load,
            conf,
            cgroup,
            child,
            pps,
            dep,
            notify,
        };
        rentry.register(relir);
        rentry
    }

    pub(super) fn base_insert(&self, unit_id: &str, unit_type: UnitType) {
        let u_base = UnitReBase::new(unit_type);
        self.base.insert(String::from(unit_id), u_base);
    }

    #[allow(dead_code)]
    pub(super) fn base_remove(&self, unit_id: &str) {
        self.base.remove(&unit_id.to_string());
    }

    pub(super) fn base_get(&self, unit_id: &str) -> Option<UnitType> {
        let u_base = self.base.get(&unit_id.to_string());
        u_base.map(|b| b.unit_type)
    }

    pub(super) fn base_keys(&self) -> Vec<String> {
        self.base.keys()
    }

    pub(super) fn load_insert(
        &self,
        unit_id: &str,
        load_state: UnitLoadState,
        transient: bool,
        paths: Vec<PathBuf>,
        transient_file: Option<PathBuf>,
        last_section_private: i8,
    ) {
        let u_load = UnitReLoad::new(
            load_state,
            transient,
            paths,
            transient_file,
            last_section_private,
        );
        self.load.insert(unit_id.to_owned(), u_load);
    }

    #[allow(dead_code)]
    pub(super) fn load_remove(&self, unit_id: &str) {
        self.load.remove(&unit_id.to_string());
    }

    #[allow(clippy::type_complexity)]
    pub(super) fn load_get(
        &self,
        unit_id: &str,
    ) -> Option<(UnitLoadState, bool, Vec<PathBuf>, Option<PathBuf>, i8)> {
        let u_load = self.load.get(&unit_id.to_string());
        u_load.map(|l| {
            (
                l.load_state,
                l.transient,
                l.paths,
                l.transient_file,
                l.last_section_private,
            )
        })
    }

    pub(super) fn conf_insert(
        &self,
        unit_id: &str,
        unit: &UeConfigUnit,
        install: &UeConfigInstall,
    ) {
        let u_conf = UnitReConfig::new(unit, install);
        self.conf.insert(unit_id.to_owned(), u_conf);
    }

    #[allow(dead_code)]
    pub(super) fn conf_remove(&self, unit_id: &str) {
        self.conf.remove(&unit_id.to_string());
    }

    pub(super) fn conf_get(&self, unit_id: &str) -> Option<(UeConfigUnit, UeConfigInstall)> {
        let u_conf = self.conf.get(&unit_id.to_string());
        u_conf.map(|c| (c.unit, c.install))
    }

    pub(super) fn cgroup_insert(&self, unit_id: &str, cg_path: &Path) {
        let u_cgroup = UnitReCgroup::new(cg_path);
        self.cgroup.insert(unit_id.to_owned(), u_cgroup);
    }

    #[allow(dead_code)]
    pub(super) fn cgroup_remove(&self, unit_id: &str) {
        self.cgroup.remove(&unit_id.to_string());
    }

    pub(super) fn cgroup_get(&self, unit_id: &str) -> Option<PathBuf> {
        let u_cgroup = self.cgroup.get(&unit_id.to_string());
        u_cgroup.map(|c| c.cg_path)
    }

    pub(super) fn child_insert(&self, unit_id: &str, pids: &[Pid]) {
        let u_child = UnitReChild::new(pids.iter().map(|x| x.as_raw()).collect::<_>());
        self.child.insert(unit_id.to_owned(), u_child);
    }

    #[allow(dead_code)]
    pub(super) fn child_remove(&self, unit_id: &str) {
        self.child.remove(&unit_id.to_string());
    }

    pub(super) fn child_get(&self, unit_id: &str) -> Vec<Pid> {
        let mut pids = Vec::new();
        if let Some(u_child) = self.child.get(&unit_id.to_string()) {
            for pid in u_child.pids.iter() {
                pids.push(Pid::from_raw(*pid));
            }
        }
        pids
    }

    pub(super) fn child_keys(&self) -> Vec<String> {
        self.child.keys()
    }

    pub(super) fn pps_insert(&self, unit_id: &str) {
        let pps_empty = UnitRePps::empty();
        self.pps.insert(unit_id.to_owned(), pps_empty);
    }

    pub(super) fn pps_set(&self, unit_id: &str, pps_mask: UnitRePps) {
        let mut pps = self.pps_get(&unit_id.to_string());
        pps.insert(pps_mask);
        self.pps.insert(unit_id.to_string(), pps);
    }

    pub(super) fn pps_clear(&self, unit_id: &str, pps_mask: UnitRePps) {
        let mut pps = self.pps_get(&unit_id.to_string());
        pps.remove(pps_mask);
        self.pps.insert(unit_id.to_string(), pps);
    }

    #[allow(dead_code)]
    pub(super) fn pps_remove(&self, unit_id: &str) {
        self.pps.remove(&unit_id.to_string());
    }

    pub(super) fn pps_contains(&self, unit_id: &str, pps_mask: UnitRePps) -> bool {
        let pps = self.pps_get(&unit_id.to_string());
        pps.contains(pps_mask)
    }

    pub(super) fn pps_keys(&self) -> Vec<String> {
        self.pps.keys()
    }

    pub(super) fn dep_insert(&self, unit_id: &str, deps: &[(UnitRelations, String)]) {
        let mut ud_config = UnitReDep::new();
        ud_config.deps = deps.to_vec();
        self.dep.insert(unit_id.to_owned(), ud_config);
    }

    #[allow(dead_code)]
    pub(super) fn dep_remove(&self, unit_id: &str) {
        self.dep.remove(&unit_id.to_string());
    }

    pub(super) fn dep_get(&self, unit_id: &str) -> Vec<(UnitRelations, String)> {
        let mut deps = Vec::new();
        if let Some(mut ud_config) = self.dep.get(&unit_id.to_string()) {
            deps.append(&mut ud_config.deps);
        }
        deps
    }

    pub(super) fn dep_keys(&self) -> Vec<String> {
        self.dep.keys()
    }

    fn pps_get(&self, unit_id: &str) -> UnitRePps {
        let pps_empty = UnitRePps::empty();
        self.pps.get(&unit_id.to_string()).unwrap_or(pps_empty)
    }

    pub(super) fn notify_insert(&self, fd: i32) {
        let um_notify = UmReNotify::new(fd);
        self.notify.insert(RELI_LAST_KEY, um_notify);
    }

    #[allow(dead_code)]
    pub(super) fn notify_remove(&self) {
        self.notify.remove(&RELI_LAST_KEY);
    }

    pub(super) fn notify_get(&self) -> Option<i32> {
        let um_notify = self.notify.get(&RELI_LAST_KEY);
        um_notify.map(|n| n.fd)
    }

    fn register(&self, relir: &Reliability) {
        // reliability-db: RELI_DB_HUNIT_BASE
        let db = Rc::clone(&self.base);
        relir.history_db_register(RELI_DB_HUNIT_BASE, db);

        // reliability-db: RELI_DB_HUNIT_LOAD
        let db = Rc::clone(&self.load);
        relir.history_db_register(RELI_DB_HUNIT_LOAD, db);

        // reliability-db: RELI_DB_HUNIT_CONFIG
        let db = Rc::clone(&self.conf);
        relir.history_db_register(RELI_DB_HUNIT_CONFIG, db);

        // reliability-db: RELI_DB_HUNIT_CGROUP
        let db = Rc::clone(&self.cgroup);
        relir.history_db_register(RELI_DB_HUNIT_CGROUP, db);

        // reliability-db: RELI_DB_HUNIT_CHILD
        let db = Rc::clone(&self.child);
        relir.history_db_register(RELI_DB_HUNIT_CHILD, db);

        // reliability-db: RELI_DB_HUNIT_PPS
        let db = Rc::clone(&self.pps);
        relir.history_db_register(RELI_DB_HUNIT_PPS, db);

        // reliability-db: RELI_DB_HUNIT_DEP
        let db = Rc::clone(&self.dep);
        relir.history_db_register(RELI_DB_HUNIT_DEP, db);

        // reliability-db: RELI_DB_HUM_NOTIFY
        let db = Rc::clone(&self.notify);
        relir.history_db_register(RELI_DB_HUM_NOTIFY, db);
    }
}
