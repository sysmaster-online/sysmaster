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

use super::base::{RELI_INTERNAL_DB_LFRAME, RELI_INTERNAL_DB_LUNIT};
use crate::error::*;
use heed::types::{OwnedType, SerdeBincode, Str};
use heed::{Database, Env};
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

const RELI_LAST_KEY: u32 = 0; // singleton

pub struct ReliLast {
    // associated objects
    env: Rc<Env>,

    // control
    ignore: RefCell<bool>,
    // data
    /* database: singleton(1), create in use */
    /* unit: Database<OwnedType<u32>, Str>; RELI_INTERNAL_DB_LUNIT; key: RELI_LAST_KEY, data: unit_id; */
    /* frame: Database<OwnedType<u32>, SerdeBincode<Vec<ReliFrame>>>; RELI_INTERNAL_DB_LFRAME; key: RELI_LAST_KEY, data: vec<f1+f2+f3>; */
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
    pub fn new(envr: &Rc<Env>) -> ReliLast {
        ReliLast {
            env: Rc::clone(envr),
            ignore: RefCell::new(false),
        }
    }

    pub fn data_clear(&self) {
        let unit = self.open_db_unit();
        let frame = self.open_db_frame();
        let mut wtxn = self.env.write_txn().expect("last.write_txn");
        if let Ok(db) = unit {
            db.clear(&mut wtxn).expect("clear.put");
        }
        if let Ok(db) = frame {
            db.clear(&mut wtxn).expect("clear.put");
        }
        wtxn.commit().expect("last.commit");
    }

    pub fn ignore_set(&self, ignore: bool) {
        *self.ignore.borrow_mut() = ignore;
    }

    pub fn set_unit(&self, unit_id: &str) {
        if self.ignore() {
            return;
        }

        let db = self.open_db_unit().unwrap();
        let mut wtxn = self.env.write_txn().expect("last.write_txn");
        db.put(&mut wtxn, &RELI_LAST_KEY, unit_id)
            .expect("last.put");
        wtxn.commit().expect("last.commit");
    }

    pub fn clear_unit(&self) {
        if self.ignore() {
            return;
        }

        let db = self.open_db_unit().unwrap();
        let mut wtxn = self.env.write_txn().expect("last.write_txn");
        db.delete(&mut wtxn, &RELI_LAST_KEY).expect("last.delete");
        wtxn.commit().expect("last.commit");
    }

    pub fn set_frame(&self, f1: u32, f2: Option<u32>, f3: Option<u32>) {
        if self.ignore() {
            return;
        }

        let db = self.open_db_frame().unwrap();
        let mut wtxn = self.env.write_txn().expect("last.write_txn");
        let mut frame = match db.get(&wtxn, &RELI_LAST_KEY).unwrap_or(None) {
            Some(f) => f,
            None => Vec::new(),
        };
        frame.push((f1, f2, f3));
        db.put(&mut wtxn, &RELI_LAST_KEY, &frame).expect("last.put");
        wtxn.commit().expect("last.commit");
    }

    pub fn clear_frame(&self) {
        if self.ignore() {
            return;
        }

        let db = self.open_db_frame().unwrap();
        let mut wtxn = self.env.write_txn().expect("last.write_txn");
        let mut frame = match db.get(&wtxn, &RELI_LAST_KEY).unwrap_or(None) {
            Some(f) => f,
            None => Vec::new(),
        };
        frame.pop();
        db.put(&mut wtxn, &RELI_LAST_KEY, &frame).expect("last.put");
        wtxn.commit().expect("last.commit");
    }

    pub fn unit(&self) -> Option<String> {
        let db = self.open_db_unit().unwrap();
        let rtxn = self.env.read_txn().expect("last.read_txn");
        let unit_id = db.get(&rtxn, &RELI_LAST_KEY).unwrap_or(None);
        unit_id.map(|u| u.to_string())
    }

    pub fn frame(&self) -> Option<ReliFrame> {
        let db = self.open_db_frame().unwrap();
        let rtxn = self.env.read_txn().expect("last.read_txn");
        let frame = db.get(&rtxn, &RELI_LAST_KEY).unwrap_or(None);
        match frame {
            Some(mut f) => f.pop(),
            None => None,
        }
    }

    pub fn ignore(&self) -> bool {
        *self.ignore.borrow()
    }

    fn unit_len(&self) -> Result<usize> {
        let db = self.open_db_unit()?;
        let rtxn = self.env.read_txn().context(HeedSnafu)?;
        db.len(&rtxn).context(HeedSnafu)
    }

    fn frame_len(&self) -> Result<u64> {
        let db = self.open_db_frame()?;
        let rtxn = self.env.read_txn().context(HeedSnafu)?;
        let frame = db.get(&rtxn, &RELI_LAST_KEY).unwrap_or(None);
        let len = match frame {
            Some(f) => f.len(),
            None => 0,
        };
        Ok(len as u64)
    }

    fn open_db_unit(&self) -> Result<Database<OwnedType<u32>, Str>> {
        /* unit: Database<OwnedType<u32>, Str>; RELI_INTERNAL_DB_LUNIT; key: RELI_LAST_KEY, data: unit_id; */
        let database = self
            .env
            .open_database(Some(RELI_INTERNAL_DB_LUNIT))
            .context(HeedSnafu)?;
        if let Some(db) = database {
            Ok(db)
        } else {
            self.env
                .create_database(Some(RELI_INTERNAL_DB_LUNIT))
                .context(HeedSnafu)
        }
    }

    fn open_db_frame(&self) -> Result<Database<OwnedType<u32>, SerdeBincode<Vec<ReliFrame>>>> {
        /* frame: Database<OwnedType<u32>, SerdeBincode<Vec<ReliFrame>>>; RELI_INTERNAL_DB_LFRAME; key: RELI_LAST_KEY, data: vec<f1+f2+f3>; */
        let database = self
            .env
            .open_database(Some(RELI_INTERNAL_DB_LFRAME))
            .context(HeedSnafu)?;
        if let Some(db) = database {
            Ok(db)
        } else {
            self.env
                .create_database(Some(RELI_INTERNAL_DB_LFRAME))
                .context(HeedSnafu)
        }
    }
}

type ReliFrame = (u32, Option<u32>, Option<u32>);
