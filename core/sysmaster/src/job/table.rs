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

use super::alloc::JobAlloc;
use super::entry::{Job, JobConf, JobInfo, JobResult};
use super::junit::JobUnit;
use super::rentry::JobKind;
use crate::unit::{JobMode, UnitDb, UnitX};
use core::error::*;
use core::unit::UnitRelationAtom;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;

pub(super) struct JobTable {
    // associated objects
    db: Rc<UnitDb>,

    // owned objects
    // key: job-id | unit, value: job
    t_id: RefCell<HashMap<u128, Rc<Job>>>, // guarantee the uniqueness of job-id
    t_unit: RefCell<JobUnitTable>,         // the running time of job is organized by unit
}

impl JobTable {
    pub(super) fn new(dbr: &Rc<UnitDb>) -> JobTable {
        JobTable {
            db: Rc::clone(dbr),
            t_id: RefCell::new(HashMap::new()),
            t_unit: RefCell::new(JobUnitTable::new()),
        }
    }

    pub(super) fn clear(&self) {
        // job_entry
        for (_, job) in self.t_id.borrow().iter() {
            job.clear();
        }

        // table
        // table-id
        self.t_id.borrow_mut().clear();

        // table-unit
        self.t_unit.borrow_mut().clear();
    }

    pub(super) fn rentry_map_suspend(&self, ja: &JobAlloc, config: &JobConf) -> Rc<Job> {
        let job = ja.alloc(config);
        job.rentry_map_suspend();
        self.insert_suspend(Rc::clone(&job)).unwrap();
        job
    }

    pub(super) fn rentry_insert_suspend(&self) {
        for job in self.t_unit.borrow().get_all_suspends() {
            job.rentry_suspends_insert();
        }
    }

    pub(super) fn rentry_map_trigger(&self, ja: &JobAlloc, config: &JobConf) -> Rc<Job> {
        let job = ja.alloc(config);
        job.rentry_map_trigger();
        self.insert_trigger(Rc::clone(&job)).unwrap();
        job
    }

    pub(super) fn rentry_insert_trigger(&self) {
        for job in self.t_unit.borrow().get_all_triggers() {
            job.rentry_trigger_insert();
        }
    }

    pub(super) fn coldplug_suspend(&self, unit: &UnitX) {
        for job in self.t_unit.borrow().get_suspends(unit).iter() {
            job.coldplug_suspend();
        }
    }

    pub(super) fn coldplug_trigger(&self, unit: &UnitX) {
        if let Some((job, _)) = self.t_unit.borrow().get_trigger_info(unit) {
            job.coldplug_trigger();
        }
    }

    pub(super) fn record_suspend(&self, ja: &JobAlloc, config: &JobConf, mode: JobMode) -> bool {
        let unit = config.get_unit();
        let kind = config.get_kind();

        // add job only when nothing with the same 'unit'+'kind' exists
        let empty = self.t_unit.borrow().get_suspend(unit, kind).is_none();
        if empty {
            let job = ja.alloc(config);
            job.init_attr(mode);
            self.insert_suspend(job).expect("insert a new job failed.");
            true
        } else {
            false
        }
    }

    #[allow(clippy::type_complexity)]
    pub(super) fn commit(
        &self,
        other: &Self,
        mode: JobMode,
    ) -> Result<(Vec<Rc<Job>>, Vec<Rc<Job>>, Vec<Rc<Job>>)> {
        // check other-jobs-id first: make rollback simple
        for (o_id, _) in other.t_id.borrow().iter() {
            if self.t_id.borrow().get(o_id).is_some() {
                return Err(Error::Internal);
            }
        }

        // isolate
        let mut iso_jobs = self.isolate_suspends(other, mode);

        // merge
        let (add_jobs, mut flush_jobs, update_jobs) = self.merge_suspends(other);

        // reshuffle
        let mut merge_jobs = self.reshuffle();

        // relation processing if something is isolated or flushed
        if !iso_jobs.is_empty() || !flush_jobs.is_empty() {
            for unit in jobs_2_units(&iso_jobs).iter() {
                self.process_relation(unit, true);
            }
            for unit in jobs_2_units(&flush_jobs).iter() {
                self.process_relation(unit, true);
            }
        }

        let mut del_jobs = Vec::new();
        del_jobs.append(&mut iso_jobs);
        del_jobs.append(&mut flush_jobs);
        del_jobs.append(&mut merge_jobs);
        Ok((add_jobs, del_jobs, update_jobs))
    }

