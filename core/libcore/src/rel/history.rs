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

use super::base::{ReDbRwTxn, ReDbTable, ReliSwitch};
use heed::Env;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

pub struct ReliHistory {
    // control
    switch: RefCell<ReliSwitch>,

    // database: multi-instance(N)
    dbs: RefCell<HashMap<String, Rc<dyn ReDbTable>>>, // key: name, value: db
}

impl fmt::Debug for ReliHistory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReliHistory")
            .field("env.dbs.len", &self.dbs.borrow().len())
            .finish()
    }
}

impl ReliHistory {
    pub fn new() -> ReliHistory {
        ReliHistory {
            switch: RefCell::new(ReliSwitch::CacheAll),
            dbs: RefCell::new(HashMap::new()),
        }
    }

    pub fn data_clear(&self, env: &Env) {
        let mut db_wtxn = ReDbRwTxn::new(env).expect("history.write_txn");
        for (_, db) in self.dbs.borrow().iter() {
            db.clear(&mut db_wtxn);
        }
        db_wtxn.0.commit().expect("history.commit");
    }

    pub fn db_register(&self, name: &str, db: Rc<dyn ReDbTable>) {
        self.dbs.borrow_mut().insert(name.to_string(), db);
    }

    #[allow(dead_code)]
    pub fn commit(&self, env: &Env) {
        // create transaction
        let mut db_wtxn = ReDbRwTxn::new(env).expect("history.write_txn");

        // export to db
        for (_, db) in self.dbs.borrow().iter() {
            db.export(&mut db_wtxn);
        }

        // commit
        db_wtxn.0.commit().expect("history.commit");
    }

    pub(super) fn flush(&self, env: &Env, switch: ReliSwitch) {
        // create transaction
        let mut db_wtxn = ReDbRwTxn::new(env).expect("history.write_txn");

        // flush to db
        for (_, db) in self.dbs.borrow().iter() {
            db.flush(&mut db_wtxn, switch);
        }

        // commit
        db_wtxn.0.commit().expect("history.commit");
    }

    pub fn import(&self) {
        // import from db
        for (_, db) in self.dbs.borrow().iter() {
            db.import();
        }
    }

    pub fn switch_set(&self, switch: ReliSwitch) {
        *self.switch.borrow_mut() = switch;
        for (_, db) in self.dbs.borrow().iter() {
            db.switch_set(switch);
        }
    }

    pub fn clear(&self) {
        self.dbs.borrow_mut().clear();
    }

    pub fn switch(&self) -> ReliSwitch {
        *self.switch.borrow()
    }
}
