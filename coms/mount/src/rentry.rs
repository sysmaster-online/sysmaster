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

use macros::EnumDisplay;
use serde::{Deserialize, Serialize};
use std::rc::Rc;
use sysmaster::rel::{ReDb, ReDbRoTxn, ReDbRwTxn, ReDbTable, Reliability};

const RELI_DB_HMOUNT_MNG: &str = "mntmng";
const RELI_DB_HMOUNTM_FRAME: &str = "mntm-frame";
const RELI_LAST_KEY: u32 = 0; // singleton

//Mount contains two states: unmounted Dead and mounted Mounted. The corresponding unit status is inactive and active
#[derive(PartialEq, Eq, Debug, Copy, Clone, Serialize, Deserialize, EnumDisplay)]
pub(super) enum MountState {
    Dead,
    Mounted,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct MountReMng {
    state: MountState,
}

impl MountReMng {
    fn new(state: MountState) -> MountReMng {
        MountReMng { state }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(super) enum MountReFrame {
    Monitor,
}

struct MountReDb<K, V>(ReDb<K, V>);

pub(super) struct MountRe {
    // database: multi-instance(N)
    mng: Rc<MountReDb<String, MountReMng>>, // RELI_DB_HMOUNT_MNG; key: unit_id, data: state;

    // database: singleton(1)
    frame: Rc<MountReDb<u32, MountReFrame>>, // RELI_DB_HMOUNTM_FRAME; key: RELI_LAST_KEY, data: MountReFrame;
}

impl MountRe {
    pub(super) fn new(relir: &Reliability) -> MountRe {
        let mng = Rc::new(MountReDb(ReDb::new(relir, RELI_DB_HMOUNT_MNG)));
        let frame = Rc::new(MountReDb(ReDb::new(relir, RELI_DB_HMOUNTM_FRAME)));
        let rentry = MountRe { mng, frame };
        rentry.register(relir);
        rentry
    }

    pub(super) fn mng_insert(&self, unit_id: &str, state: MountState) {
        let mng = MountReMng::new(state);
        self.mng.0.insert(unit_id.to_string(), mng);
    }

    pub(super) fn mng_get(&self, unit_id: &String) -> Option<MountState> {
        let mng = self.mng.0.get(unit_id);
        mng.map(|m| m.state)
    }

    pub(super) fn set_last_frame(&self, frame: MountReFrame) {
        self.frame.0.insert(RELI_LAST_KEY, frame);
    }

    pub(super) fn clear_last_frame(&self) {
        self.frame.0.remove(&RELI_LAST_KEY);
    }

    pub(super) fn last_frame(&self) -> Option<MountReFrame> {
        self.frame.0.get(&RELI_LAST_KEY)
    }

    fn register(&self, relir: &Reliability) {
        // rel-db: RELI_DB_HMOUNT_MNG
        let db = Rc::clone(&self.mng);
        relir.history_db_register(RELI_DB_HMOUNT_MNG, db);

        // rel-db: RELI_DB_HMOUNTM_FRAME
        let db = Rc::clone(&self.frame);
        relir.history_db_register(RELI_DB_HMOUNTM_FRAME, db);
    }
}

impl ReDbTable for MountReDb<String, MountReMng> {
    fn clear(&self, wtxn: &mut ReDbRwTxn) {
        self.0.do_clear(wtxn);
    }

    fn export(&self, db_wtxn: &mut ReDbRwTxn) {
        self.0.cache_2_db(db_wtxn);
    }

    fn flush(&self, db_wtxn: &mut ReDbRwTxn) {
        self.0.buffer_2_db(db_wtxn);
    }

    fn import<'a>(&self, db_rtxn: &ReDbRoTxn) {
        self.0.db_2_cache(db_rtxn);
    }

    fn switch_set(&self, switch: bool) {
        self.0.switch_buffer(switch);
    }
}

impl ReDbTable for MountReDb<u32, MountReFrame> {
    fn clear(&self, wtxn: &mut ReDbRwTxn) {
        self.0.do_clear(wtxn);
    }

    fn export(&self, db_wtxn: &mut ReDbRwTxn) {
        self.0.cache_2_db(db_wtxn);
    }

    fn flush(&self, db_wtxn: &mut ReDbRwTxn) {
        self.0.buffer_2_db(db_wtxn);
    }

    fn import<'a>(&self, db_rtxn: &ReDbRoTxn) {
        self.0.db_2_cache(db_rtxn);
    }

    fn switch_set(&self, switch: bool) {
        self.0.switch_buffer(switch);
    }
}