    pub(super) fn remove_suspends(
        &self,
        unit: &UnitX,
        kind1: JobKind,
        kind2: Option<JobKind>,
        result: JobResult,
    ) -> Vec<Rc<Job>> {
        // table-unit
        let del_jobs = self
            .t_unit
            .borrow_mut()
            .remove_suspends(unit, kind1, kind2, result);

        // synchronize table-id
        for job in del_jobs.iter() {
            self.t_id.borrow_mut().remove(&job.get_id());
        }

        // relation processing if something is deleted
        if !del_jobs.is_empty() {
            self.process_relation(unit, true);
        }

        del_jobs
    }

    #[allow(clippy::type_complexity)]
    pub(super) fn try_trigger(
        &self,
        unit: Option<&UnitX>,
    ) -> Option<(Option<(JobInfo, Option<JobResult>)>, Option<Rc<Job>>)> {
        // try trigger table-unit
        let trigger_ret = self.do_try_trigger(&self.db, unit);

        // synchronize table-id
        if let Some((_, Some(job))) = &trigger_ret {
            self.t_id.borrow_mut().remove(&job.get_id());
        }

        trigger_ret
    }

    pub(super) fn finish_trigger(&self, unit: &UnitX, result: JobResult) -> Option<Rc<Job>> {
        assert!(self.get_trigger_info(unit).is_some()); // guaranteed by caller

        // finish table-unit
        let del_trigger = self.t_unit.borrow_mut().finish_trigger(unit, result);

        // synchronize table-id
        if let Some(job) = &del_trigger {
            self.t_id.borrow_mut().remove(&job.get_id());
        }

        // relation processing if something is finished, whether or not something is deleted
        self.process_relation(unit, del_trigger.is_some());

        del_trigger
    }

    pub(super) fn resume_unit(&self, unit: &UnitX) {
        // resume table-unit
        self.t_unit.borrow_mut().resume_unit(unit);

        // synchronize table-id: nothing changed
    }

    pub(super) fn remove_unit(&self, unit: &UnitX) -> (Option<Rc<Job>>, Vec<Rc<Job>>) {
        // get jobs
        let del_trigger = self
            .t_unit
            .borrow()
            .get_trigger_info(unit)
            .map(|(job, _)| job);
        let del_suspends = self.t_unit.borrow().get_suspends(unit);

        // table-id
        if del_trigger.is_some() {
            let job = del_trigger.as_ref().cloned().unwrap();
            self.t_id.borrow_mut().remove(&job.get_id());
        }

        for job in del_suspends.iter() {
            self.t_id.borrow_mut().remove(&job.get_id());
        }

        // table-unit
        self.t_unit.borrow_mut().remove_unit(unit);

        (del_trigger, del_suspends)
    }

    pub(super) fn reshuffle(&self) -> Vec<Rc<Job>> {
        // reshuffle table-unit
        let merge_jobs = self.t_unit.borrow_mut().reshuffle();

        // synchronize table-id
        for job in merge_jobs.iter() {
            self.t_id.borrow_mut().remove(&job.get_id());
        }

        merge_jobs
    }

    pub(super) fn update_up_ready(&self) {
        self.t_unit.borrow_mut().update_up_ready()
    }

    #[allow(dead_code)]
    pub(super) fn len(&self) -> usize {
        self.t_id.borrow().len()
    }

    #[allow(dead_code)]
    pub(super) fn ready_len(&self) -> usize {
        self.t_unit.borrow().ready_len()
    }

    pub(super) fn get(&self, id: u128) -> Option<JobInfo> {
        self.t_id.borrow().get(&id).map(|job| JobInfo::map(job))
    }

