use crate::error::*;
use heed::types::{OwnedType, SerdeBincode};
use heed::{Database, Env, EnvOpenOptions};
use std::path::Path;
use std::{fmt, fs};

const RELI_ENABLE_DIR: &str = "enable.mdb";
const RELI_ENABLE_MAX_DBS: u32 = 1;
const RELI_DB_ENABLE: &str = "enable";
#[allow(dead_code)]
static RELI_ENABLE_DB_NAME: [&str; RELI_ENABLE_MAX_DBS as usize] = [RELI_DB_ENABLE];
const RELI_ENABLE_KEY: u32 = 0; // singleton

pub struct ReliEnable {
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

impl ReliEnable {
    pub fn new(dir_str: &str) -> ReliEnable {
        // init environment
        let path = Path::new(dir_str).join(RELI_ENABLE_DIR);
        let env = EnvOpenOptions::new()
            .max_dbs(RELI_ENABLE_MAX_DBS)
            .open(path)
            .unwrap();

        // create db
        let enable = env.create_database(Some(RELI_DB_ENABLE)).unwrap();

        // return
        ReliEnable { env, enable }
    }

    pub fn data_clear(&self) {
        let mut wtxn = self.env.write_txn().expect("enable.write_txn");
        self.enable.clear(&mut wtxn).expect("clear.put");
        wtxn.commit().expect("enable.commit");
    }

    pub fn set_enable(&self, enable: bool) {
        let mut wtxn = self.env.write_txn().expect("enable.write_txn");
        self.enable
            .put(&mut wtxn, &RELI_ENABLE_KEY, &enable)
            .expect("enable.put");
        wtxn.commit().expect("enable.commit");
    }

    pub fn enable(&self) -> bool {
        let rtxn = self.env.read_txn().expect("enable.read_txn");
        let enable = self.enable.get(&rtxn, &RELI_ENABLE_KEY).unwrap_or(None);
        enable.unwrap_or(false)
    }

    fn enable_len(&self) -> Result<u64> {
        let rtxn = self.env.read_txn().context(HeedSnafu)?;
        self.enable.len(&rtxn).context(HeedSnafu)
    }
}

pub fn prepare(dir_str: &str) -> Result<()> {
    let enable = Path::new(dir_str).join(RELI_ENABLE_DIR);
    if !enable.exists() {
        fs::create_dir_all(&enable).context(IoSnafu)?;
    }

    Ok(())
}
