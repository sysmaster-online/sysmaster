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

use crate::manager::rentry::{RELI_DB_HJOB_SUSPENDS, RELI_DB_HJOB_TRIGGER};
use core::rel::{ReDb, Reliability};
use serde::{Deserialize, Serialize};
use std::hash::Hash;
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub(crate) enum JobKind {
    // 'type' is better, but it's keyword in rust
    // basic kind
    /* mut: the stage of unit can be changed */
    Start,
    Stop,
    Reload,
    Restart,

    /* non-mut: the stage of unit can not be changed */
    Verify,
    Nop,

    // compound kind
    TryReload,
    TryRestart,
    ReloadOrStart,
}

pub(super) fn job_is_basic_op(kind: JobKind) -> bool {
    match kind {
        JobKind::Start | JobKind::Stop | JobKind::Reload | JobKind::Restart => true,
        JobKind::Verify | JobKind::Nop => true,
        JobKind::TryReload | JobKind::TryRestart | JobKind::ReloadOrStart => false, // compound kind
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct JobAttr {
    pub(crate) ignore_order: bool,
    pub(crate) irreversible: bool,
    pub(crate) force: bool,
    pub(crate) no_relevancy: bool,
}

impl JobAttr {
    pub(super) fn new(
        ignore_order: bool,
        irreversible: bool,
        force: bool,
        no_relevancy: bool,
    ) -> JobAttr {
        JobAttr {
            ignore_order,
            irreversible,
            force,
            no_relevancy,
        }
    }

    pub(super) fn or(&mut self, other: &Self) -> &Self {
        self.ignore_order |= other.ignore_order;
        self.irreversible |= other.irreversible;
        self.force |= other.force;
        self.no_relevancy |= other.no_relevancy;
        self
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct JobReTrigData {
    kind: JobKind,
    attr: JobAttr,
}

impl JobReTrigData {
    fn new(kind: JobKind, attrr: &JobAttr) -> JobReTrigData {
        JobReTrigData {
            kind,
            attr: attrr.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct JobReSuspKey {
    unit_id: String,
    kind: JobKind,
}

impl JobReSuspKey {
    fn new(unit_idr: &str, kind: JobKind) -> JobReSuspKey {
        JobReSuspKey {
            unit_id: String::from(unit_idr),
            kind,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct JobReSuspData {
    attr: JobAttr,
}

impl JobReSuspData {
    fn new(attrr: &JobAttr) -> JobReSuspData {
        JobReSuspData {
            attr: attrr.clone(),
        }
    }
}

pub(super) struct JobRe {
    // database: multi-instance(N)
    trigger: Rc<ReDb<String, JobReTrigData>>, // RELI_DB_HJOB_TRIGGER; key: unit_id, data: kind+attr;
    suspends: Rc<ReDb<JobReSuspKey, JobReSuspData>>, // RELI_DB_HJOB_SUSPENDS; key: unit_id+kind, data: attr;
}

impl JobRe {
    pub(super) fn new(relir: &Rc<Reliability>) -> JobRe {
        let trigger = Rc::new(ReDb::new(relir, RELI_DB_HJOB_TRIGGER));
        let suspends = Rc::new(ReDb::new(relir, RELI_DB_HJOB_SUSPENDS));
        let rentry = JobRe { trigger, suspends };
        rentry.register(relir);
        rentry
    }

    pub(super) fn trigger_insert(&self, unit_id: &str, kind: JobKind, attr: &JobAttr) {
        assert!(job_is_basic_op(kind));
        let jt_data = JobReTrigData::new(kind, attr);
        self.trigger.insert(String::from(unit_id), jt_data);
    }

    pub(super) fn trigger_remove(&self, unit_id: &str) {
        self.trigger.remove(&unit_id.to_string());
    }

    pub(super) fn trigger_get(&self, unit_id: &str) -> Option<(JobKind, JobAttr)> {
        if let Some(jt_data) = self.trigger.get(&unit_id.to_string()) {
            Some((jt_data.kind, jt_data.attr))
        } else {
            None
        }
    }

    pub(super) fn trigger_keys(&self) -> Vec<String> {
        self.trigger.keys()
    }

    pub(super) fn trigger_entries(&self) -> Vec<(String, JobKind, JobAttr)> {
        self.trigger
            .entries()
            .iter()
            .map(|(unit_id, jt_data)| (unit_id.clone(), jt_data.kind, jt_data.attr.clone()))
            .collect::<_>()
    }

    pub(super) fn suspends_insert(&self, unit_id: &str, kind: JobKind, attr: &JobAttr) {
        assert!(job_is_basic_op(kind));
        let js_key = JobReSuspKey::new(unit_id, kind);
        let js_kdata = JobReSuspData::new(attr);
        self.suspends.insert(js_key, js_kdata);
    }

    pub(super) fn suspends_remove(&self, unit_id: &str, kind: JobKind) {
        let js_key = JobReSuspKey::new(unit_id, kind);
        self.suspends.remove(&js_key);
    }

    pub(super) fn suspends_get(&self, unit_id: &str, kind: JobKind) -> Option<JobAttr> {
        let js_key = JobReSuspKey::new(unit_id, kind);
        if let Some(js_kdata) = self.suspends.get(&js_key) {
            Some(js_kdata.attr)
        } else {
            None
        }
    }

    pub(super) fn suspends_entries(&self) -> Vec<(String, JobKind, JobAttr)> {
        self.suspends
            .entries()
            .iter()
            .map(|(js_key, js_kdata)| (js_key.unit_id.clone(), js_key.kind, js_kdata.attr.clone()))
            .collect::<_>()
    }

    fn register(&self, relir: &Reliability) {
        // reliability-db: RELI_DB_HJOB_TRIGGER
        let db = Rc::clone(&self.trigger);
        relir.history_db_register(RELI_DB_HJOB_TRIGGER, db);

        // reliability-db: RELI_DB_HJOB_SUSPENDS
        let db = Rc::clone(&self.suspends);
        relir.history_db_register(RELI_DB_HJOB_SUSPENDS, db);
    }
}