    pub(super) fn get_suspends(&self, unit: &UnitX) -> Vec<JobInfo> {
        self.t_unit
            .borrow()
            .get_suspends(unit)
            .into_iter()
            .map(|job| JobInfo::map(&job))
            .collect()
    }

    pub(super) fn get_suspend(&self, unit: &UnitX, kind: JobKind) -> Option<JobInfo> {
        self.t_unit
            .borrow()
            .get_suspend(unit, kind)
            .map(|job| JobInfo::map(&job))
    }

    pub(super) fn get_trigger_info(&self, unit: &UnitX) -> Option<(JobInfo, bool)> {
        self.t_unit
            .borrow()
            .get_trigger_info(unit)
            .map(|(job, pause)| (JobInfo::map(&job), pause))
    }

    #[allow(dead_code)]
    pub(super) fn is_empty(&self) -> bool {
        self.t_unit.borrow().is_empty()
    }

    pub(super) fn is_unit_empty(&self, unit: &UnitX) -> bool {
        self.t_unit.borrow().is_unit_empty(unit)
    }

    pub(super) fn is_trigger(&self, id: u128) -> bool {
        if let Some(job_info) = self.get(id) {
            if let Some((t_info, _)) = self.get_trigger_info(&job_info.unit) {
                return t_info.id == job_info.id;
            }
        }
        false
    }

    #[allow(dead_code)]
    pub(super) fn is_suspend(&self, id: u128) -> bool {
        if let Some(job_info) = self.get(id) {
            if let Some(s_info) = self.get_suspend(&job_info.unit, job_info.kind) {
                return s_info.id == job_info.id;
            }
        }
        false
    }

    pub(super) fn is_suspends_conflict(&self) -> bool {
        self.t_unit.borrow().is_suspends_conflict()
    }

    pub(super) fn is_suspends_conflict_with(&self, other: &Self) -> bool {
        self.t_unit
            .borrow()
            .is_suspends_conflict_with(&other.t_unit.borrow())
    }

    pub(super) fn is_suspends_replace_with(&self, other: &Self) -> bool {
        self.t_unit
            .borrow()
            .is_suspends_replace_with(&other.t_unit.borrow())
    }

    pub(super) fn up_ready(&self) -> bool {
        self.t_unit.borrow().up_ready()
    }

    pub(super) fn calc_ready(&self) -> bool {
        self.t_unit.borrow().calc_ready()
    }

    #[allow(clippy::type_complexity)]
    fn do_try_trigger(
        &self,
        db: &UnitDb,
        unit: Option<&UnitX>,
    ) -> Option<(Option<(JobInfo, Option<JobResult>)>, Option<Rc<Job>>)> {
        assert!(self.t_unit.borrow().is_sync());

        // table-ready: "pop_last + trigger_last" or "pop_all + trigger_all"
        // the single 'last' operation is better, but 'pop_last' is not currently supported.
        // we select the batch 'all' operation to simulate the 'single' operation now.
        let uv_try = self.t_unit.borrow_mut().ready_pop(unit);
        if let Some(uv) = uv_try {
            let (trigger_info, merge_trigger) = self.try_trigger_entry(db, Rc::clone(&uv)); // status(sync): not changed
            assert!(!uv.is_empty());
            Some((trigger_info, merge_trigger))
        } else {
            None
        }
    }

    #[allow(clippy::type_complexity)]
    fn try_trigger_entry(
        &self,
        db: &UnitDb,
        value: Rc<JobUnit>,
    ) -> (Option<(JobInfo, Option<JobResult>)>, Option<Rc<Job>>) {
        let uv = value;

        assert!(!uv.is_dirty());

        // try to trigger unit: trigger (order-allowed)it or pause (order-non-allowed)it
        let is_runnable = self.t_unit.borrow().is_uv_runnable(db, Rc::clone(&uv));
        let (trigger_info, merge_trigger) = match is_runnable {
            true => uv.do_trigger(),
            false => {
                uv.pause();
                (None, None)
            }
        };

        // synchronize t_ready: the value has been removed from 't_ready', and it's not ready now, which are corresponding with each other.
        assert!(!uv.is_ready()); // something has just been triggered or paused above

        // t_data: do nothing
        // dirty
        uv.clear_dirty(); // the 'dirty' entry has been synced(corresponding), it's not dirty now.

        (trigger_info, merge_trigger)
    }

