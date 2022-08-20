use heed::types::{OwnedType, SerdeBincode, Str};
use heed::{Database, Env, EnvOpenOptions, RoTxn, RwTxn};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;
use std::hash::Hash;
use std::io::Error;
use std::path::Path;
use std::rc::Rc;

pub(super) const RELI_PATH_DIR: &str = "/run/systemd/reliability";

#[derive(Debug)]
pub struct Reliability {
    enable: ReliEnable,
    last: ReliLast,
    entry: ReliEntry,
    station: ReliStation,
}

impl Reliability {
    pub fn entry_db_register(&self, name: &str, db: Rc<dyn ReDbTable>) {
        self.entry.db_register(name, db);
    }

    pub fn station_register(&self, name: &str, kind: ReStationKind, station: Rc<dyn ReStation>) {
        self.station.station_register(name, kind, station);
    }

    pub(super) fn new() -> Reliability {
        reli_prepare().expect("reliability prepare");
        Reliability {
            enable: ReliEnable::new(),
            last: ReliLast::new(),
            entry: ReliEntry::new(),
            station: ReliStation::new(),
        }
    }

    pub(super) fn clear(&self) {
        self.enable.clear();
        self.last.clear();
        self.entry.clear();
    }

    pub(super) fn set_enable(&self, enable: bool) {
        self.enable.set_enable(enable);
    }

    pub(super) fn set_last_unit(&self, unit_id: &str) {
        self.last.set_unit(unit_id);
    }

    pub(super) fn clear_last_unit(&self) {
        self.entry.commit();
        self.last.clear_unit();
    }

    pub(super) fn set_last_frame(&self, frame: ReliLastFrame) {
        self.last.set_frame(frame);
    }

    pub(super) fn clear_last_frame(&self) {
        self.entry.commit();
        self.last.clear_frame();
    }

    pub(super) fn set_last_queue(&self, queue: ReliLastQue) {
        self.last.set_queue(queue);
    }

    // process reentrant
    pub(super) fn recover(&self) {
        // ignore last's input
        self.last.ignore_set(true);

        self.entry.import();
        self.input_rebuild();
        self.entry_compensate();
        self.entry_map();
        self.make_consistent();

        // restore last's ignore
        self.last.ignore_set(false);

        // clear last
        self.last.clear_unit();
        self.last.clear_frame();
    }

    pub(super) fn enable(&self) -> bool {
        self.enable.enable()
    }

    pub(super) fn last_unit(&self) -> Option<String> {
        self.last.unit()
    }

    pub(super) fn last_frame(&self) -> Option<ReliLastFrame> {
        self.last.frame()
    }

    pub(super) fn last_queue(&self) -> Option<ReliLastQue> {
        self.last.queue()
    }

    fn input_rebuild(&self) {
        // ignore entry's input
        self.entry.ignore_set(true);

        self.station.input_rebuild();

        // restore entry's ignore
        self.entry.ignore_set(false);
    }

    fn entry_compensate(&self) {
        // get last
        let lframe = self.last_frame();
        let lunit = self.last_unit();

        self.station.entry_compensate(lframe, lunit);
        self.entry.commit();
    }

    fn entry_map(&self) {
        // ignore entry's input
        self.entry.ignore_set(true);

        self.station.entry_map();

        // restore entry's ignore
        self.entry.ignore_set(false);
    }

    fn make_consistent(&self) {
        // get last
        let lframe = self.last_frame();
        let lunit = self.last_unit();

        self.station.make_consistent(lframe, lunit);
        self.entry.commit();
    }
}

// enable
const RELI_ENABLE_FILE: &str = "enable.mdb";
const RELI_ENABLE_MAX_DBS: u32 = 1;
const RELI_DB_ENABLE: &str = "enable";
static RELI_ENABLE_DB_NAME: [&str; RELI_ENABLE_MAX_DBS as usize] = [RELI_DB_ENABLE];
const RELI_ENABLE_KEY: u32 = 0; // singleton

struct ReliEnable {
    // data
    /* environment */
    env: Env,

    /* database: singleton(1) */
    enable: Database<OwnedType<u32>, SerdeBincode<bool>>, // RELI_DB_ENABLE; key: RELI_ENABLE_KEY, data: bool;
}

