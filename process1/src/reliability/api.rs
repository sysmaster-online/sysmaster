use super::base::{self, ReDbRoTxn, ReDbRwTxn, ReDbTable};
use super::enable::{self, ReliEnable};
use super::history::{self, ReliHistory};
use super::last::{self, ReliLast};
use super::pending::{self, ReliPending};
use super::station::{ReStation, ReStationKind, ReliStation};
use heed::types::SerdeBincode;
use heed::Database;
use nix::errno::Errno;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::hash::Hash;
use std::io::Error;
use std::path::Path;
use std::rc::Rc;
use std::thread;
use std::time::Duration;

const RELI_DEBUG_SWITCH_FILE: &str = "switch.debug";
const RELI_DEBUG_CLEAR_FILE: &str = "clear.debug";
const RELI_DEBUG_CFIRST_FILE: &str = "clear_first.debug";
const RELI_DEBUG_ENABLE_FILE: &str = "enable.debug";
const RELI_DEBUG_PANIC_FILE: &str = "panic.debug";
const RELI_DEBUG_PFIRST_FILE: &str = "panic_first.debug";
const RELI_DEBUG_SLEEP_FILE: &str = "sleep.debug";

/// reliability instance
#[derive(Debug)]
pub struct Reliability {
    // control data
    dir_string: String,
    enable: ReliEnable,

    // output data
    last: ReliLast,
    history: ReliHistory,
    pending: ReliPending,

    // input & recover
    station: ReliStation,
}

impl Drop for Reliability {
    fn drop(&mut self) {
        log::debug!("Reliability drop, clear.");
        // repeating protection
        self.clear();
    }
}

impl Reliability {
    /// create reliability instance
    pub fn new(max_db: u32) -> Reliability {
        let dir_s = reli_prepare().expect("reliability prepare");
        let reli = Reliability {
            dir_string: dir_s.clone(),
            enable: ReliEnable::new(&dir_s),
            last: ReliLast::new(&dir_s),
            history: ReliHistory::new(&dir_s, max_db),
            pending: ReliPending::new(&dir_s),
            station: ReliStation::new(),
        };
        reli.debug_enable();
        reli
    }

    /// set the enable flag
    pub fn set_enable(&self, enable: bool) {
        self.enable.set_enable(enable);
    }

    /// set the last unit
    pub fn set_last_unit(&self, unit_id: &str) {
        self.last.set_unit(unit_id);
    }

    /// clear the last unit
    pub fn clear_last_unit(&self) {
        self.history.commit();
        self.last.clear_unit();
    }

    /// set the last frame
    pub fn set_last_frame(&self, f1: u32, f2: Option<u32>, f3: Option<u32>) {
        self.last.set_frame(f1, f2, f3);
    }

    /// set the last frame with just one parameter
    pub fn set_last_frame1(&self, f1: u32) {
        self.last.set_frame(f1, None, None);
    }

    /// set the last frame with two parameters
    pub fn set_last_frame2(&self, f1: u32, f2: u32) {
        self.last.set_frame(f1, Some(f2), None);
    }

    /// clear the last frame
    pub fn clear_last_frame(&self) {
        self.history.commit();
        self.last.clear_frame();
    }

    /// register history database
    pub fn history_db_register(&self, name: &str, db: Rc<dyn ReDbTable>) {
        self.history.db_register(name, db);
    }

    /// set the fd's 'cloexec' flag and record it
    pub fn fd_cloexec(&self, fd: i32, cloexec: bool) -> Result<(), Errno> {
        self.pending.fd_cloexec(fd, cloexec)
    }

    /// take the fd away
    pub fn fd_take(&self, fd: i32) -> i32 {
        self.pending.fd_take(fd)
    }

    /// register a station
    pub fn station_register(&self, name: &str, kind: ReStationKind, station: Rc<dyn ReStation>) {
        self.station.station_register(name, kind, station);
    }

