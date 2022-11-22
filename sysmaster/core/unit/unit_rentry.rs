#![allow(non_snake_case)]

use crate::core::manager::rentry::{
    RELI_DB_HUM_NOTIFY, RELI_DB_HUNIT_BASE, RELI_DB_HUNIT_CGROUP, RELI_DB_HUNIT_CHILD,
    RELI_DB_HUNIT_CONFIG, RELI_DB_HUNIT_DEP, RELI_DB_HUNIT_LOAD, RELI_DB_HUNIT_PPS,
};
use bitflags::bitflags;
use confique::Config;
use libsysmaster::reliability::{Reliability,ReDb};
use libsysmaster::unit::{UnitRelations,UnitType};
use libutils::serialize::DeserializeWith;
use nix::unistd::Pid;
use serde::{Deserialize, Deserializer, Serialize};
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub(crate) fn unit_name_to_type(unit_name: &str) -> UnitType {
    let words: Vec<&str> = unit_name.split('.').collect();
    match words[words.len() - 1] {
        "service" => UnitType::UnitService,
        "target" => UnitType::UnitTarget,
        "socket" => UnitType::UnitSocket,
        "mount" => UnitType::UnitMount,
        _ => UnitType::UnitTypeInvalid,
    }
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
    UnitStub = 0,
    UnitLoaded,
    UnitNotFound,
    UnitError,
    UnitMerged,
    UnitMasked,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct UnitReLoad {
    load_state: UnitLoadState,
}

impl UnitReLoad {
    fn new(load_state: UnitLoadState) -> UnitReLoad {
        UnitReLoad { load_state }
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
        JobMode::Replace
    }
}

impl DeserializeWith for JobMode {
    type Item = Self;
    fn deserialize_with<'de, D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(de)?;

        match s.as_ref() {
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

#[derive(Config, Default, Clone, Debug, Serialize, Deserialize)]
pub(crate) struct UeConfigUnit {
    #[config(default = "")]
    pub Description: String,
    #[config(default = "")]
    pub Documentation: String,
    #[config(default = false)]
    pub AllowIsolate: bool,
    //When set to true, the unit will not be stopped when systemctl isolate is executed. For service, target, socket timer and path, the default value is false. For other units, the default value is true
    #[config(default = false)]
    pub IgnoreOnIsolate: bool,
    #[config(default = true)]
    pub DefaultDependencies: bool,
    // #[config(deserialize_with = JobMode::deserialize_with)]
    // #[config(default = "replace")]
    // pub on_success_job_mode: JobMode,
    #[config(deserialize_with = JobMode::deserialize_with)]
    #[config(default = "replace")]
    pub OnFailureJobMode: JobMode,
    #[config(deserialize_with = Vec::<String>::deserialize_with)]
    #[config(default = "")]
    pub Wants: Vec<String>,
    #[config(deserialize_with = Vec::<String>::deserialize_with)]
    #[config(default = "")]
    pub Requires: Vec<String>,
    #[config(deserialize_with = Vec::<String>::deserialize_with)]
    #[config(default = "")]
    pub Before: Vec<String>,
    #[config(deserialize_with = Vec::<String>::deserialize_with)]
    #[config(default = "")]
    pub After: Vec<String>,
    #[config(deserialize_with = Vec::<String>::deserialize_with)]
    #[config(default = "")]
    pub Conflicts: Vec<String>,
    #[config(default = "")]
    pub ConditionFileNotEmpty: String,
    #[config(default = "")]
    pub ConditionNeedsUpdate: String,
    #[config(default = "")]
    pub ConditionPathExists: String,
    #[config(default = "")]
    pub AssertPathExists: String,
    #[config(default = 0)]
    pub StartLimitInterval: u64,
    #[config(default = 0)]
    pub StartLimitBurst: u32,
}

#[derive(Config, Default, Clone, Debug, Serialize, Deserialize)]
pub struct UeConfigInstall {
    #[config(deserialize_with = Vec::<String>::deserialize_with)]
    #[config(default = "")]
    pub Alias: Vec<String>,
    #[config(deserialize_with = Vec::<String>::deserialize_with)]
    #[config(default = "")]
    pub WantedBy: Vec<String>,
    #[config(deserialize_with = Vec::<String>::deserialize_with)]
    #[config(default = "")]
    pub RequiredBy: Vec<String>,
    #[config(deserialize_with = Vec::<String>::deserialize_with)]
    #[config(default = "")]
    pub Also: Vec<String>,
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

pub(in crate::core) struct UnitRe {
    // database: multi-instance(N)
    base: Rc<ReDb<String, UnitReBase>>, // RELI_DB_HUNIT_BASE; key: unit_id, data: unit_type;
    load: Rc<ReDb<String, UnitReLoad>>, // RELI_DB_HUNIT_LOAD; key: unit_id, data: load_state;
    conf: Rc<ReDb<String, UnitReConfig>>, // RELI_DB_HUNIT_CONFIG; key: unit_id, data: unit_conf+install_conf;
    cgroup: Rc<ReDb<String, UnitReCgroup>>, // RELI_DB_HUNIT_CGROUP; key: unit_id, data: cg_path;
    child: Rc<ReDb<String, UnitReChild>>, // RELI_DB_HUNIT_CHILD; key: unit_id, data: pid[s];
    pps: Rc<ReDb<String, UnitRePps>>,     // RELI_DB_HUNIT_PPS; key: unit_id, data: pipeline[s];
    dep: Rc<ReDb<String, UnitReDep>>, // RELI_DB_HUNIT_DEP; key: unit_id, data: {UnitRelation+unit_id}[s]

    // database: singleton(1)
    notify: Rc<ReDb<u32, UmReNotify>>, // RELI_DB_HUM_NOTIFY; key: RELI_LAST_KEY, data: UmReNotify;
}

impl UnitRe {
    pub fn new(relir: &Reliability) -> UnitRe {
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
    pub(super) fn base_remove(&self, unit_id: &String) {
        self.base.remove(unit_id);
    }

    pub(super) fn base_get(&self, unit_id: &String) -> Option<UnitType> {
        let u_base = self.base.get(unit_id);
        u_base.map(|b| b.unit_type)
    }

    pub(super) fn base_keys(&self) -> Vec<String> {
        self.base.keys()
    }

    pub(super) fn load_insert(&self, unit_id: &String, load_state: UnitLoadState) {
        assert!(self.base_contains(unit_id));

        let u_load = UnitReLoad::new(load_state);
        self.load.insert(unit_id.clone(), u_load);
    }

    #[allow(dead_code)]
    pub(super) fn load_remove(&self, unit_id: &String) {
        self.load.remove(unit_id);
    }

    pub(super) fn load_get(&self, unit_id: &String) -> Option<UnitLoadState> {
        let u_load = self.load.get(unit_id);
        u_load.map(|l| l.load_state)
    }

    pub(super) fn conf_insert(
        &self,
        unit_id: &String,
        unit: &UeConfigUnit,
        install: &UeConfigInstall,
    ) {
        assert!(self.base_contains(unit_id));

        let u_conf = UnitReConfig::new(unit, install);
        self.conf.insert(unit_id.clone(), u_conf);
    }

    #[allow(dead_code)]
    pub(super) fn conf_remove(&self, unit_id: &String) {
        self.conf.remove(unit_id);
    }

    pub(super) fn conf_get(&self, unit_id: &String) -> Option<(UeConfigUnit, UeConfigInstall)> {
        let u_conf = self.conf.get(unit_id);
        u_conf.map(|c| (c.unit, c.install))
    }

    pub(super) fn cgroup_insert(&self, unit_id: &String, cg_path: &Path) {
        assert!(self.base_contains(unit_id));

        let u_cgroup = UnitReCgroup::new(cg_path);
        self.cgroup.insert(unit_id.clone(), u_cgroup);
    }

    #[allow(dead_code)]
    pub(super) fn cgroup_remove(&self, unit_id: &String) {
        self.cgroup.remove(unit_id);
    }

    pub(super) fn cgroup_get(&self, unit_id: &String) -> Option<PathBuf> {
        let u_cgroup = self.cgroup.get(unit_id);
        u_cgroup.map(|c| c.cg_path)
    }

    pub(super) fn child_insert(&self, unit_id: &String, pids: &[Pid]) {
        assert!(self.base_contains(unit_id));

        let u_child = UnitReChild::new(pids.iter().map(|x| x.as_raw() as i32).collect::<_>());
        self.child.insert(unit_id.clone(), u_child);
    }

    #[allow(dead_code)]
    pub(super) fn child_remove(&self, unit_id: &String) {
        self.child.remove(unit_id);
    }

    pub(super) fn child_get(&self, unit_id: &String) -> Vec<Pid> {
        let mut pids = Vec::new();
        if let Some(u_child) = self.child.get(unit_id) {
            for pid in u_child.pids.iter() {
                pids.push(Pid::from_raw(*pid));
            }
        }
        pids
    }

    pub(super) fn child_keys(&self) -> Vec<String> {
        self.child.keys()
    }

    pub(super) fn pps_insert(&self, unit_id: &String) {
        assert!(self.base_contains(unit_id));

        let pps_empty = UnitRePps::empty();
        self.pps.insert(unit_id.clone(), pps_empty);
    }

    pub(super) fn pps_set(&self, unit_id: &String, pps_mask: UnitRePps) {
        assert!(self.base_contains(unit_id));

        let mut pps = self.pps_get(unit_id);
        pps.insert(pps_mask);
        self.pps.insert(unit_id.clone(), pps);
    }

    pub(super) fn pps_clear(&self, unit_id: &String, pps_mask: UnitRePps) {
        assert!(self.base_contains(unit_id));

        let mut pps = self.pps_get(unit_id);
        pps.remove(pps_mask);
        self.pps.insert(unit_id.clone(), pps);
    }

    #[allow(dead_code)]
    pub(super) fn pps_remove(&self, unit_id: &String) {
        self.pps.remove(unit_id);
    }

    pub(super) fn pps_contains(&self, unit_id: &String, pps_mask: UnitRePps) -> bool {
        let pps = self.pps_get(unit_id);
        pps.contains(pps_mask)
    }

    pub(super) fn pps_keys(&self) -> Vec<String> {
        self.pps.keys()
    }

    pub(super) fn dep_insert(&self, unit_id: &String, deps: &[(UnitRelations, String)]) {
        assert!(self.base_contains(unit_id));

        let mut ud_config = UnitReDep::new();
        ud_config.deps = deps.to_vec();
        self.dep.insert(unit_id.clone(), ud_config);
    }

    #[allow(dead_code)]
    pub(super) fn dep_remove(&self, unit_id: &String) {
        self.dep.remove(unit_id);
    }

    pub(super) fn dep_get(&self, unit_id: &String) -> Vec<(UnitRelations, String)> {
        let mut deps = Vec::new();
        if let Some(mut ud_config) = self.dep.get(unit_id) {
            deps.append(&mut ud_config.deps);
        }
        deps
    }

    pub(super) fn dep_keys(&self) -> Vec<String> {
        self.dep.keys()
    }

    fn base_contains(&self, unit_id: &String) -> bool {
        self.base.contains_key(unit_id)
    }

    fn pps_get(&self, unit_id: &String) -> UnitRePps {
        let pps_empty = UnitRePps::empty();
        self.pps.get(unit_id).unwrap_or(pps_empty)
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