impl fmt::Debug for ReliEnable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReliEnable")
            .field("env.path", &self.env.path())
            .field("enable.len", &self.enable_len().unwrap_or(0))
            .finish()
    }
}

// the declaration "pub(self)" is for identification only.
impl ReliEnable {
    pub(self) fn new() -> ReliEnable {
        // init environment
        let path = Path::new(RELI_PATH_DIR).join(RELI_ENABLE_FILE);
        let env = EnvOpenOptions::new()
            .max_dbs(RELI_ENABLE_MAX_DBS)
            .open(path)
            .unwrap();

        // create db
        let enable = env.create_database(Some(RELI_DB_ENABLE)).unwrap();

        // return
        ReliEnable { env, enable }
    }

    pub(self) fn clear(&self) {
        let mut wtxn = self.env.write_txn().expect("enable.write_txn");
        self.enable.clear(&mut wtxn).expect("clear.put");
        wtxn.commit().expect("enable.commit");
    }

    pub(self) fn set_enable(&self, enable: bool) {
        let mut wtxn = self.env.write_txn().expect("enable.write_txn");
        self.enable
            .put(&mut wtxn, &RELI_ENABLE_KEY, &enable)
            .expect("enable.put");
        wtxn.commit().expect("enable.commit");
    }

    pub(self) fn enable(&self) -> bool {
        let rtxn = self.env.read_txn().expect("enable.read_txn");
        let enable = self.enable.get(&rtxn, &RELI_ENABLE_KEY).unwrap_or(None);
        enable.unwrap_or(false)
    }

    fn enable_len(&self) -> heed::Result<u64> {
        let rtxn = self.env.read_txn()?;
        self.enable.len(&rtxn)
    }
}

// last
const RELI_LAST_FILE: &str = "last.mdb";
const RELI_LAST_MAX_DBS: u32 = 7;
const RELI_DB_LUNIT: &str = "unit";
const RELI_DB_LFRAME: &str = "frame";
const RELI_DB_LFQUE: &str = "queue";
const RELI_DB_LFJOB: &str = "job";
const RELI_DB_LFSIGCHLD: &str = "sigchld";
const RELI_DB_LFCGEVENT: &str = "cgroup-event";
const RELI_DB_LFMNGOP: &str = "manager-operation";
static RELI_LAST_DB_NAME: [&str; RELI_LAST_MAX_DBS as usize] = [
    RELI_DB_LUNIT,
    RELI_DB_LFRAME,
    RELI_DB_LFQUE,
    RELI_DB_LFJOB,
    RELI_DB_LFSIGCHLD,
    RELI_DB_LFCGEVENT,
    RELI_DB_LFMNGOP,
];
const RELI_LAST_KEY: u32 = 0; // singleton
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ReliLastFrame {
    Queue,
    Job,
    SigChld,
    CgEvent,
    ManagerOp,
    CmdOp,
    OtherEvent,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(super) enum ReliLastQue {
    Load,
    GcJob,
    GcUnit,
    Clean,
    CgRealize,
    StartWhenUpheld,
    StopWhenBound,
    StopWhenUnneeded,
    Dbus,
}

struct ReliLast {
    // control
    ignore: RefCell<bool>,

    // data
    /* environment */
    env: Env,

    /* database: singleton(1) */
    unit: Database<OwnedType<u32>, Str>, // RELI_DB_LUNIT; key: RELI_LAST_KEY, data: unit_id;
    frame: Database<OwnedType<u32>, SerdeBincode<ReliLastFrame>>, // RELI_DB_LFRAME; key: RELI_LAST_KEY, data: ReliLastFrame;
    queue: Database<OwnedType<u32>, SerdeBincode<ReliLastQue>>, // RELI_DB_LFQUE; key: RELI_LAST_KEY, data: ReliLastQue;
}

impl fmt::Debug for ReliLast {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReliLast")
            .field("env.path", &self.env.path())
            .field("unit.len", &self.unit_len().unwrap_or(0))
            .field("frame.len", &self.unit_len().unwrap_or(0))
            .field("queue.len", &self.unit_len().unwrap_or(0))
            .finish()
    }
}