    fn process_relation(&self, unit: &UnitX, st_deleted: bool) {
        // trigger-notify only if something is deleted
        if st_deleted {
            let atom = UnitRelationAtom::UnitAtomTriggeredBy;
            for other in self.db.dep_gets_atom(unit, atom) {
                other.trigger(unit);
            }
        }

        // resume order-related units
        let atom = UnitRelationAtom::UnitAtomAfter;
        for other in self.db.dep_gets_atom(unit, atom).iter() {
            self.resume_unit(other);
        }
        let atom = UnitRelationAtom::UnitAtomBefore;
        for other in self.db.dep_gets_atom(unit, atom).iter() {
            self.resume_unit(other);
        }
    }

    fn isolate_suspends(&self, other: &Self, mode: JobMode) -> Vec<Rc<Job>> {
        // isolate table-unit
        let del_jobs = match mode {
            JobMode::Isolate | JobMode::Flush => self
                .t_unit
                .borrow_mut()
                .isolate_suspends(&other.t_unit.borrow()),
            _ => Vec::new(), // empty
        };

        // synchronize table-id
        for job in del_jobs.iter() {
            self.t_id.borrow_mut().remove(&job.get_id());
        }

        del_jobs
    }

    #[allow(clippy::type_complexity)]
    fn merge_suspends(&self, other: &Self) -> (Vec<Rc<Job>>, Vec<Rc<Job>>, Vec<Rc<Job>>) {
        // merge table-unit
        let (add_jobs, del_jobs, update_jobs) = self
            .t_unit
            .borrow_mut()
            .merge_suspends(&other.t_unit.borrow());

        // synchronize table-id
        for job in add_jobs.iter() {
            self.t_id.borrow_mut().insert(job.get_id(), Rc::clone(job));
        }
        for job in del_jobs.iter() {
            self.t_id.borrow_mut().remove(&job.get_id());
        }

        (add_jobs, del_jobs, update_jobs)
    }

    fn insert_suspend(&self, job: Rc<Job>) -> Result<()> {
        // check job-id
        let id = job.get_id();
        if self.t_id.borrow().get(&id).is_some() {
            return Err(Error::Internal);
        }

        // table-unit
        let old = self.t_unit.borrow_mut().insert_suspend(Rc::clone(&job));
        if old.is_some() {
            return Err(Error::Input);
        }

        // table-id
        self.t_id.borrow_mut().insert(id, job);

        Ok(())
    }

    fn insert_trigger(&self, job: Rc<Job>) -> Result<()> {
        // check job-id
        let id = job.get_id();
        if self.t_id.borrow().get(&id).is_some() {
            return Err(Error::Internal);
        }

        // table-unit
        let old = self.t_unit.borrow_mut().insert_trigger(Rc::clone(&job));
        if old.is_some() {
            return Err(Error::Input);
        }

        // table-id
        self.t_id.borrow_mut().insert(id, job);

        Ok(())
    }
}

//#[derive(Debug)]
struct JobUnitTable {
    // key: unit, value: jobs with order
    // data
    t_data: HashMap<Rc<UnitX>, Rc<JobUnit>>,   // quick search
    t_ready: BTreeMap<Rc<UnitX>, Rc<JobUnit>>, // quick sort for readies

    // status
    /* t_ready */
    readys: Vec<Rc<JobUnit>>, // simulate 'BTreeMap->pop_last'
    up_ready: bool,           // 'ready' status in up-level
    /* the entire entry */
    sync: bool, // sync flag of the entire table, including data and ready.
}

// the declaration "pub(self)" is for identification only.
impl JobUnitTable {
    pub(self) fn new() -> JobUnitTable {
        JobUnitTable {
            t_data: HashMap::new(),
            t_ready: BTreeMap::new(),

            readys: Vec::new(),
            up_ready: false,
            sync: true,
        }
    }

