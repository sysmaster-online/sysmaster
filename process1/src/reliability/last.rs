#![allow(clippy::type_complexity)]
use heed::types::{OwnedType, SerdeBincode, Str};
use heed::{Database, Env, EnvOpenOptions};
use std::cell::RefCell;
use std::fmt;
use std::fs;
use std::io::Error;
use std::path::Path;

const RELI_LAST_DIR: &str = "last.mdb";
const RELI_LAST_MAX_DBS: u32 = 2;
const RELI_DB_LUNIT: &str = "unit";
const RELI_DB_LFRAME: &str = "frame";
#[allow(dead_code)]
static RELI_LAST_DB_NAME: [&str; RELI_LAST_MAX_DBS as usize] = [RELI_DB_LUNIT, RELI_DB_LFRAME];
const RELI_LAST_KEY: u32 = 0; // singleton

pub(super) struct ReliLast {
    // control
    ignore: RefCell<bool>,

    // data
    /* environment */
    env: Env,

    /* database: singleton(1) */
    unit: Database<OwnedType<u32>, Str>, // RELI_DB_LUNIT; key: RELI_LAST_KEY, data: unit_id;
    frame: Database<OwnedType<u32>, SerdeBincode<(u32, Option<u32>, Option<u32>)>>, // RELI_DB_LFRAME; key: RELI_LAST_KEY, data: f1+f2+f3;
}

impl fmt::Debug for ReliLast {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReliLast")
            .field("env.path", &self.env.path())
            .field("unit.len", &self.unit_len().unwrap_or(0))
            .field("frame.len", &self.frame_len().unwrap_or(0))
            .finish()
    }
}

impl ReliLast {
    pub(super) fn new(dir_str: &str) -> ReliLast {
        // init environment
        let path = Path::new(dir_str).join(RELI_LAST_DIR);
        let env = EnvOpenOptions::new()
            .max_dbs(RELI_LAST_MAX_DBS)
            .open(path)
            .unwrap();

        // create db
        let unit = env.create_database(Some(RELI_DB_LUNIT)).unwrap();
        let frame = env.create_database(Some(RELI_DB_LFRAME)).unwrap();

        // return
        ReliLast {
            ignore: RefCell::new(false),
            env,
            unit,
            frame,
        }
    }

    pub(super) fn data_clear(&self) {
        let mut wtxn = self.env.write_txn().expect("last.write_txn");
        self.unit.clear(&mut wtxn).expect("clear.put");
        self.frame.clear(&mut wtxn).expect("clear.put");
        wtxn.commit().expect("last.commit");
    }

    pub(super) fn ignore_set(&self, ignore: bool) {
        *self.ignore.borrow_mut() = ignore;
    }

    pub(super) fn set_unit(&self, unit_id: &str) {
        if self.ignore() {
            return;
        }

        let mut wtxn = self.env.write_txn().expect("last.write_txn");
        self.unit
            .put(&mut wtxn, &RELI_LAST_KEY, unit_id)
            .expect("last.put");
        wtxn.commit().expect("last.commit");
    }

    pub(super) fn clear_unit(&self) {
        if self.ignore() {
            return;
        }

        let mut wtxn = self.env.write_txn().expect("last.write_txn");
        self.unit
            .delete(&mut wtxn, &RELI_LAST_KEY)
            .expect("last.delete");
        wtxn.commit().expect("last.commit");
    }

    pub(super) fn set_frame(&self, f1: u32, f2: Option<u32>, f3: Option<u32>) {
        if self.ignore() {
            return;
        }

        let mut wtxn = self.env.write_txn().expect("last.write_txn");
        self.frame
            .put(&mut wtxn, &RELI_LAST_KEY, &(f1, f2, f3))
            .expect("last.put");
        wtxn.commit().expect("last.commit");
    }

    pub(super) fn clear_frame(&self) {
        if self.ignore() {
            return;
        }

        let mut wtxn = self.env.write_txn().expect("last.write_txn");
        self.frame
            .delete(&mut wtxn, &RELI_LAST_KEY)
            .expect("last.delete");
        wtxn.commit().expect("last.commit");
    }

    pub(super) fn unit(&self) -> Option<String> {
        let rtxn = self.env.read_txn().expect("last.read_txn");
        let unit_id = self.unit.get(&rtxn, &RELI_LAST_KEY).unwrap_or(None);
        unit_id.map(|u| u.to_string())
    }

    pub(super) fn frame(&self) -> Option<(u32, Option<u32>, Option<u32>)> {
        let rtxn = self.env.read_txn().expect("last.read_txn");
        let frame = self.frame.get(&rtxn, &RELI_LAST_KEY).unwrap_or(None);
        frame
    }

    fn unit_len(&self) -> heed::Result<u64> {
        let rtxn = self.env.read_txn()?;
        self.unit.len(&rtxn)
    }

    fn frame_len(&self) -> heed::Result<u64> {
        let rtxn = self.env.read_txn()?;
        self.frame.len(&rtxn)
    }

    pub(super) fn ignore(&self) -> bool {
        *self.ignore.borrow()
    }
}

pub(super) fn prepare(dir_str: &str) -> Result<(), Error> {
    let last = Path::new(dir_str).join(RELI_LAST_DIR);
    if !last.exists() {
        fs::create_dir_all(&last)?;
    }

    Ok(())
}
