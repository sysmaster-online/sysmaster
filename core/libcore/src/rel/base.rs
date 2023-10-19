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

use super::Reliability;
use crate::error::*;
use heed::types::SerdeBincode;
use heed::{Database, Env, RoTxn, RwTxn};
use nix::sys::stat::{self, Mode};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
#[cfg(debug)]
use std::env;
use std::fmt::Debug;
use std::fs;
use std::hash::Hash;
use std::path::Path;
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
/// the switch of the reliability database, which control the caching behavior.
pub enum ReliSwitch {
    /// record to cache, including variable(add + del) and snapshot(snapshot)
    CacheAll,
    /// record to variable cache(add + del) only
    CacheVar,
    /// record to buffer
    Buffer,
    /// ignore the input
    Ignore,
}

/// the reliability database
/// K & V that can be deserialized without borrowing any data from the deserializer.
pub struct ReDb<K, V> {
    // associated objects
    reli: Rc<Reliability>,

    // control
    switch: RefCell<ReliSwitch>,

    // data
    /* database: create in use */
    /* db: Database<SerdeBincode<K>, SerdeBincode<V>>; */

    /* cache */
    snapshot: RefCell<HashMap<K, V>>, // the copy of db
    add: RefCell<HashMap<K, V>>,
    del: RefCell<HashSet<K>>,

    /* buffer */
    buffer: RefCell<HashMap<K, V>>, // daemon-reload or daemon-reexec will temporarily store the data here first, and finally refreshes it to db.

    /* property */
    name: String,
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

    fn flush(&self, db_wtxn: &mut ReDbRwTxn, switch: ReliSwitch) {
        self.data_2_db(db_wtxn, switch);
    }

    fn import(&self) {
        self.db_2_cache();
    }

    fn switch_set(&self, switch: ReliSwitch) {
        self.switch_buffer(switch);
    }
}

