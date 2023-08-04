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

use super::base::RELI_INTERNAL_DB_ENABLE;
use crate::error::*;
use heed::types::{OwnedType, SerdeBincode};
use heed::{Database, Env};
use std::fmt;
use std::rc::Rc;

const RELI_ENABLE_KEY: u32 = 0; // singleton

pub struct ReliEnable {
    // associated objects
    env: Rc<Env>,
    // data
    /* database: singleton(1), create in use */
    /* enable: Database<OwnedType<u32>, SerdeBincode<bool>>; RELI_INTERNAL_DB_ENABLE; key: RELI_ENABLE_KEY, data: bool */
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
    pub fn new(envr: &Rc<Env>) -> ReliEnable {
        ReliEnable {
            env: Rc::clone(envr),
        }
    }

    pub fn data_clear(&self) {
        if let Ok(db) = self.open_db_enable() {
            let mut wtxn = self.env.write_txn().expect("enable.write_txn");
            db.clear(&mut wtxn).expect("clear.put");
            wtxn.commit().expect("enable.commit");
        }
    }

    pub fn set_enable(&self, enable: bool) {
        let db = self.open_db_enable().unwrap();
        let mut wtxn = self.env.write_txn().expect("enable.write_txn");
        db.put(&mut wtxn, &RELI_ENABLE_KEY, &enable)
            .expect("enable.put");
        wtxn.commit().expect("enable.commit");
    }

    pub fn enable(&self) -> bool {
        let db = self.open_db_enable().unwrap();
        let rtxn = self.env.read_txn().expect("enable.read_txn");
        let enable = db.get(&rtxn, &RELI_ENABLE_KEY).unwrap_or(None);
        enable.unwrap_or(false)
    }

    fn enable_len(&self) -> Result<usize> {
        let db = self.open_db_enable()?;
        let rtxn = self.env.read_txn().context(HeedSnafu)?;
        db.len(&rtxn).context(HeedSnafu)
    }

    fn open_db_enable(&self) -> Result<Database<OwnedType<u32>, SerdeBincode<bool>>> {
        /* enable: Database<OwnedType<u32>, SerdeBincode<bool>>; RELI_INTERNAL_DB_ENABLE; key: RELI_ENABLE_KEY, data: bool */
        let database = self
            .env
            .open_database(Some(RELI_INTERNAL_DB_ENABLE))
            .context(HeedSnafu)?;
        if let Some(db) = database {
            Ok(db)
        } else {
            self.env
                .create_database(Some(RELI_INTERNAL_DB_ENABLE))
                .context(HeedSnafu)
        }
    }
}
