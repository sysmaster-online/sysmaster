use libsysmaster::{ReDb, ReDbRoTxn, ReDbRwTxn, ReDbTable, Reliability};
use serde::{Deserialize, Serialize};
use std::rc::Rc;

const RELI_DB_HTARGET_MNG: &str = "tarmng";

// target contain Dead and Active state，correspond the inactive and active of the unit
#[derive(PartialEq, Eq, Debug, Copy, Clone, Serialize, Deserialize)]
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
    pub(super) fn new(relir: &Reliability) -> TargetRe {
        let mng = Rc::new(TargetReDb(ReDb::new(relir, RELI_DB_HTARGET_MNG)));
        let rentry = TargetRe { mng };
        rentry.register(relir);
        rentry
    }

    pub(super) fn mng_insert(&self, unit_id: &str, state: TargetState) {
        let mng = TargetReMng::new(state);
        self.mng.0.insert(unit_id.to_string(), mng);
    }

    pub(super) fn mng_get(&self, unit_id: &String) -> Option<TargetState> {
        let mng = self.mng.0.get(unit_id);
        mng.map(|m| m.state)
    }

    fn register(&self, relir: &Reliability) {
        // reliability-db: RELI_DB_HTARGET_MNG
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

    fn import<'a>(&self, db_rtxn: &'a ReDbRoTxn) {
        self.0.db_2_cache(db_rtxn);
    }

    fn ignore_set(&self, ignore: bool) {
        self.0.set_ignore(ignore);
    }
}