    /// [process reentrant] recover the data
    pub fn recover(&self) {
        self.debug_clear();

        // ignore last's input
        self.last.ignore_set(true);

        self.history.import();
        self.input_rebuild();
        self.db_compensate();
        self.db_map();
        self.make_consistent();

        // restore last's ignore
        self.last.ignore_set(false);

        // clear last
        self.last.clear_unit();
        self.last.clear_frame();
    }

    /// get the enable flag
    pub fn enable(&self) -> bool {
        self.enable.enable()
    }

    /// get the last unit
    pub fn last_unit(&self) -> Option<String> {
        self.last.unit()
    }

    /// get the last frame
    pub fn last_frame(&self) -> Option<(u32, Option<u32>, Option<u32>)> {
        self.last.frame()
    }

    /// clear all data
    pub fn data_clear(&self) {
        // data-only
        /* contrl */
        self.enable.data_clear();
        /* output */
        self.last.data_clear();
        self.history.data_clear();
        self.pending.data_clear();
    }

    /// [repeating protection] clear all registers
    pub fn clear(&self) {
        self.history.clear();
        self.station.clear();
    }

    /// get the ignore flag of last data
    pub fn last_ignore(&self) -> bool {
        self.last.ignore()
    }

    /// get the ignore flag of history data
    pub fn history_ignore(&self) -> bool {
        self.history.ignore()
    }

    /// do the debug action: enable the recover process
    pub fn debug_enable(&self) {
        let enable = Path::new(&self.dir_string).join(RELI_DEBUG_ENABLE_FILE);
        if enable.exists() {
            log::info!("reliability debug: enable data...");
            self.set_enable(true);
        }
    }

    /// do the debug action: clear data excluding enable
    pub fn debug_clear(&self) {
        let clear = Path::new(&self.dir_string).join(RELI_DEBUG_CLEAR_FILE);
        if clear.exists() {
            log::info!("reliability debug: clear data...");
            let cfirst = Path::new(&self.dir_string).join(RELI_DEBUG_CFIRST_FILE);
            if cfirst.exists() {
                // do nothing
                log::info!("reliability debug_clear: non-first time, do nothing.");
            } else {
                log::info!("reliability debug_clear: first time, try clear.");
                File::create(&cfirst).unwrap();
                log::info!("reliability debug_clear: first time, clear ...");

                // clear data excluding enable
                let enable = self.enable();
                self.data_clear();
                self.set_enable(enable);
            }
        }
    }

    /// do the debug action: panic
    pub fn debug_panic(&self) {
        let panic = Path::new(&self.dir_string).join(RELI_DEBUG_PANIC_FILE);
        if panic.exists() {
            log::info!("reliability debug: panic...");
            let pfirst = Path::new(&self.dir_string).join(RELI_DEBUG_PFIRST_FILE);
            if pfirst.exists() {
                // do nothing
                log::info!("reliability debug_panic: non-first time, do nothing.");
            } else {
                log::info!("reliability debug_panic: first time, try panic.");
                File::create(&pfirst).unwrap();
                log::info!("reliability debug_panic: first time, panic ...");
                panic!("first debug_panic.");
            }
        }
    }

    /// do the debug action: sleep
    pub fn debug_sleep(&self) {
        let sleep = Path::new(&self.dir_string).join(RELI_DEBUG_SLEEP_FILE);
        if sleep.exists() {
            log::info!("reliability debug: sleep...");
            thread::sleep(Duration::from_secs(3600));
        }
    }

    fn input_rebuild(&self) {
        // ignore history's input
        self.history.ignore_set(true);

        self.station.input_rebuild();

        // restore history's ignore
        self.history.ignore_set(false);
    }

    fn db_compensate(&self) {
        // get last
        let lframe = self.last_frame();
        let lunit = self.last_unit();

        log::debug!("db_compensate, lframe:{:?}, lunit{:?}.", lframe, lunit);
        self.station.db_compensate(lframe, lunit);
        self.history.commit();
    }