// the declaration "pub(self)" is for identification only.
impl ReliLast {
    pub(self) fn new() -> ReliLast {
        // init environment
        let path = Path::new(RELI_PATH_DIR).join(RELI_LAST_FILE);
        let env = EnvOpenOptions::new()
            .max_dbs(RELI_LAST_MAX_DBS)
            .open(path)
            .unwrap();

        // create db
        let unit = env.create_database(Some(RELI_DB_LUNIT)).unwrap();
        let frame = env.create_database(Some(RELI_DB_LFRAME)).unwrap();
        let queue = env.create_database(Some(RELI_DB_LFQUE)).unwrap();

        // return
        ReliLast {
            ignore: RefCell::new(false),
            env,
            unit,
            frame,
            queue,
        }
    }

    pub(self) fn clear(&self) {
        let mut wtxn = self.env.write_txn().expect("last.write_txn");
        self.unit.clear(&mut wtxn).expect("clear.put");
        self.frame.clear(&mut wtxn).expect("clear.put");
        self.queue.clear(&mut wtxn).expect("clear.put");
        wtxn.commit().expect("last.commit");
    }

    pub(self) fn ignore_set(&self, ignore: bool) {
        *self.ignore.borrow_mut() = ignore;
    }

    pub(self) fn set_unit(&self, unit_id: &str) {
        if self.ignore() {
            return;
        }

        let mut wtxn = self.env.write_txn().expect("last.write_txn");
        self.unit
            .put(&mut wtxn, &RELI_LAST_KEY, unit_id)
            .expect("last.put");
        wtxn.commit().expect("last.commit");
    }

    pub(self) fn clear_unit(&self) {
        if self.ignore() {
            return;
        }

        let mut wtxn = self.env.write_txn().expect("last.write_txn");
        self.unit
            .delete(&mut wtxn, &RELI_LAST_KEY)
            .expect("last.delete");
        wtxn.commit().expect("last.commit");
    }

    pub(self) fn set_frame(&self, frame: ReliLastFrame) {
        if self.ignore() {
            return;
        }

        let mut wtxn = self.env.write_txn().expect("last.write_txn");
        self.frame
            .put(&mut wtxn, &RELI_LAST_KEY, &frame)
            .expect("last.put");
        wtxn.commit().expect("last.commit");
    }

    pub(self) fn clear_frame(&self) {
        if self.ignore() {
            return;
        }

        let mut wtxn = self.env.write_txn().expect("last.write_txn");
        self.frame
            .delete(&mut wtxn, &RELI_LAST_KEY)
            .expect("last.delete");
        wtxn.commit().expect("last.commit");
    }

    pub(self) fn set_queue(&self, queue: ReliLastQue) {
        if self.ignore() {
            return;
        }

        let mut wtxn = self.env.write_txn().expect("last.write_txn");
        self.queue
            .put(&mut wtxn, &RELI_LAST_KEY, &queue)
            .expect("last.put");
        wtxn.commit().expect("last.commit");
    }

    pub(self) fn unit(&self) -> Option<String> {
        let rtxn = self.env.read_txn().expect("last.read_txn");
        let unit_id = self.unit.get(&rtxn, &RELI_LAST_KEY).unwrap_or(None);
        unit_id.map(|u| u.to_string())
    }

    pub(self) fn frame(&self) -> Option<ReliLastFrame> {
        let rtxn = self.env.read_txn().expect("last.read_txn");
        let frame = self.frame.get(&rtxn, &RELI_LAST_KEY).unwrap_or(None);
        frame
    }

    pub(self) fn queue(&self) -> Option<ReliLastQue> {
        let rtxn = self.env.read_txn().expect("last.read_txn");
        let queue = self.queue.get(&rtxn, &RELI_LAST_KEY).unwrap_or(None);
        queue
    }

    fn unit_len(&self) -> heed::Result<u64> {
        let rtxn = self.env.read_txn()?;
        self.unit.len(&rtxn)
    }

    fn frame_len(&self) -> heed::Result<u64> {
        let rtxn = self.env.read_txn()?;
        self.frame.len(&rtxn)
    }

    fn queue_len(&self) -> heed::Result<u64> {
        let rtxn = self.env.read_txn()?;
        self.queue.len(&rtxn)
    }

