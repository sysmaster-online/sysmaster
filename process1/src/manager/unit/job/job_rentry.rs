use crate::manager::reliability::{
    ReDb, ReDbRoTxn, ReDbRwTxn, ReDbTable, Reliability, RELI_DB_EJOB_SUSPENDS, RELI_DB_EJOB_TRIGGER,
};
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use std::rc::Rc;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct JobReTrigData {}

impl JobReTrigData {
    fn new() -> JobReTrigData {
        JobReTrigData {}
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct JobReSuspKey {
    unit_id: String,
}

impl JobReSuspKey {
    fn new(unit_idr: &str) -> JobReSuspKey {
        JobReSuspKey {
            unit_id: String::from(unit_idr),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct JobReSuspData {}

impl JobReSuspData {
    fn new() -> JobReSuspData {
        JobReSuspData {}
    }
}

pub(super) struct JobRe {
    // database: multi-instance(N)
    trigger: Rc<ReDb<String, JobReTrigData>>, // RELI_DB_EJOB_TRIGGER; key: unit_id, data: kind+attr;
    suspends: Rc<ReDb<JobReSuspKey, JobReSuspData>>, // RELI_DB_EJOB_SUSPENDS; key: unit_id+kind, data: attr;
}

impl JobRe {
    pub(super) fn new(relir: &Reliability) -> JobRe {
        let trigger = Rc::new(ReDb::new(relir, RELI_DB_EJOB_TRIGGER));
        let suspends = Rc::new(ReDb::new(relir, RELI_DB_EJOB_SUSPENDS));
        let rentry = JobRe { trigger, suspends };
        rentry.register(relir);
        rentry
    }

    pub(super) fn trigger_insert(&self, unit_id: &String) {
        let jt_data = JobReTrigData::new();
        self.trigger.insert(unit_id.clone(), jt_data);
    }

    pub(super) fn trigger_remove(&self, unit_id: &String) {
        self.trigger.remove(unit_id);
    }

    pub(super) fn trigger_get(&self, unit_id: &String) -> Option<()> {
        if let Some(_jt_data) = self.trigger.get(unit_id) {
            Some(())
        } else {
            None
        }
    }

    pub(super) fn trigger_keys(&self) -> Vec<String> {
        self.trigger.keys()
    }

    pub(super) fn trigger_entries(&self) -> Vec<(String, ())> {
        self.trigger
            .entries()
            .iter()
            .map(|(unit_id, _jt_data)| (unit_id.clone(), ()))
            .collect::<_>()
    }

    pub(super) fn suspends_insert(&self, unit_id: &String) {
        let js_key = JobReSuspKey::new(unit_id);
        let js_kdata = JobReSuspData::new();
        self.suspends.insert(js_key, js_kdata);
    }

    pub(super) fn suspends_remove(&self, unit_id: &String) {
        let js_key = JobReSuspKey::new(unit_id);
        self.suspends.remove(&js_key);
    }

    pub(super) fn suspends_get(&self, unit_id: &String) -> Option<()> {
        let js_key = JobReSuspKey::new(unit_id);
        if let Some(_js_kdata) = self.suspends.get(&js_key) {
            Some(())
        } else {
            None
        }
    }

    pub(super) fn suspends_entries(&self) -> Vec<(String, ())> {
        self.suspends
            .entries()
            .iter()
            .map(|(js_key, _js_kdata)| (js_key.unit_id.clone(), ()))
            .collect::<_>()
    }

    fn register(&self, relir: &Reliability) {
        // reliability-db: RELI_DB_EJOB_TRIGGER
        let db = Rc::clone(&self.trigger);
        relir.entry_db_register(RELI_DB_EJOB_TRIGGER, db);

        // reliability-db: RELI_DB_EJOB_SUSPENDS
        let db = Rc::clone(&self.suspends);
        relir.entry_db_register(RELI_DB_EJOB_SUSPENDS, db);
    }
}

impl ReDbTable for ReDb<String, JobReTrigData> {
    fn clear(&self, wtxn: &mut ReDbRwTxn) {
        self.do_clear(wtxn);
    }

    fn export(&self, db_wtxn: &mut ReDbRwTxn) {
        self.cache_2_db(db_wtxn);
    }

    fn import<'a>(&self, db_rtxn: &'a ReDbRoTxn) {
        self.db_2_cache(db_rtxn);
    }

    fn ignore_set(&self, ignore: bool) {
        self.set_ignore(ignore);
    }
}

impl ReDbTable for ReDb<JobReSuspKey, JobReSuspData> {
    fn clear(&self, wtxn: &mut ReDbRwTxn) {
        self.do_clear(wtxn);
    }

    fn export(&self, db_wtxn: &mut ReDbRwTxn) {
        self.cache_2_db(db_wtxn);
    }

    fn import<'a>(&self, db_rtxn: &'a ReDbRoTxn) {
        self.db_2_cache(db_rtxn);
    }

    fn ignore_set(&self, ignore: bool) {
        self.set_ignore(ignore);
    }
}