    pub(self) fn clear(&mut self) {
        // data
        self.t_data.clear();
        self.t_ready.clear();

        // status
        self.readys.clear();
        self.up_ready = false;
        self.sync = true;
    }

    pub(self) fn insert_suspend(&mut self, job: Rc<Job>) -> Option<Rc<Job>> {
        // t_data
        let uv = self.get_mut_uv_pad(Rc::clone(job.unit()));
        let old = uv.insert_suspend(Rc::clone(&job));

        // t_ready: wait to sync in 'reshuffle', just remark it in unit-value
        assert!(uv.is_dirty());

        // status
        self.sync = false;

        old
    }

    pub(self) fn isolate_suspends(&mut self, other: &Self) -> Vec<Rc<Job>> {
        let mut del_jobs = Vec::new();

        for (unit, uv) in self.t_data.iter() {
            // condition
            if let true = unit
                .get_config()
                .config_data()
                .borrow()
                .Unit
                .IgnoreOnIsolate
            {
                continue;
            }

            if other.t_data.get(unit).is_some() {
                continue;
            }

            // t_data
            del_jobs.append(&mut uv.flush_suspends()); // flush job
                                                       // the uv should be retained until 'reshuffle', keeping the 'dirty' information.

            // t_ready: wait to sync in 'reshuffle', just remark it in unit-value
            assert!(uv.is_dirty());
        }

        // status
        self.sync = false; // make it simple

        del_jobs
    }

    #[allow(clippy::type_complexity)]
    pub(self) fn merge_suspends(
        &mut self,
        other: &Self,
    ) -> (Vec<Rc<Job>>, Vec<Rc<Job>>, Vec<Rc<Job>>) {
        let mut add_jobs = Vec::new();
        let mut del_jobs = Vec::new();
        let mut update_jobs = Vec::new();

        for (unit, o_uv) in other.t_data.iter() {
            // t_data
            let (mut adds, mut dels, mut updates) =
                self.get_mut_uv_pad(Rc::clone(unit)).merge_suspends(o_uv);
            add_jobs.append(&mut adds);
            del_jobs.append(&mut dels);
            update_jobs.append(&mut updates);

            // t_ready: may wait to sync in 'reshuffle', just remark it in unit-value
        }

        // status
        self.sync = false; // make it simple

        (add_jobs, del_jobs, update_jobs)
    }

    pub(self) fn reshuffle(&mut self) -> Vec<Rc<Job>> {
        let mut merge_jobs = Vec::new();

        // data
        for (u, uv) in self
            .t_data
            .iter()
            .map(|(ur, uvr)| (Rc::clone(ur), Rc::clone(uvr)))
            .collect::<Vec<_>>()
            .iter()
        {
            merge_jobs.append(&mut self.reshuffle_entry(&(u, uv)));
            self.try_gc_empty_unit(&(u, uv));
        }

        // status(sync)
        self.sync = true;

        merge_jobs
    }

    pub(self) fn insert_trigger(&mut self, job: Rc<Job>) -> Option<Rc<Job>> {
        // t_data
        let uv = self.get_mut_uv_pad(Rc::clone(job.unit()));
        let old = uv.insert_trigger(Rc::clone(&job));

        // t_ready: wait to sync in 'reshuffle', just remark it in unit-value
        assert!(uv.is_dirty());

        // status
        self.sync = false;

        old
    }

    pub(self) fn finish_trigger(&mut self, unit: &UnitX, result: JobResult) -> Option<Rc<Job>> {
        assert!(self.sync);

        let (ur, uvr) = self
            .t_data
            .get_key_value(unit)
            .expect("guaranteed by caller.");
        let (u, uv) = (Rc::clone(ur), Rc::clone(uvr));
        assert!(uv.get_trigger().is_some(), "guaranteed by caller.");
        let del_trigger = self.finish_entry(&(&u, &uv), result); // status(sync): not changed
        self.try_gc_empty_unit(&(&u, &uv));

        del_trigger
    }

