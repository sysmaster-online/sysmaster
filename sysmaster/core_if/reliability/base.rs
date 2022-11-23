use heed::{Env, RoTxn, RwTxn};
use serde::de::DeserializeOwned;
use std::env;
use std::fs;
use std::io::{Error, ErrorKind};
use std::path::Path;

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use heed::types::SerdeBincode;
use heed::Database;
use serde::Serialize;
use std::fmt::Debug;
use std::hash::Hash;

use super::Reliability;

/// the reliability database
/// K & V that can be deserialized without borrowing any data from the deserializer.
pub struct ReDb<K, V> {
    // control
    ignore: RefCell<bool>,

    // data
    db: Database<SerdeBincode<K>, SerdeBincode<V>>,
    cache: RefCell<HashMap<K, V>>,
    add: RefCell<HashMap<K, V>>,
    del: RefCell<HashSet<K>>,
    name: String,
    //_phantom: PhantomData<&'a K>,
}

impl<K, V> ReDbTable for ReDb<K, V>
where
    K: 'static + Serialize + Eq + Hash + Clone + Debug + DeserializeOwned,
    V: 'static + Serialize + Clone + Debug + DeserializeOwned,
{
    fn clear(&self, wtxn: &mut ReDbRwTxn) {
        self.do_clear(wtxn);
    }

    fn export(&self, db_wtxn: &mut ReDbRwTxn) {
        self.cache_2_db(db_wtxn);
    }

    fn import(&self, db_rtxn: &ReDbRoTxn) {
        self.db_2_cache(db_rtxn);
    }

    fn ignore_set(&self, ignore: bool) {
        self.set_ignore(ignore);
    }
}