    fn ignore(&self) -> bool {
        *self.ignore.borrow()
    }
}

// entry
const RELI_ENTRY_FILE: &str = "entry.mdb";
const RELI_ENTRY_MAX_DBS: u32 = 11;
/* manager */
const RELI_DB_LMNG: &str = "manager";
/* job */
pub(super) const RELI_DB_EJOB_TRIGGER: &str = "jtrigger";
pub(super) const RELI_DB_EJOB_SUSPENDS: &str = "jsuspends";
/* unit */
pub(super) const RELI_DB_EUNIT_BASE: &str = "ubase";
pub(super) const RELI_DB_EUNIT_LOAD: &str = "uload";
pub(super) const RELI_DB_EUNIT_CONFIG: &str = "uconfig";
pub(super) const RELI_DB_EUNIT_CGROUP: &str = "ucgroup";
pub(super) const RELI_DB_EUNIT_CHILD: &str = "uchild";
pub(super) const RELI_DB_EUNIT_PPS: &str = "upps";
pub(super) const RELI_DB_EUNIT_DEP: &str = "udep";
/* service */
const RELI_DB_ESERVICE_CONF: &str = "svcconf";
const RELI_DB_ESERVICE_MNG: &str = "svcmng";
static RELI_ENTRY_DB_NAME: [&str; RELI_ENTRY_MAX_DBS as usize] = [
    RELI_DB_EJOB_TRIGGER,
    RELI_DB_EJOB_SUSPENDS,
    RELI_DB_EUNIT_BASE,
    RELI_DB_EUNIT_LOAD,
    RELI_DB_EUNIT_CONFIG,
    RELI_DB_EUNIT_CGROUP,
    RELI_DB_EUNIT_CHILD,
    RELI_DB_EUNIT_PPS,
    RELI_DB_EUNIT_DEP,
    RELI_DB_ESERVICE_CONF,
    RELI_DB_ESERVICE_MNG,
];

struct ReliEntry {
    // environment
    env: Env,

    // database: multi-instance(N)
    dbs: RefCell<HashMap<String, Rc<dyn ReDbTable>>>, // key: name, value: db
}

impl fmt::Debug for ReliEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReliEntry")
            .field("env.path", &self.env.path())
            .field("env.dbs.len", &self.dbs.borrow().len())
            .finish()
    }
}

// the declaration "pub(self)" is for identification only.
impl ReliEntry {
    pub(self) fn new() -> ReliEntry {
        // init environment
        let path = Path::new(RELI_PATH_DIR).join(RELI_ENTRY_FILE);
        let env = EnvOpenOptions::new()
            .max_dbs(RELI_ENTRY_MAX_DBS)
            .open(path)
            .unwrap();

        // return
        ReliEntry {
            env,
            dbs: RefCell::new(HashMap::new()),
        }
    }

    pub(self) fn clear(&self) {
        let mut db_wtxn = ReDbRwTxn::new(&self.env).expect("entry.write_txn");
        for (_, db) in self.dbs.borrow().iter() {
            db.clear(&mut db_wtxn);
        }
        db_wtxn.0.commit().expect("entry.commit");
    }

    pub(self) fn db_register(&self, name: &str, db: Rc<dyn ReDbTable>) {
        self.dbs.borrow_mut().insert(name.to_string(), db);
    }

    pub(self) fn commit(&self) {
        // create transaction
        let mut db_wtxn = ReDbRwTxn::new(&self.env).expect("entry.write_txn");

        // flush to db
        for (_, db) in self.dbs.borrow().iter() {
            db.export(&mut db_wtxn);
        }

        // commit
        db_wtxn.0.commit().expect("entry.commit");
    }

    pub(self) fn import(&self) {
        let db_rtxn = ReDbRoTxn::new(&self.env).expect("entry.write_txn");

        // import from db
        for (_, db) in self.dbs.borrow().iter() {
            db.import(&db_rtxn);
        }
    }

    pub(self) fn ignore_set(&self, ignore: bool) {
        // set ignore
        for (_, db) in self.dbs.borrow().iter() {
            db.ignore_set(ignore);
        }
    }

    pub(self) fn env(&self) -> &Env {
        &self.env
    }
}

pub struct ReDbRwTxn<'e, 'p>(RwTxn<'e, 'p>);