impl<K, V> ReDb<K, V>
where
    K: 'static + Serialize + Eq + Hash + Clone + Debug,
    V: 'static + Serialize + Clone + Debug,
{
    /// create reliability database instance
    pub fn new(relir: &Rc<Reliability>, db_name: &str) -> ReDb<K, V> {
        ReDb {
            reli: Rc::clone(relir),
            switch: RefCell::new(ReliSwitch::CacheAll),
            snapshot: RefCell::new(HashMap::new()),
            add: RefCell::new(HashMap::new()),
            del: RefCell::new(HashSet::new()),
            buffer: RefCell::new(HashMap::new()),
            name: String::from(db_name),
        }
    }

    /// clear all data
    pub fn do_clear(&self, wtxn: &mut ReDbRwTxn) {
        if let Ok(db) = self.open_db(wtxn) {
            db.clear(&mut wtxn.0).expect("history.clear");
        }
        self.add.borrow_mut().clear();
        self.del.borrow_mut().clear();
        // Do not clear the snapshot and buffer, because their data are transient.
    }

    /// switch between cache and buffer
    pub fn switch_buffer(&self, switch: ReliSwitch) {
        // Before using the buffer, data needs to be cleared.
        self.buffer.borrow_mut().clear();
        *self.switch.borrow_mut() = switch;
    }

    /// insert a entry
    pub fn insert(&self, k: K, v: V) {
        let switch = self.switch();
        log::debug!(
            "ReDb[{}] insert, key: {:?}, value: {:?}, switch: {:?}.",
            &self.name,
            &k,
            &v,
            switch
        );

        match switch {
            ReliSwitch::CacheAll => {
                // remove "del" + insert "add"
                self.del.borrow_mut().remove(&k);
                self.add.borrow_mut().insert(k.clone(), v.clone());

                // update snapshot
                self.snapshot.borrow_mut().insert(k, v);
            }
            ReliSwitch::CacheVar => {
                // remove "del" + insert "add"
                self.del.borrow_mut().remove(&k);
                self.add.borrow_mut().insert(k, v);
            }
            ReliSwitch::Buffer => {
                // update buffer only
                self.buffer.borrow_mut().insert(k, v);
            }
            ReliSwitch::Ignore => {
                // do nothing
            }
        }
    }

    /// remove a entry
    pub fn remove(&self, k: &K) {
        let switch = self.switch();
        log::debug!("ReDb[{}] remove, key:{:?}.", &self.name, &k);
        log::debug!("remove with switch:{:?}.", switch);

        match switch {
            ReliSwitch::CacheAll => {
                // remove "add" + insert "del"
                self.add.borrow_mut().remove(k);
                self.del.borrow_mut().insert(k.clone());

                // update snapshot
                self.snapshot.borrow_mut().remove(k);
            }
            ReliSwitch::CacheVar => {
                // remove "add" + insert "del"
                self.add.borrow_mut().remove(k);
                self.del.borrow_mut().insert(k.clone());
            }
            ReliSwitch::Buffer => {
                // update buffer only
                self.buffer.borrow_mut().remove(k);
            }
            ReliSwitch::Ignore => {
                // do nothing
            }
        }
    }

    /// get a entry
    pub fn get(&self, k: &K) -> Option<V> {
        let value = self.snapshot.borrow().get(k).cloned();
        let n = &self.name;
        log::debug!("ReDb[{}] get, key: {:?}, value: {:?}.", n, k, &value);
        value
    }

    /// get all keys
    pub fn keys(&self) -> Vec<K> {
        let keys = self
            .snapshot
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
            .snapshot
            .borrow()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<_>();
        log::debug!("ReDb[{}] entries, entries: {:?}.", &self.name, &entries);
        entries
    }

    /// export changed data from cache to database
    pub fn cache_2_db(&self, wtxn: &mut ReDbRwTxn) {
        let db = self.open_db(wtxn).unwrap();

        // "add" -> db.put + clear "add"
        for (k, v) in self.add.borrow().iter() {
            db.put(&mut wtxn.0, k, v).expect("history.put");
        }
        self.add.borrow_mut().clear();

        // "del" -> db.delete + clear "del"
        for k in self.del.borrow().iter() {
            db.delete(&mut wtxn.0, k).expect("history.delete");
        }
        self.del.borrow_mut().clear();
    }

    /// flush internal data to database
    pub fn data_2_db(&self, wtxn: &mut ReDbRwTxn, switch: ReliSwitch) {
        match switch {
            ReliSwitch::CacheAll | ReliSwitch::CacheVar => {
                // clear "snapshot" only, which is the same with db.
                self.snapshot.borrow_mut().clear();
            }
            ReliSwitch::Buffer => {
                // clear all data, including "db" and "add" + "del"
                self.do_clear(wtxn);

                // "buffer" -> db.put + clear "buffer"
                let db = self.open_db(wtxn).unwrap();
                for (k, v) in self.buffer.borrow().iter() {
                    db.put(&mut wtxn.0, k, v).expect("history.put");
                }
                self.buffer.borrow_mut().clear();
            }
            ReliSwitch::Ignore => {
                // do nothing
            }
        }
    }

    /// import all data from database to cache
    pub fn db_2_cache(&self)
    where
        K: DeserializeOwned,
        V: DeserializeOwned,
    {
        // clear "add" + "del" + "snapshot"
        self.add.borrow_mut().clear();
        self.del.borrow_mut().clear();
        self.snapshot.borrow_mut().clear();

        // db(open only) -> snapshot
        let env = self.reli.env().expect("get env");
        if let Some(db) = env
            .open_database::<SerdeBincode<K>, SerdeBincode<V>>(Some(&self.name))
            .unwrap_or(None)
        {
            let rtxn = ReDbRoTxn::new(&env).expect("db_2_cache.ro_txn");
            let iter = db.iter(&rtxn.0).unwrap();
            for entry in iter {
                let (k, v) = entry.unwrap();
                self.snapshot.borrow_mut().insert(k, v);
            }
        }
    }

    fn open_db(&self, wtxn: &mut ReDbRwTxn) -> Result<Database<SerdeBincode<K>, SerdeBincode<V>>> {
        let env = self.reli.env().expect("get env");
        let database = env.open_database(Some(&self.name)).context(HeedSnafu)?;
        if let Some(db) = database {
            Ok(db)
        } else {
            env.create_database_with_txn(Some(&self.name), &mut wtxn.0)
                .context(HeedSnafu)
        }
    }

    fn switch(&self) -> ReliSwitch {
        *self.switch.borrow()
    }
}