    fn db_map(&self) {
        // ignore history's input
        self.history.ignore_set(true);

        self.station.db_map();

        // restore history's ignore
        self.history.ignore_set(false);
    }

    fn make_consistent(&self) {
        // get last
        let lframe = self.last_frame();
        let lunit = self.last_unit();

        self.pending.make_consistent();
        self.station.make_consistent(lframe, lunit);
        self.history.commit();
    }
}

/// the reliability database
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
    /// create reliability database instance
    pub fn new(relir: &Reliability, name: &str) -> ReDb<K, V> {
        let db = relir.history.env().create_database(Some(name)).unwrap();
        ReDb {
            ignore: RefCell::new(false),
            db,
            cache: RefCell::new(HashMap::new()),
            add: RefCell::new(HashMap::new()),
            del: RefCell::new(HashSet::new()),
        }
    }

    /// clear all data
    pub fn do_clear(&self, wtxn: &mut ReDbRwTxn) {
        self.db.clear(&mut wtxn.0).expect("history.clear");
        self.cache.borrow_mut().clear();
        self.add.borrow_mut().clear();
        self.del.borrow_mut().clear();
    }

    /// set the ignore flag of data
    pub fn set_ignore(&self, ignore: bool) {
        *self.ignore.borrow_mut() = ignore;
    }

    /// insert a entry
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

    /// remove a entry
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

    /// get a entry
    pub fn get(&self, k: &K) -> Option<V> {
        self.cache.borrow().get(k).cloned()
    }

    /// get the existance of the key
    pub fn contains_key(&self, k: &K) -> bool {
        self.cache.borrow().contains_key(k)
    }

    /// get all keys
    pub fn keys(&self) -> Vec<K> {
        self.cache
            .borrow()
            .iter()
            .map(|(k, _)| k.clone())
            .collect::<_>()
    }

    /// get all entrys
    pub fn entries(&self) -> Vec<(K, V)> {
        self.cache
            .borrow()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<_>()
    }

    /// export all data from cache to database
    pub fn cache_2_db(&self, wtxn: &mut ReDbRwTxn) {
        // "add" -> db.put + clear "add"
        for (k, v) in self.add.borrow().iter() {
            self.db.put(&mut wtxn.0, k, v).expect("history.put");
        }
        self.add.borrow_mut().clear();

        // "del" -> db.delete + clear "del"
        for k in self.del.borrow().iter() {
            self.db.delete(&mut wtxn.0, k).expect("history.delete");
        }
        self.del.borrow_mut().clear();
    }

    /// emport all data from database to cache
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

/// do the debug action: enable or disable switch flag. effective after restart.
#[allow(dead_code)]
pub fn reli_debug_enable_switch(enable: bool) -> Result<(), Error> {
    log::info!("reliability debug: enable[{}] switch.", enable);

    // [enable]touch switch.debug or [disable]rm -rf switch.debug
    let dir_string = base::reli_dir_get().unwrap();
    let switch = Path::new(&dir_string).join(RELI_DEBUG_SWITCH_FILE);
    if enable {
        if !switch.exists() {
            File::create(&switch)?;
        }
    } else if switch.exists() {
        fs::remove_file(&switch)?;
    }

    Ok(())
}

/// get the debug flag of switch
pub fn reli_debug_get_switch() -> bool {
    let dir_string = base::reli_dir_get().unwrap();
    let switch = Path::new(&dir_string).join(RELI_DEBUG_SWITCH_FILE);
    log::info!("reliability debug: get switch file: {:?}.", switch);
    switch.exists()
}

fn reli_prepare() -> Result<String, Error> {
    // directory
    base::reli_dir_prepare()?; // again
    let dir_string = base::reli_dir_get().unwrap();

    // enable
    enable::prepare(&dir_string)?;

    // last
    last::prepare(&dir_string)?;

    // history
    history::prepare(&dir_string)?;

    // pending
    pending::prepare(&dir_string)?;

    Ok(dir_string)
}