    pub(self) fn remove_suspends(
        &mut self,
        unit: &UnitX,
        kind1: JobKind,
        kind2: Option<JobKind>,
        result: JobResult,
    ) -> Vec<Rc<Job>> {
        assert!(self.sync);

        let mut del_jobs = Vec::new();
        if let Some((ur, uvr)) = self.t_data.get_key_value(unit) {
            let (u, uv) = (Rc::clone(ur), Rc::clone(uvr));
            del_jobs.append(&mut self.remove_entry(&(&u, &uv), kind1, kind2, result));
            self.try_gc_empty_unit(&(&u, &uv));

            // status(sync): nothing changed
        }

        del_jobs
    }

    pub(self) fn resume_unit(&mut self, unit: &UnitX) {
        if let Some((ur, uvr)) = self.t_data.get_key_value(unit) {
            let (u, uv) = (Rc::clone(ur), Rc::clone(uvr));
            self.resume_entry(&(&u, &uv)); // status(sync): not changed
        }
    }

    pub(self) fn remove_unit(&mut self, unit: &UnitX) {
        // data
        self.t_data.remove(unit);
        self.t_ready.remove(unit);

        // status(sync): nothing changed
    }

    pub(self) fn ready_pop(&mut self, unit: Option<&UnitX>) -> Option<Rc<JobUnit>> {
        // something changes, data + status
        // data
        let uv_ret = match unit {
            Some(u) => {
                self.readys_backfill();
                self.t_ready.remove(u)
            }
            None => {
                self.readys_fill();
                self.readys.pop()
            }
        };

        // status
        uv_ret.map(|uv| {
            uv.clear_up_ready();
            uv
        })
    }

    pub(self) fn update_up_ready(&mut self) {
        self.up_ready = self.calc_ready();
    }

    #[allow(dead_code)]
    pub(self) fn ready_len(&self) -> usize {
        self.t_ready.len()
    }

    pub(self) fn get_suspend(&self, unit: &UnitX, kind: JobKind) -> Option<Rc<Job>> {
        if let Some(uv) = self.t_data.get(unit) {
            uv.get_suspend(kind)
        } else {
            None
        }
    }

    pub(self) fn get_suspends(&self, unit: &UnitX) -> Vec<Rc<Job>> {
        let mut jobs = Vec::new();
        if let Some(uv) = self.t_data.get(unit) {
            jobs.append(&mut uv.get_suspends());
        }
        jobs
    }

    pub(self) fn get_all_suspends(&self) -> Vec<Rc<Job>> {
        let mut jobs = Vec::new();
        for (_, uv) in self.t_data.iter() {
            jobs.append(&mut uv.get_suspends());
        }
        jobs
    }

    pub(self) fn get_all_triggers(&self) -> Vec<Rc<Job>> {
        let mut jobs = Vec::new();
        for (_, uv) in self.t_data.iter() {
            if let Some(job) = uv.get_trigger() {
                jobs.push(job);
            }
        }
        jobs
    }

    pub(self) fn get_trigger_info(&self, unit: &UnitX) -> Option<(Rc<Job>, bool)> {
        if let Some(uv) = self.t_data.get(unit) {
            uv.get_trigger()
                .map(|trigger| (Rc::clone(&trigger), uv.is_pause()))
        } else {
            None
        }
    }

    pub(self) fn calc_ready(&self) -> bool {
        if self.sync {
            // the data has been synchronized
            // nothing -> not ready, something -> ready
            !matches!(
                (self.t_ready.is_empty(), self.readys.is_empty()),
                (true, true)
            )
        } else {
            // the data has not been synchronized, not ready
            false
        }
    }

    pub(self) fn is_sync(&self) -> bool {
        self.sync
    }

    #[allow(dead_code)]
    pub(self) fn is_empty(&self) -> bool {
        self.t_data.is_empty()
    }

    pub(self) fn is_unit_empty(&self, unit: &UnitX) -> bool {
        !self.t_data.contains_key(unit)
    }

    pub(self) fn is_suspends_conflict(&self) -> bool {
        for (_, uv) in self.t_data.iter() {
            if uv.is_suspends_conflict() {
                return true;
            }
        }

        false
    }