impl<'e, 'p> ReDbRwTxn<'e, 'p> {
    fn new(env: &'e Env) -> heed::Result<ReDbRwTxn> {
        env.write_txn().map(|w| ReDbRwTxn(w))
    }
}

pub struct ReDbRoTxn<'e>(RoTxn<'e>);

impl<'e> ReDbRoTxn<'e> {
    fn new(env: &'e Env) -> heed::Result<ReDbRoTxn> {
        env.read_txn().map(|r| ReDbRoTxn(r))
    }
}

pub trait ReDbTable {
    fn clear(&self, wtxn: &mut ReDbRwTxn);
    fn export(&self, wtxn: &mut ReDbRwTxn);
    fn import<'a>(&self, rtxn: &'a ReDbRoTxn);
    fn ignore_set(&self, ignore: bool);
}

pub struct ReDb<K, V> {
    // control
    ignore: RefCell<bool>,

    // data
    db: Database<SerdeBincode<K>, SerdeBincode<V>>,
    cache: RefCell<HashMap<K, V>>,
    add: RefCell<HashMap<K, V>>,
    del: RefCell<HashSet<K>>,
}

impl<K, V> ReDb<K, V>
where
    K: 'static + Serialize + Eq + Hash + Clone,
    V: 'static + Serialize + Clone,
{
    pub fn new(relir: &Reliability, name: &str) -> ReDb<K, V> {
        let db = relir.entry.env().create_database(Some(name)).unwrap();
        ReDb {
            ignore: RefCell::new(false),
            db,
            cache: RefCell::new(HashMap::new()),
            add: RefCell::new(HashMap::new()),
            del: RefCell::new(HashSet::new()),
        }
    }

    pub fn do_clear(&self, wtxn: &mut ReDbRwTxn) {
        self.db.clear(&mut wtxn.0).expect("entry.clear");
        self.cache.borrow_mut().clear();
        self.add.borrow_mut().clear();
        self.del.borrow_mut().clear();
    }

    pub fn set_ignore(&self, ignore: bool) {
        *self.ignore.borrow_mut() = ignore;
    }

    pub fn insert(&self, k: K, v: V) {
        if self.ignore() {
            return;
        }

        // remove "del" + insert "add"
        self.del.borrow_mut().remove(&k);
        self.add.borrow_mut().insert(k.clone(), v.clone());

        // update cache
        self.cache.borrow_mut().insert(k, v);
    }

    pub fn remove(&self, k: &K) {
        if self.ignore() {
            return;
        }

        // remove "add" + insert "del"
        self.add.borrow_mut().remove(k);
        self.del.borrow_mut().insert(k.clone());

        // update cache
        self.cache.borrow_mut().remove(k);
    }

    pub fn get(&self, k: &K) -> Option<V> {
        self.cache.borrow().get(k).cloned()
    }

    pub fn contains_key(&self, k: &K) -> bool {
        self.cache.borrow().contains_key(k)
    }

    pub fn keys(&self) -> Vec<K> {
        self.cache
            .borrow()
            .iter()
            .map(|(k, _)| k.clone())
            .collect::<_>()
    }

    pub fn entrys(&self) -> Vec<(K, V)> {
        self.cache
            .borrow()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<_>()
    }

    pub fn cache_2_db(&self, wtxn: &mut ReDbRwTxn) {
        // "add" -> db.put + clear "add"
        for (k, v) in self.add.borrow().iter() {
            self.db.put(&mut wtxn.0, k, v).expect("entry.put");
        }
        self.add.borrow_mut().clear();

        // "del" -> db.delete + clear "del"
        for k in self.del.borrow().iter() {
            self.db.delete(&mut wtxn.0, k).expect("entry.delete");
        }
        self.del.borrow_mut().clear();
    }

    pub fn db_2_cache<'a>(&self, rtxn: &'a ReDbRoTxn)
    where
        K: Deserialize<'a>,
        V: Deserialize<'a>,
    {
        // clear "add" + "del" + "cache"
        self.add.borrow_mut().clear();
        self.del.borrow_mut().clear();
        self.cache.borrow_mut().clear();

        // db -> cache
        let iter = self.db.iter(&rtxn.0).unwrap();
        for entry in iter {
            let (k, v) = entry.unwrap();
            self.cache.borrow_mut().insert(k, v);
        }
    }

    fn ignore(&self) -> bool {
        *self.ignore.borrow()
    }
}

