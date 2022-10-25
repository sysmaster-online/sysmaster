use super::base::{ReDbRoTxn, ReDbRwTxn, ReDbTable};
use heed::{Env, EnvOpenOptions};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io::Error;
use std::path::Path;
use std::rc::Rc;

const RELI_HISTORY_DIR: &str = "history.mdb";

pub(super) struct ReliHistory {
    // control
    ignore: RefCell<bool>,

    // environment
    env: Env,

    // database: multi-instance(N)
    dbs: RefCell<HashMap<String, Rc<dyn ReDbTable>>>, // key: name, value: db
}

impl fmt::Debug for ReliHistory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReliHistory")
            .field("env.path", &self.env.path())
            .field("env.dbs.len", &self.dbs.borrow().len())
            .finish()
    }
}

impl ReliHistory {
    pub(super) fn new(dir_str: &str, max: u32) -> ReliHistory {
        // init environment
        let path = Path::new(dir_str).join(RELI_HISTORY_DIR);
        let env = EnvOpenOptions::new().max_dbs(max).open(path).unwrap();

        // return
        ReliHistory {
            ignore: RefCell::new(false),
            env,
            dbs: RefCell::new(HashMap::new()),
        }
    }

    pub(super) fn data_clear(&self) {
        let mut db_wtxn = ReDbRwTxn::new(&self.env).expect("history.write_txn");
        for (_, db) in self.dbs.borrow().iter() {
            db.clear(&mut db_wtxn);
        }
        db_wtxn.0.commit().expect("history.commit");
    }

    pub(super) fn db_register(&self, name: &str, db: Rc<dyn ReDbTable>) {
        self.dbs.borrow_mut().insert(name.to_string(), db);
    }

    pub(super) fn commit(&self) {
        // create transaction
        let mut db_wtxn = ReDbRwTxn::new(&self.env).expect("history.write_txn");

        // flush to db
        for (_, db) in self.dbs.borrow().iter() {
            db.export(&mut db_wtxn);
        }

        // commit
        db_wtxn.0.commit().expect("history.commit");
    }

    pub(super) fn import(&self) {
        let db_rtxn = ReDbRoTxn::new(&self.env).expect("history.write_txn");

        // import from db
        for (_, db) in self.dbs.borrow().iter() {
            db.import(&db_rtxn);
        }
    }

    pub(super) fn ignore_set(&self, ignore: bool) {
        // set ignore
        *self.ignore.borrow_mut() = ignore;
        for (_, db) in self.dbs.borrow().iter() {
            db.ignore_set(ignore);
        }
    }

    pub(super) fn env(&self) -> &Env {
        &self.env
    }

    pub(super) fn clear(&self) {
        self.dbs.borrow_mut().clear();
    }

    pub(super) fn ignore(&self) -> bool {
        *self.ignore.borrow()
    }
}

pub(super) fn prepare(dir_str: &str) -> Result<(), Error> {
    let history = Path::new(dir_str).join(RELI_HISTORY_DIR);
    if !history.exists() {
        fs::create_dir_all(&history)?;
    }

    Ok(())
}