    pub(self) fn is_suspends_conflict_with(&self, other: &Self) -> bool {
        for (unit, uv) in self.t_data.iter() {
            if let Some(o_uv) = other.t_data.get(unit) {
                if uv.is_suspends_conflict_with(o_uv) {
                    return true;
                }
            }
        }

        false
    }

    pub(self) fn is_suspends_replace_with(&self, other: &Self) -> bool {
        for (unit, uv) in self.t_data.iter() {
            if let Some(o_uv) = other.t_data.get(unit) {
                if !uv.is_suspends_replace_with(o_uv) {
                    return false;
                }
            }
        }

        true
    }

    pub(self) fn up_ready(&self) -> bool {
        self.up_ready
    }

    pub(self) fn is_uv_runnable(&self, db: &UnitDb, uv: Rc<JobUnit>) -> bool {
        let unit = uv.get_unit();

        let atom = UnitRelationAtom::UnitAtomAfter;
        for other in db.dep_gets_atom(&unit, atom).iter() {
            if let Some(other_uv) = self.t_data.get(other) {
                if !uv.is_next_trigger_order_with(other_uv, atom) {
                    return false;
                }
            }
        }

        let atom = UnitRelationAtom::UnitAtomBefore;
        for other in db.dep_gets_atom(&unit, atom).iter() {
            if let Some(other_uv) = self.t_data.get(other) {
                if !uv.is_next_trigger_order_with(other_uv, atom) {
                    return false;
                }
            }
        }

        true
    }

    fn reshuffle_entry(&mut self, entry: &(&Rc<UnitX>, &Rc<JobUnit>)) -> Vec<Rc<Job>> {
        let mut merge_jobs = Vec::new();

        let (u, uv) = entry;
        if uv.is_dirty() {
            // reshuffle dirty unit only
            // reshuffle t_data
            merge_jobs.append(&mut uv.reshuffle());

            // synchronize t_ready
            self.ready_sync(Rc::clone(u), Rc::clone(uv));

            // dirty
            uv.clear_dirty(); // the 'dirty' entry has been synced, it's not dirty now.
        }

        merge_jobs
    }

    fn finish_entry(
        &mut self,
        entry: &(&Rc<UnitX>, &Rc<JobUnit>),
        result: JobResult,
    ) -> Option<Rc<Job>> {
        let (u, uv) = entry;

        assert!(!uv.is_dirty());

        // finish t_data
        let del_trigger = uv.finish_trigger(result);

        // synchronize t_ready
        self.ready_sync(Rc::clone(u), Rc::clone(uv));

        // dirty
        uv.clear_dirty(); // the 'dirty' entry has been synced, it's not dirty now.

        del_trigger
    }

    fn remove_entry(
        &mut self,
        entry: &(&Rc<UnitX>, &Rc<JobUnit>),
        kind1: JobKind,
        kind2: Option<JobKind>,
        result: JobResult,
    ) -> Vec<Rc<Job>> {
        let (_, uv) = entry;

        assert!(!uv.is_dirty());

        // remove t_data
        let mut del_jobs = Vec::new();
        if let Some(j1) = uv.remove_suspend(kind1, result) {
            del_jobs.push(j1);
        }
        if let Some(k2) = kind2 {
            if let Some(j2) = uv.remove_suspend(k2, result) {
                del_jobs.push(j2);
            }
        }

        // reshuffle itself
        self.reshuffle_entry(entry);

        del_jobs
    }

    fn resume_entry(&mut self, other_entry: &(&Rc<UnitX>, &Rc<JobUnit>)) {
        let (other_u, other_uv) = other_entry;

        assert!(!other_uv.is_dirty());

        if other_uv.is_pause() {
            // resume
            other_uv.resume();

            // synchronize t_ready
            self.ready_sync(Rc::clone(other_u), Rc::clone(other_uv));

            // t_data: do nothing
            // dirty
            other_uv.clear_dirty(); // the 'dirty' entry has been synced, it's not dirty now.
        }
    }

    fn ready_sync(&mut self, unit: Rc<UnitX>, uv: Rc<JobUnit>) {
        if uv.is_ready() {
            self.ready_insert(unit, uv);
        } else {
            self.ready_remove(unit, uv);
        }
    }