/// reliability writeable transaction
pub struct ReDbRwTxn<'e, 'p>(pub RwTxn<'e, 'p>);

impl<'e, 'p> ReDbRwTxn<'e, 'p> {
    ///
    pub fn new(env: &'e Env) -> heed::Result<ReDbRwTxn> {
        env.write_txn().map(ReDbRwTxn)
    }
}

/// reliability read-only transaction
pub struct ReDbRoTxn<'e>(pub RoTxn<'e>);

impl<'e> ReDbRoTxn<'e> {
    ///
    pub fn new(env: &'e Env) -> heed::Result<ReDbRoTxn> {
        env.read_txn().map(ReDbRoTxn)
    }
}

/// reliability data table
pub trait ReDbTable {
    /// clear all data
    fn clear(&self, wtxn: &mut ReDbRwTxn);
    /// export the changed data to database
    fn export(&self, wtxn: &mut ReDbRwTxn);
    /// flush data to database
    fn flush(&self, wtxn: &mut ReDbRwTxn, switch: ReliSwitch);
    /// import all data from database
    fn import(&self);
    /// set the switch flag of data, does switch control whether to use buffer, cache, or none
    fn switch_set(&self, switch: ReliSwitch);
}

pub(super) const RELI_DIR: &str = "reliability.mdb";
#[allow(dead_code)]
pub(super) const RELI_DATA_FILE: &str = "data.mdb";
#[allow(dead_code)]
pub(super) const RELI_LOCK_FILE: &str = "lock.mdb";

pub(super) const RELI_INTERNAL_DB_ENABLE: &str = "enable";
pub(super) const RELI_INTERNAL_DB_LUNIT: &str = "unit";
pub(super) const RELI_INTERNAL_DB_LFRAME: &str = "frame";
pub(super) const RELI_INTERNAL_DB_PFD: &str = "fd";
pub(super) const RELI_INTERNAL_MAX_DBS: u32 = 4;
#[allow(dead_code)]
static RELI_INTERNAL_DB_NAME: [&str; RELI_INTERNAL_MAX_DBS as usize] = [
    RELI_INTERNAL_DB_ENABLE,
    RELI_INTERNAL_DB_LUNIT,
    RELI_INTERNAL_DB_LFRAME,
    RELI_INTERNAL_DB_PFD,
];

const RELI_PATH_DIR: &str = "/run/sysmaster/reliability";

/// get the directory for reliability.
pub fn reli_dir_get() -> Result<String> {
    #[cfg(debug)]
    return reli_dir_get_debug();
    #[cfg(not(debug))]
    return reli_dir_get_release();
}

#[cfg(not(debug))]
fn reli_dir_get_release() -> Result<String> {
    return reli_dir_get_run();
}

#[cfg(debug)]
fn reli_dir_get_debug() -> Result<String> {
    // /run/sysmaster/reliability/
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
    Err(Error::NotFound {
        what: "reli dir".to_string(),
    })
}

/// prepare the directory for reliability.
/// the reliability path is prepared and searched according to the following priority, from high to low:
/// 1. /run/sysmaster/reliability/: the real running directory.
/// 2. [debug-only]OUT_DIR/../reliability/: make CI happy, which is target/debug/reliability/ or target/release/reliability/ usually.
/// 3. [debug-only]ROCESS_RELI_PATH: the path customized.
pub fn reli_dir_prepare() -> Result<()> {
    // create '/run/sysmaster/reliability' or 'xxx/reliability' with mode 700
    let old_mask = stat::umask(Mode::from_bits_truncate(!0o700));
    #[cfg(debug)]
    let ret = reli_dir_prepare_debug();
    #[cfg(not(debug))]
    let ret = reli_dir_prepare_release();
    let _ = stat::umask(old_mask);
    ret
}

