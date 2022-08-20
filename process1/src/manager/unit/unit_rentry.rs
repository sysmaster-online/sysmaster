use crate::manager::data::UnitRelations;
use crate::manager::reliability::{
    ReDb, ReDbRoTxn, ReDbRwTxn, ReDbTable, Reliability, RELI_DB_EUNIT_BASE, RELI_DB_EUNIT_CGROUP,
    RELI_DB_EUNIT_CHILD, RELI_DB_EUNIT_CONFIG, RELI_DB_EUNIT_DEP, RELI_DB_EUNIT_LOAD,
    RELI_DB_EUNIT_PPS,
};
use bitflags::bitflags;
use nix::unistd::Pid;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct UnitReBase {}

impl UnitReBase {
    fn new() -> UnitReBase {
        UnitReBase {}
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct UnitReLoad {
    cf_path: String,
    cf_mtime: u128,
}

impl UnitReLoad {
    fn new(cf_pathr: &String, cf_mtime: u128) -> UnitReLoad {
        UnitReLoad {
            cf_path: cf_pathr.clone(),
            cf_mtime,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct UnitReConfig {}

impl UnitReConfig {
    fn new() -> UnitReConfig {
        UnitReConfig {}
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct UnitReCgroup {
    cg_path: PathBuf,
}

impl UnitReCgroup {
    fn new(cg_pathr: &PathBuf) -> UnitReCgroup {
        UnitReCgroup {
            cg_path: cg_pathr.clone(),
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

pub(super) struct UnitRe {
    // database: multi-instance(N)
    base: Rc<ReDb<String, UnitReBase>>, // RELI_DB_EUNIT_BASE; key: unit_id, data: unit_type;
    load: Rc<ReDb<String, UnitReLoad>>, // RELI_DB_EUNIT_LOAD; key: unit_id, data: load_state+conf_path+conf_mtime;
    conf: Rc<ReDb<String, UnitReConfig>>, // RELI_DB_EUNIT_CONFIG; key: unit_id, data: unit_conf+install_conf;
    cgroup: Rc<ReDb<String, UnitReCgroup>>, // RELI_DB_EUNIT_CGROUP; key: unit_id, data: cg_path;
    child: Rc<ReDb<String, UnitReChild>>, // RELI_DB_EUNIT_CHILD; key: unit_id, data: pid[s];
    pps: Rc<ReDb<String, UnitRePps>>,     // RELI_DB_EUNIT_PPS; key: unit_id, data: pipeline[s];
    dep: Rc<ReDb<String, UnitReDep>>, // RELI_DB_EUNIT_DEP; key: unit_id, data: {UnitRelation+unit_id}[s]
}

impl UnitRe {
    pub(super) fn new(relir: &Reliability) -> UnitRe {
        let base = Rc::new(ReDb::new(relir, RELI_DB_EUNIT_BASE));
        let load = Rc::new(ReDb::new(relir, RELI_DB_EUNIT_LOAD));
        let conf = Rc::new(ReDb::new(relir, RELI_DB_EUNIT_CONFIG));
        let cgroup = Rc::new(ReDb::new(relir, RELI_DB_EUNIT_CGROUP));
        let child = Rc::new(ReDb::new(relir, RELI_DB_EUNIT_CHILD));
        let pps = Rc::new(ReDb::new(relir, RELI_DB_EUNIT_PPS));
        let dep = Rc::new(ReDb::new(relir, RELI_DB_EUNIT_DEP));
        let rentry = UnitRe {
            base,
            load,
            conf,
            cgroup,
            child,
            pps,
            dep,
        };
        rentry.register(relir);
        rentry
    }

    pub(super) fn base_insert(&self, unit_id: &String) {
        let u_base = UnitReBase::new();
        self.base.insert(unit_id.clone(), u_base);
    }

    pub(super) fn base_remove(&self, unit_id: &String) {
        self.base.remove(unit_id);
    }

    pub(super) fn base_get(&self, unit_id: &String) -> Option<()> {
        let u_base = self.base.get(unit_id);
        u_base.map(|_b| ())
    }

    pub(super) fn base_keys(&self) -> Vec<String> {
        self.base.keys()
    }

    pub(super) fn load_insert(&self, unit_id: &String, cf_pathr: &String, cf_mtime: u128) {
        assert!(self.base_contains(unit_id));

        let u_load = UnitReLoad::new(cf_pathr, cf_mtime);
        self.load.insert(unit_id.clone(), u_load);
    }

    pub(super) fn load_remove(&self, unit_id: &String) {
        self.load.remove(unit_id);
    }

    pub(super) fn load_get(&self, unit_id: &String) -> Option<(String, u128)> {
        let u_load = self.load.get(unit_id);
        u_load.map(|l| (l.cf_path, l.cf_mtime))
    }

    pub(super) fn conf_insert(&self, unit_id: &String) {
        assert!(self.base_contains(unit_id));

        let u_conf = UnitReConfig::new();
        self.conf.insert(unit_id.clone(), u_conf);
    }

    pub(super) fn conf_remove(&self, unit_id: &String) {
        self.conf.remove(unit_id);
    }

    pub(super) fn conf_get(&self, unit_id: &String) -> Option<()> {
        let u_conf = self.conf.get(unit_id);
        u_conf.map(|_c| ())
    }

    pub(super) fn cgroup_insert(&self, unit_id: &String, cg_path: &PathBuf) {
        assert!(self.base_contains(unit_id));

        let u_cgroup = UnitReCgroup::new(cg_path);
        self.cgroup.insert(unit_id.clone(), u_cgroup);
    }

    pub(super) fn cgroup_remove(&self, unit_id: &String) {
        self.cgroup.remove(unit_id);
    }

    pub(super) fn cgroup_get(&self, unit_id: &String) -> Option<PathBuf> {
        let u_cgroup = self.cgroup.get(unit_id);
        u_cgroup.map(|c| c.cg_path)
    }

    pub(super) fn child_insert(&self, unit_id: &String, pids: &Vec<Pid>) {
        assert!(self.base_contains(unit_id));

        let u_child = UnitReChild::new(pids.iter().map(|x| x.as_raw() as i32).collect::<_>());
        self.child.insert(unit_id.clone(), u_child);
    }

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

    pub(super) fn dep_insert(&self, unit_id: &String, deps: &Vec<(UnitRelations, String)>) {
        assert!(self.base_contains(unit_id));

        let mut ud_config = UnitReDep::new();
        ud_config.deps = deps.clone();
        self.dep.insert(unit_id.clone(), ud_config);
    }

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

    fn register(&self, relir: &Reliability) {
        // reliability-db: RELI_DB_EUNIT_BASE
        let db = Rc::clone(&self.base);
        relir.entry_db_register(RELI_DB_EUNIT_BASE, db);

        // reliability-db: RELI_DB_EUNIT_LOAD
        let db = Rc::clone(&self.load);
        relir.entry_db_register(RELI_DB_EUNIT_LOAD, db);

        // reliability-db: RELI_DB_EUNIT_CONFIG
        let db = Rc::clone(&self.conf);
        relir.entry_db_register(RELI_DB_EUNIT_CONFIG, db);

        // reliability-db: RELI_DB_EUNIT_CGROUP
        let db = Rc::clone(&self.cgroup);
        relir.entry_db_register(RELI_DB_EUNIT_CGROUP, db);

        // reliability-db: RELI_DB_EUNIT_CHILD
        let db = Rc::clone(&self.child);
        relir.entry_db_register(RELI_DB_EUNIT_CHILD, db);

        // reliability-db: RELI_DB_EUNIT_PPS
        let db = Rc::clone(&self.pps);
        relir.entry_db_register(RELI_DB_EUNIT_PPS, db);

        // reliability-db: RELI_DB_EUNIT_DEP
        let db = Rc::clone(&self.dep);
        relir.entry_db_register(RELI_DB_EUNIT_DEP, db);
    }
}

impl ReDbTable for ReDb<String, UnitReBase> {
    fn clear(&self, wtxn: &mut ReDbRwTxn) {
        self.do_clear(wtxn);
    }

    fn export(&self, db_wtxn: &mut ReDbRwTxn) {
        self.cache_2_db(db_wtxn);
    }

    fn import<'a>(&self, db_rtxn: &'a ReDbRoTxn) {
        self.db_2_cache(db_rtxn);
    }

    fn ignore_set(&self, ignore: bool) {
        self.set_ignore(ignore);
    }
}

impl ReDbTable for ReDb<String, UnitReLoad> {
    fn clear(&self, wtxn: &mut ReDbRwTxn) {
        self.do_clear(wtxn);
    }

    fn export(&self, db_wtxn: &mut ReDbRwTxn) {
        self.cache_2_db(db_wtxn);
    }

    fn import<'a>(&self, db_rtxn: &'a ReDbRoTxn) {
        self.db_2_cache(db_rtxn);
    }

    fn ignore_set(&self, ignore: bool) {
        self.set_ignore(ignore);
    }
}

impl ReDbTable for ReDb<String, UnitReConfig> {
    fn clear(&self, wtxn: &mut ReDbRwTxn) {
        self.do_clear(wtxn);
    }

    fn export(&self, db_wtxn: &mut ReDbRwTxn) {
        self.cache_2_db(db_wtxn);
    }

    fn import<'a>(&self, db_rtxn: &'a ReDbRoTxn) {
        self.db_2_cache(db_rtxn);
    }

    fn ignore_set(&self, ignore: bool) {
        self.set_ignore(ignore);
    }
}

impl ReDbTable for ReDb<String, UnitReCgroup> {
    fn clear(&self, wtxn: &mut ReDbRwTxn) {
        self.do_clear(wtxn);
    }

    fn export(&self, db_wtxn: &mut ReDbRwTxn) {
        self.cache_2_db(db_wtxn);
    }

    fn import<'a>(&self, db_rtxn: &'a ReDbRoTxn) {
        self.db_2_cache(db_rtxn);
    }

    fn ignore_set(&self, ignore: bool) {
        self.set_ignore(ignore);
    }
}

impl ReDbTable for ReDb<String, UnitReChild> {
    fn clear(&self, wtxn: &mut ReDbRwTxn) {
        self.do_clear(wtxn);
    }

    fn export(&self, db_wtxn: &mut ReDbRwTxn) {
        self.cache_2_db(db_wtxn);
    }

    fn import<'a>(&self, db_rtxn: &'a ReDbRoTxn) {
        self.db_2_cache(db_rtxn);
    }

    fn ignore_set(&self, ignore: bool) {
        self.set_ignore(ignore);
    }
}

impl ReDbTable for ReDb<String, UnitRePps> {
    fn clear(&self, wtxn: &mut ReDbRwTxn) {
        self.do_clear(wtxn);
    }

    fn export(&self, db_wtxn: &mut ReDbRwTxn) {
        self.cache_2_db(db_wtxn);
    }

    fn import<'a>(&self, db_rtxn: &'a ReDbRoTxn) {
        self.db_2_cache(db_rtxn);
    }

    fn ignore_set(&self, ignore: bool) {
        self.set_ignore(ignore);
    }
}

impl ReDbTable for ReDb<String, UnitReDep> {
    fn clear(&self, wtxn: &mut ReDbRwTxn) {
        self.do_clear(wtxn);
    }

    fn export(&self, db_wtxn: &mut ReDbRwTxn) {
        self.cache_2_db(db_wtxn);
    }

    fn import<'a>(&self, db_rtxn: &'a ReDbRoTxn) {
        self.db_2_cache(db_rtxn);
    }

    fn ignore_set(&self, ignore: bool) {
        self.set_ignore(ignore);
    }
}
