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

use core::rel::{ReDb, ReDbRwTxn, ReDbTable, ReliSwitch, Reliability};
use macros::EnumDisplay;
use serde::{Deserialize, Serialize};
use std::rc::Rc;

const RELI_DB_HTARGET_MNG: &str = "tarmng";

// target contain Dead and Active state，correspond the inactive and active of the unit
#[derive(PartialEq, Eq, Debug, Copy, Clone, Serialize, Deserialize, EnumDisplay)]
pub(super) enum TargetState {
    Dead,
    Active,
    StateMax,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct TargetReMng {
    state: TargetState,
}

impl TargetReMng {
    fn new(state: TargetState) -> TargetReMng {
        TargetReMng { state }
    }
}

struct TargetReDb<K, V>(ReDb<K, V>);

pub(super) struct TargetRe {
    // database: multi-instance(N)
    mng: Rc<TargetReDb<String, TargetReMng>>, // RELI_DB_HTARGET_MNG; key: unit_id, data: ;
}

impl TargetRe {
    pub(super) fn new(relir: &Rc<Reliability>) -> TargetRe {
        let mng = Rc::new(TargetReDb(ReDb::new(relir, RELI_DB_HTARGET_MNG)));
        let rentry = TargetRe { mng };
        rentry.register(relir);
        rentry
    }

    pub(super) fn mng_insert(&self, unit_id: &str, state: TargetState) {
        let mng = TargetReMng::new(state);
        self.mng.0.insert(unit_id.to_string(), mng);
    }

    pub(super) fn mng_get(&self, unit_id: &str) -> Option<TargetState> {
        let mng = self.mng.0.get(&unit_id.to_string());
        mng.map(|m| m.state)
    }

    fn register(&self, relir: &Reliability) {
        // rel-db: RELI_DB_HTARGET_MNG
        let db = Rc::clone(&self.mng);
        relir.history_db_register(RELI_DB_HTARGET_MNG, db);
    }
}

impl ReDbTable for TargetReDb<String, TargetReMng> {
    fn clear(&self, wtxn: &mut ReDbRwTxn) {
        self.0.do_clear(wtxn);
    }

    fn export(&self, db_wtxn: &mut ReDbRwTxn) {
        self.0.cache_2_db(db_wtxn);
    }

    fn flush(&self, db_wtxn: &mut ReDbRwTxn, switch: ReliSwitch) {
        self.0.data_2_db(db_wtxn, switch);
    }

    fn import<'a>(&self) {
        self.0.db_2_cache();
    }

    fn switch_set(&self, switch: ReliSwitch) {
        self.0.switch_buffer(switch);
    }
}
