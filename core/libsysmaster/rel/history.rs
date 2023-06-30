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
use basic::{do_entry_log, do_entry_or_return_io_error};
use heed::{CompactionOption, Env, EnvOpenOptions};
use nix::sys::stat::{self, Mode};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::rc::Rc;

const RELI_HISTORY_A_DIR: &str = "a";
const RELI_HISTORY_B_DIR: &str = "b";
const RELI_HISTORY_BFLAG_FILE: &str = "b.effect";

const RELI_HISTORY_DIR: &str = "history.mdb";
const RELI_HISTORY_DATA_FILE: &str = "data.mdb";
const RELI_HISTORY_LOCK_FILE: &str = "lock.mdb";

pub struct ReliHistory {
    // control
    switch: RefCell<bool>,

    // directory
    b_exist: bool,
    hdir: String, // home-directory

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
    pub fn new(dir_str: &str, map_size: Option<usize>, max_dbs: Option<u32>) -> ReliHistory {
        // init environment, path: dir/history.mdb/(a|b)/
        let history = history_path_get(dir_str);
        let b_exist = bflag_path_get(history.clone()).exists();
        let path = history.join(subdir_cur_get(b_exist));
        let env = open_env(path.clone(), map_size, max_dbs).expect("history open env");
        log::info!("history with path {:?} successfully.", path);

        // return
        ReliHistory {
            switch: RefCell::new(false),
            b_exist,
            hdir: String::from(dir_str),
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

        // export to db
        for (_, db) in self.dbs.borrow().iter() {
            db.export(&mut db_wtxn);
        }

        // commit
        db_wtxn.0.commit().expect("history.commit");
    }

    pub(super) fn flush(&self) {
        // create transaction
        let mut db_wtxn = ReDbRwTxn::new(&self.env).expect("history.write_txn");

        // flush to db
        for (_, db) in self.dbs.borrow().iter() {
            db.flush(&mut db_wtxn);
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

    pub(super) fn compact(&self) -> Result<()> {
        // action with mode 700, excluding group and other users
        let old_mask = stat::umask(Mode::from_bits_truncate(!0o700));
        let ret = self.compact_body();
        let _ = stat::umask(old_mask);
        ret
    }

    fn compact_body(&self) -> Result<()> {
        // a -> b or b -> a
        // prepare next
        let history = history_path_get(&self.hdir);
        let next_path = history.join(subdir_next_get(self.b_exist));
        let next_file = next_path.join(RELI_HISTORY_DATA_FILE);

        // clear next: delete and re-create the whole directory
        do_entry_or_return_io_error!(fs::remove_dir_all, next_path, "remove");
        do_entry_or_return_io_error!(fs::create_dir_all, next_path, "create");

        // copy to next
        self.env
            .copy_to_path(next_file.clone(), CompactionOption::Disabled)
            .context(HeedSnafu)?;
        log::info!("compact to file {:?} successfully.", next_file);

        // remark the next flag at last: the another one
        let bflag = bflag_path_get(history.clone());
        if self.b_exist {
            do_entry_or_return_io_error!(fs::remove_file, bflag, "remove");
        } else {
            do_entry_or_return_io_error!(File::create, bflag, "create");
        }

        // try to clear previous: it would be done in the next re-exec, but we try to delete it as soon as possible.
        let cur_path = history.join(subdir_cur_get(self.b_exist));
        let cur_data = cur_path.join(RELI_HISTORY_DATA_FILE);
        let cur_lock = cur_path.join(RELI_HISTORY_LOCK_FILE);
        do_entry_log!(fs::remove_file, cur_data, "remove");
        do_entry_log!(fs::remove_file, cur_lock, "remove");

        Ok(())
    }

    pub fn switch_set(&self, switch: bool) {
        *self.switch.borrow_mut() = switch;
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

    pub fn switch(&self) -> bool {
        *self.switch.borrow()
    }
}

pub fn prepare(dir_str: &str) -> Result<()> {
    // directory
    let history = history_path_get(dir_str);
    if !history.exists() {
        do_entry_or_return_io_error!(fs::create_dir_all, history, "create");
    }

    // sub-directory
    let a = history.join(RELI_HISTORY_A_DIR);
    if !a.exists() {
        do_entry_or_return_io_error!(fs::create_dir_all, a, "create");
    }

    let b = history.join(RELI_HISTORY_B_DIR);
    if !b.exists() {
        do_entry_or_return_io_error!(fs::create_dir_all, b, "create");
    }

    Ok(())
}

fn open_env(path: PathBuf, map_size: Option<usize>, max_dbs: Option<u32>) -> heed::Result<Env> {
    let mut eoo = EnvOpenOptions::new();
    if let Some(size) = map_size {
        eoo.map_size(size);
    }
    if let Some(max) = max_dbs {
        eoo.max_dbs(max);
    }
    eoo.open(path)
}

fn subdir_next_get(b_exist: bool) -> String {
    if b_exist {
        // b->a
        String::from(RELI_HISTORY_A_DIR)
    } else {
        // a->b
        String::from(RELI_HISTORY_B_DIR)
    }
}

fn subdir_cur_get(b_exist: bool) -> String {
    if b_exist {
        // b
        String::from(RELI_HISTORY_B_DIR)
    } else {
        // a
        String::from(RELI_HISTORY_A_DIR)
    }
}

fn bflag_path_get(history: PathBuf) -> PathBuf {
    history.join(RELI_HISTORY_BFLAG_FILE)
}

fn history_path_get(dir: &str) -> PathBuf {
    Path::new(dir).join(RELI_HISTORY_DIR)
}
