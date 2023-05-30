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

use super::base::{ReDbRoTxn, ReDbRwTxn, ReDbTable};
use crate::error::*;
use heed::{Env, EnvOpenOptions};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;
use std::{fmt, fs};

const RELI_HISTORY_DIR: &str = "history.mdb";

pub struct ReliHistory {
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
    pub fn new(dir_str: &str, max: u32) -> ReliHistory {
        // init environment
        let path = Path::new(dir_str).join(RELI_HISTORY_DIR);
        let env = EnvOpenOptions::new()
            .map_size(10 * 1024 * 1024)
            .max_dbs(max)
            .open(path)
            .unwrap();

        // return
        ReliHistory {
            ignore: RefCell::new(false),
            env,
            dbs: RefCell::new(HashMap::new()),
        }
    }

    pub fn data_clear(&self) {
        let mut db_wtxn = ReDbRwTxn::new(&self.env).expect("history.write_txn");
        for (_, db) in self.dbs.borrow().iter() {
            db.clear(&mut db_wtxn);
        }
        db_wtxn.0.commit().expect("history.commit");
    }

    pub fn db_register(&self, name: &str, db: Rc<dyn ReDbTable>) {
        self.dbs.borrow_mut().insert(name.to_string(), db);
    }

    pub fn commit(&self) {
        // create transaction
        let mut db_wtxn = ReDbRwTxn::new(&self.env).expect("history.write_txn");

        // flush to db
        for (_, db) in self.dbs.borrow().iter() {
            db.export(&mut db_wtxn);
        }

        // commit
        db_wtxn.0.commit().expect("history.commit");
    }

    /// daemon-reload or daemon-reexec clear db and data reflush to db
    pub fn reflush(&self) {
        // create transaction
        let mut db_wtxn = ReDbRwTxn::new(&self.env).expect("history.write_txn");

        // flush to db
        for (_, db) in self.dbs.borrow().iter() {
            db.clear(&mut db_wtxn);
            db.reexport(&mut db_wtxn);
        }

        // commit
        db_wtxn.0.commit().expect("history.commit");
    }

    pub fn import(&self) {
        let db_rtxn = ReDbRoTxn::new(&self.env).expect("history.write_txn");

        // import from db
        for (_, db) in self.dbs.borrow().iter() {
            db.import(&db_rtxn);
        }
    }

    pub fn switch_set(&self, switch: bool) {
        // set switch
        *self.ignore.borrow_mut() = switch;
        for (_, db) in self.dbs.borrow().iter() {
            db.switch_set(switch);
        }
    }

    pub fn env(&self) -> &Env {
        &self.env
    }

    pub fn clear(&self) {
        self.dbs.borrow_mut().clear();
    }

    pub fn ignore(&self) -> bool {
        *self.ignore.borrow()
    }
}

pub fn prepare(dir_str: &str) -> Result<()> {
    let history = Path::new(dir_str).join(RELI_HISTORY_DIR);
    if !history.exists() {
        fs::create_dir_all(&history).context(IoSnafu)?;
    }

    Ok(())
}