impl<K, V> ReDb<K, V>
where
    K: 'static + Serialize + Eq + Hash + Clone + Debug,
    V: 'static + Serialize + Clone + Debug,
{
    /// create reliability database instance
    pub fn new(relir: &Reliability, db_name: &str) -> ReDb<K, V> {
        let db = relir.create_database(Some(db_name)).unwrap();
        ReDb {
            ignore: RefCell::new(false),
            db,
            cache: RefCell::new(HashMap::new()),
            add: RefCell::new(HashMap::new()),
            del: RefCell::new(HashSet::new()),
            name: String::from(db_name),
            //_phantom: PhantomData,
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

        let n = &self.name;
        log::debug!("ReDb[{}] insert, key: {:?}, value: {:?}.", n, &k, &v);

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

        log::debug!("ReDb[{}] remove, key: {:?}.", &self.name, &k);

        // remove "add" + insert "del"
        self.add.borrow_mut().remove(k);
        self.del.borrow_mut().insert(k.clone());

        // update cache
        self.cache.borrow_mut().remove(k);
    }

    /// get a entry
    pub fn get(&self, k: &K) -> Option<V> {
        let value = self.cache.borrow().get(k).cloned();
        let n = &self.name;
        log::debug!("ReDb[{}] get, key: {:?}, value: {:?}.", n, k, &value);
        value
    }

    /// get the existence of the key
    pub fn contains_key(&self, k: &K) -> bool {
        self.cache.borrow().contains_key(k)
    }

    /// get all keys
    pub fn keys(&self) -> Vec<K> {
        let keys = self
            .cache
            .borrow()
            .iter()
            .map(|(k, _)| k.clone())
            .collect::<_>();
        log::debug!("ReDb[{}] keys, keys: {:?}.", &self.name, &keys);
        keys
    }

    /// get all entries
    pub fn entries(&self) -> Vec<(K, V)> {
        let entries = self
            .cache
            .borrow()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<_>();
        log::debug!("ReDb[{}] entries, entries: {:?}.", &self.name, &entries);
        entries
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
    pub fn db_2_cache(&self, rtxn: &ReDbRoTxn)
    where
        K: DeserializeOwned,
        V: DeserializeOwned,
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

/// reliability writeable transaction
pub struct ReDbRwTxn<'e, 'p>(pub(super) RwTxn<'e, 'p>);

impl<'e, 'p> ReDbRwTxn<'e, 'p> {
    pub(super) fn new(env: &'e Env) -> heed::Result<ReDbRwTxn> {
        env.write_txn().map(ReDbRwTxn)
    }
}

/// reliability read-only transaction
pub struct ReDbRoTxn<'e>(pub(super) RoTxn<'e>);

impl<'e> ReDbRoTxn<'e> {
    pub(super) fn new(env: &'e Env) -> heed::Result<ReDbRoTxn> {
        env.read_txn().map(ReDbRoTxn)
    }
}

/// reliability data table
pub trait ReDbTable {
    /// clear all data
    fn clear(&self, wtxn: &mut ReDbRwTxn);
    /// export all data to database
    fn export(&self, wtxn: &mut ReDbRwTxn);
    /// import all data from database
    fn import<'a>(&self, rtxn: &'a ReDbRoTxn);
    /// set the ignore flag of data
    fn ignore_set(&self, ignore: bool);
}

const RELI_PATH_DIR: &str = "/run/systemd/reliability";

pub(super) fn reli_dir_get() -> Result<String, Error> {
    // /run/systemd/reliability/
    let ret_run = reli_dir_get_run();
    if ret_run.is_ok() {
        return ret_run; // ok
    }

    // OUT_DIR/../
    let ret_out = reli_dir_get_out();
    if ret_out.is_ok() {
        return ret_out; // ok
    }

    // PROCESS_RELI_PATH
    let ret_customize = reli_dir_get_customize();
    if ret_customize.is_ok() {
        return ret_customize; // ok
    }

    // nothing exists, return failure.
    Err(Error::from(ErrorKind::NotFound))
}

/// prepare the directory for reliability.
/// the reliability path is prepared and searched according to the following priority, from high to low:
/// 1. /run/systemd/reliability/: the real running directory.
/// 2. OUT_DIR/../reliability/: make CI happy, which is target/debug/reliability/ or target/release/reliability/ usually.
/// 3. PROCESS_RELI_PATH: the path customized.
pub fn reli_dir_prepare() -> Result<(), Error> {
    // /run/systemd/reliability/
    let ret_run = reli_dir_prepare_run();
    if ret_run.is_ok() {
        return ret_run; // ok
    }
    let err_run = ret_run.unwrap_err();

    // OUT_DIR/../
    let ret_out = reli_dir_prepare_out();
    if ret_out.is_ok() {
        return ret_out; // ok
    }
    let err_out = ret_out.unwrap_err();

    // PROCESS_RELI_PATH
    let ret_customize = reli_dir_prepare_customize();
    if ret_customize.is_ok() {
        return ret_customize; // ok
    }
    let err_customize = ret_customize.unwrap_err();

    // nothing has been prepared, return failure.
    if err_customize.kind() != ErrorKind::NotFound {
        Err(err_customize)
    } else if err_out.kind() != ErrorKind::NotFound {
        Err(err_out)
    } else {
        Err(err_run)
    }
}

fn reli_dir_prepare_run() -> Result<(), Error> {
    let dir = Path::new(RELI_PATH_DIR);
    if !dir.exists() {
        fs::create_dir_all(dir)?;
    }

    log::info!(
        "prepare reliability running directory successfully: {}.",
        RELI_PATH_DIR
    );
    Ok(())
}

fn reli_dir_get_run() -> Result<String, Error> {
    let dir = Path::new(RELI_PATH_DIR);
    if dir.exists() {
        log::info!(
            "get reliability run directory successfully: {}.",
            RELI_PATH_DIR
        );
        Ok(String::from(RELI_PATH_DIR))
    } else {
        Err(Error::from(ErrorKind::NotFound))
    }
}

fn reli_dir_prepare_out() -> Result<(), Error> {
    let dir_string = out_dir_string_get();
    if let Some(d_str) = dir_string {
        let dir = Path::new(&d_str);
        if !dir.exists() {
            fs::create_dir_all(dir)?;
        }

        log::info!("prepare reliability out directory successfully: {}.", d_str);
        Ok(())
    } else {
        Err(Error::from(ErrorKind::NotFound))
    }
}

fn reli_dir_get_out() -> Result<String, Error> {
    let dir_string = out_dir_string_get();
    if let Some(d_str) = dir_string {
        let dir = Path::new(&d_str);
        if dir.exists() {
            log::info!("get reliability out directory successfully: {}.", d_str);
            Ok(d_str)
        } else {
            Err(Error::from(ErrorKind::NotFound))
        }
    } else {
        Err(Error::from(ErrorKind::NotFound))
    }
}

fn reli_dir_prepare_customize() -> Result<(), Error> {
    let dir_string = env::var("PROCESS_LIB_LOAD_PATH").ok();
    if let Some(d_str) = dir_string {
        let dir = Path::new(&d_str);
        if !dir.exists() {
            fs::create_dir_all(dir)?;
        }

        log::info!(
            "prepare reliability customized directory successfully: {}.",
            d_str
        );
        Ok(())
    } else {
        Err(Error::from(ErrorKind::NotFound))
    }
}

fn reli_dir_get_customize() -> Result<String, Error> {
    let dir_string = env::var("PROCESS_LIB_LOAD_PATH").ok();
    if let Some(d_str) = dir_string {
        let dir = Path::new(&d_str);
        if dir.exists() {
            log::info!(
                "get reliability customized directory successfully: {}.",
                d_str
            );
            Ok(d_str)
        } else {
            Err(Error::from(ErrorKind::NotFound))
        }
    } else {
        Err(Error::from(ErrorKind::NotFound))
    }
}

fn out_dir_string_get() -> Option<String> {
    let run = env::var("OUT_DIR").ok();
    let compile: Option<String> = option_env!("OUT_DIR").map(String::from);
    run.or(compile).map(|dir_string| {
        if dir_string.contains("build") {
            let _tmp: Vec<_> = dir_string.split("build").collect();
            format!("{}reliability", _tmp[0])
        } else {
            format!("{}reliability", dir_string)
        }
    })
}