#[cfg(not(debug))]
fn reli_dir_prepare_release() -> Result<()> {
    return reli_dir_prepare_run();
}

#[cfg(debug)]
fn reli_dir_prepare_debug() -> Result<()> {
    // // /run/sysmaster/reliability/
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
    if let Error::NotFound { what: _ } = err_customize {
        Err(err_customize)
    } else if let Error::NotFound { what: _ } = err_out {
        Err(err_out)
    } else {
        Err(err_run)
    }
}

fn reli_dir_prepare_run() -> Result<()> {
    let dir = Path::new(RELI_PATH_DIR);
    if !dir.exists() {
        fs::create_dir_all(dir).context(IoSnafu)?;
    }

    log::info!(
        "prepare reliability running directory successfully: {}.",
        RELI_PATH_DIR
    );
    Ok(())
}

fn reli_dir_get_run() -> Result<String> {
    let dir = Path::new(RELI_PATH_DIR);
    if dir.exists() {
        log::info!(
            "get reliability run directory successfully: {}.",
            RELI_PATH_DIR
        );
        Ok(String::from(RELI_PATH_DIR))
    } else {
        Err(Error::NotFound {
            what: dir.to_string_lossy().to_string(),
        })
    }
}

#[cfg(debug)]
fn reli_dir_prepare_out() -> Result<()> {
    let dir_string = out_dir_string_get();
    if let Some(d_str) = dir_string {
        let dir = Path::new(&d_str);
        if !dir.exists() {
            fs::create_dir_all(dir).context(IoSnafu)?;
        }

        log::info!("prepare reliability out directory successfully: {}.", d_str);
        return Ok(());
    }

    Err(Error::NotFound {
        what: "prepare reliability out directory".to_string(),
    })
}

#[cfg(debug)]
fn reli_dir_get_out() -> Result<String> {
    let dir_string = out_dir_string_get();
    if let Some(d_str) = dir_string {
        let dir = Path::new(&d_str);
        if dir.exists() {
            log::info!("get reliability out directory successfully: {}.", d_str);
            return Ok(d_str);
        }
    }

    Err(Error::NotFound {
        what: "get reliability out directory".to_string(),
    })
}

#[cfg(debug)]
fn reli_dir_prepare_customize() -> Result<()> {
    let dir_string = env::var("PROCESS_LIB_LOAD_PATH").ok();
    if let Some(d_str) = dir_string {
        let dir = Path::new(&d_str);
        if !dir.exists() {
            fs::create_dir_all(dir).context(IoSnafu)?;
        }

        log::info!(
            "prepare reliability customized directory successfully: {}.",
            d_str
        );
        return Ok(());
    }

    Err(Error::NotFound {
        what: "prepare reliability customized directory".to_string(),
    })
}

#[cfg(debug)]
fn reli_dir_get_customize() -> Result<String> {
    let dir_string = env::var("PROCESS_LIB_LOAD_PATH").ok();
    if let Some(d_str) = dir_string {
        let dir = Path::new(&d_str);
        if dir.exists() {
            log::info!(
                "get reliability customized directory successfully: {}.",
                d_str
            );
            return Ok(d_str);
        }
    }

    Err(Error::NotFound {
        what: "get reliability customized directory".to_string(),
    })
}

#[cfg(debug)]
fn out_dir_string_get() -> Option<String> {
    let run = env::var("OUT_DIR").ok();
    let compile: Option<String> = option_env!("OUT_DIR").map(String::from);
    run.or(compile).map(|dir_string| {
        if dir_string.contains("build") {
            let tmp: Vec<_> = dir_string.split("build").collect();
            format!("{}reliability", tmp[0])
        } else {
            format!("{}reliability", dir_string)
        }
    })
}