// station
struct ReliStation {
    t_name: RefCell<HashMap<String, Rc<dyn ReStation>>>, // key: name, value: station
    t_kind: RefCell<HashMap<ReStationKind, Vec<Rc<dyn ReStation>>>>, // key: kind, value: stations
}

impl fmt::Debug for ReliStation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReliLast")
            .field("stations.len", &self.t_name.borrow().len())
            .finish()
    }
}

// the declaration "pub(self)" is for identification only.
impl ReliStation {
    pub(self) fn new() -> ReliStation {
        ReliStation {
            t_name: RefCell::new(HashMap::new()),
            t_kind: RefCell::new(HashMap::new()),
        }
    }

    pub(self) fn station_register(
        &self,
        name: &str,
        kind: ReStationKind,
        station: Rc<dyn ReStation>,
    ) {
        let sta = Rc::clone(&station);
        if let None = self.t_name.borrow_mut().insert(name.to_string(), station) {
            // new, update kind-table
            let mut stations = self.t_kind.borrow_mut().remove(&kind).unwrap_or(Vec::new());
            stations.push(sta);
        }
    }

    pub(self) fn input_rebuild(&self) {
        for (_, station) in self.t_name.borrow().iter() {
            station.input_rebuild();
        }
    }

    pub(self) fn entry_compensate(&self, lframe: Option<ReliLastFrame>, lunit: Option<String>) {
        // entry first, dealing with history records.
        for (_, station) in self.t_name.borrow().iter() {
            station.db_compensate_history();
        }

        // last second, dealing with last words, which could produce histroy records.
        if lframe.is_some() {
            let lf = lframe.unwrap();
            let lu = lunit.as_ref();
            for (_, station) in self.t_name.borrow().iter() {
                station.db_compensate_last(lf, lu);
            }
        }
    }

    pub(self) fn entry_map(&self) {
        // unit
        for station in self.get_kind(ReStationKind::Unit).iter() {
            station.db_map();
        }

        // manager
        for station in self.get_kind(ReStationKind::Manager).iter() {
            station.db_map();
        }
    }

    pub(self) fn make_consistent(&self, lframe: Option<ReliLastFrame>, lunit: Option<String>) {
        let lu = lunit.as_ref();

        // last first, dealing with more untrusted infomation.
        if lframe.is_some() {
            let lf = lframe.unwrap();
            for (_, station) in self.t_name.borrow().iter() {
                station.do_compensate_last(lf, lu);
            }
        }

        // other secondj, dealing with less untrusted infomation.
        for (_, station) in self.t_name.borrow().iter() {
            station.do_compensate_others(lu);
        }
    }

    fn get_kind(&self, kind: ReStationKind) -> Vec<Rc<dyn ReStation>> {
        self.t_kind
            .borrow()
            .get(&kind)
            .cloned()
            .unwrap_or(Vec::new())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ReStationKind {
    Unit,
    Manager,
}

pub trait ReStation {
    fn input_rebuild(&self);

    // process reentrant
    fn db_compensate_last(&self, _lframe: ReliLastFrame, _lunit: Option<&String>);

    // process reentrant
    fn db_compensate_history(&self);

    fn db_map(&self);

    // process reentrant
    fn do_compensate_last(&self, _lframe: ReliLastFrame, _lunit: Option<&String>);

    // process reentrant
    fn do_compensate_others(&self, _lunit: Option<&String>);
}

fn reli_prepare() -> Result<(), Error> {
    // directory
    let dir = Path::new(RELI_PATH_DIR);
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }

    // enable
    let enable = dir.join(RELI_ENABLE_FILE);
    if !enable.exists() {
        fs::create_dir_all(&enable)?;
    }

    // last
    let last = dir.join(RELI_LAST_FILE);
    if !last.exists() {
        fs::create_dir_all(&last)?;
    }

    // entry
    let entry = dir.join(RELI_ENTRY_FILE);
    if !entry.exists() {
        fs::create_dir_all(&entry)?;
    }

    Ok(())
}