    fn ready_insert(&mut self, unit: Rc<UnitX>, uv: Rc<JobUnit>) {
        if !uv.is_up_ready() {
            // something changes
            self.readys_backfill();

            // data
            self.t_ready.insert(unit, Rc::clone(&uv));

            // status
            uv.set_up_ready();
        }
    }

    fn ready_remove(&mut self, unit: Rc<UnitX>, uv: Rc<JobUnit>) {
        if uv.is_up_ready() {
            // something changes
            self.readys_backfill();

            // data
            self.t_ready.remove(&unit);

            // status
            uv.clear_up_ready();
        }
    }

    fn readys_backfill(&mut self) {
        // readys -> t_ready
        if !self.readys.is_empty() {
            let readys = self.readys.iter().map(Rc::clone).collect::<Vec<_>>();
            self.readys.clear();
            for uv in readys.iter() {
                self.t_ready.insert(uv.get_unit(), Rc::clone(uv));
            }
        }
    }

    fn readys_fill(&mut self) {
        // t_ready -> readys: data
        if self.readys.is_empty() {
            self.readys = self.t_ready.values().map(Rc::clone).collect::<Vec<_>>();
            self.t_ready.clear();
        }
    }

    fn try_gc_empty_unit(&mut self, entry: &(&Rc<UnitX>, &Rc<JobUnit>)) {
        let (u, uv) = entry;
        if uv.is_empty() {
            assert!(!uv.is_ready());
            assert!(!uv.is_up_ready());
            self.t_data.remove(*u);
            self.t_ready.remove(*u);
        }
    }

    fn get_mut_uv_pad(&mut self, unit: Rc<UnitX>) -> &Rc<JobUnit> {
        // verify existence
        if self.t_data.get(&unit).is_none() {
            // nothing exists, pad it.
            self.t_data
                .insert(Rc::clone(&unit), Rc::new(JobUnit::new(Rc::clone(&unit))));
        }

        // return the one that must exist
        self.t_data
            .get(&unit)
            .expect("something inserted is not found.")
    }
}

pub(super) fn jobs_2_units(jobs: &[Rc<Job>]) -> Vec<Rc<UnitX>> {
    let mut units = HashSet::new();
    for job in jobs.iter() {
        units.insert(Rc::clone(job.unit()));
    }
    units.iter().map(Rc::clone).collect::<_>()
}

#[cfg(test)]
mod tests {
    use super::super::rentry::JobRe;
    use super::*;
    use crate::manager::RELI_HISTORY_MAX_DBS;
    use crate::unit::test_utils;
    use crate::unit::DataManager;
    use crate::unit::UnitRe;
    use core::rel::{ReliConf, Reliability};
    use event::Events;

    #[test]
    fn job_table_record_suspend() {
        let dm = Rc::new(DataManager::new());
        let reli = Rc::new(Reliability::new(
            ReliConf::new().set_max_dbs(RELI_HISTORY_MAX_DBS),
        ));
        let rentry = Rc::new(UnitRe::new(&reli));
        let db = Rc::new(UnitDb::new(&rentry));
        let job_rentry = Rc::new(JobRe::new(&reli));
        let name_test1 = String::from("test1.service");
        let unit_test1 = create_unit(&dm, &reli, &rentry, &name_test1);
        let ja = JobAlloc::new(
            &reli,
            &job_rentry,
            &Rc::new(Events::new().unwrap()),
            &Rc::new(DataManager::new()),
        );
        let table = JobTable::new(&db);

        let conf = JobConf::new(&unit_test1, JobKind::Nop);
        let new = table.record_suspend(&ja, &conf, JobMode::Replace);
        assert!(new);
    }

    fn create_unit(
        dmr: &Rc<DataManager>,
        relir: &Rc<Reliability>,
        rentryr: &Rc<UnitRe>,
        name: &str,
    ) -> Rc<UnitX> {
        log::init_log_to_console("create_unit", log::Level::Trace);
        log::info!("test");
        test_utils::create_unit_for_test_pub(dmr, relir, rentryr, name)
    }
}
