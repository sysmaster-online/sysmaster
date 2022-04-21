#![warn(unused_imports)]
use super::job_alloc::JobAlloc;
use super::job_entry::{Job, JobConf, JobInfo, JobKind, JobResult};
use super::job_unit_entry::JobUnit;
use super::JobErrno;
use crate::manager::data::{JobMode, UnitConfigItem};
use crate::manager::unit::unit_datastore::UnitDb;
use crate::manager::unit::unit_entry::UnitX;
use crate::manager::unit::unit_relation_atom::UnitRelationAtom;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

pub(super) struct JobTable {
    // key: job-id | unit, value: job
    // data
    t_id: RefCell<HashMap<u32, Rc<Job>>>, // guarantee the uniqueness of job-id
    t_unit: JobUnitTable,                 // the running time of job is organized by unit
}

impl JobTable {
    pub(super) fn new() -> JobTable {
        JobTable {
            t_id: RefCell::new(HashMap::new()),
            t_unit: JobUnitTable::new(),
        }
    }

    pub(super) fn record_suspend(
        &self,
        ja: &JobAlloc,
        config: JobConf,
        mode: JobMode,
        operate: bool,
    ) -> bool {
        let unit = config.get_unit();
        let kind = config.get_kind();

        // add job only when nothing with the same 'unit'+'kind' exists
        if let None = self.t_unit.get_suspend(unit, kind) {
            self.insert_suspend(ja.alloc(Rc::clone(unit), kind, mode), operate)
                .expect("insert a new job failed.");
            true
        } else {
            false
        }
    }

    pub(super) fn remove_suspends(
        &self,
        db: &UnitDb,
        unit: &UnitX,
        kind1: JobKind,
        kind2: Option<JobKind>,
        result: JobResult,
    ) -> Vec<Rc<Job>> {
        // table-unit
        let del_jobs = self.t_unit.remove_suspends(db, unit, kind1, kind2, result);

        // synchronize table-id
        for job in del_jobs.iter() {
            self.t_id.borrow_mut().remove(&job.get_id());
        }

        del_jobs
    }

    pub(super) fn commit(
        &self,
        other: &Self,
        mode: JobMode,
    ) -> Result<(Vec<Rc<Job>>, Vec<Rc<Job>>, Vec<Rc<Job>>), JobErrno> {
        // check other-jobs-id first: make rollback simple
        for (o_id, _) in other.t_id.borrow().iter() {
            if let Some(_) = self.t_id.borrow().get(o_id) {
                return Err(JobErrno::JobErrInternel);
            }
        }

        // isolate
        let mut del_jobs = self.isolate_suspends(other, mode);

        // merge
        let (add_jobs, mut flush_jobs, update_jobs) = self.merge_suspends(other);
        del_jobs.append(&mut flush_jobs);

        // reshuffle
        del_jobs.append(&mut self.reshuffle());

        Ok((add_jobs, del_jobs, update_jobs))
    }

    pub(super) fn try_trigger(
        &self,
        db: &UnitDb,
    ) -> Option<(Option<(JobInfo, Option<JobResult>)>, Option<Rc<Job>>)> {
        // try trigger table-unit
        let trigger_ret = self.t_unit.try_trigger(db);

        // synchronize table-id
        if let Some((_, Some(job))) = &trigger_ret {
            self.t_id.borrow_mut().remove(&job.get_id());
        }

        trigger_ret
    }

    pub(super) fn finish_trigger(
        &self,
        db: &UnitDb,
        unit: &UnitX,
        result: JobResult,
    ) -> Option<Rc<Job>> {
        // finish table-unit
        let del_trigger = self.t_unit.finish_trigger(db, unit, result);

        // synchronize table-id
        if let Some(job) = &del_trigger {
            self.t_id.borrow_mut().remove(&job.get_id());
        }

        del_trigger
    }

    pub(super) fn resume_unit(&self, unit: &UnitX) {
        // resume table-unit
        self.t_unit.resume_unit(unit);

        // synchronize table-id: nothing changed
    }

    pub(super) fn remove_unit(&self, unit: &UnitX) {
        // table-id
        for job in self.t_unit.get_suspends(unit).iter() {
            self.t_id.borrow_mut().remove(&job.get_id());
        }

        // table-unit
        self.t_unit.remove_unit(unit);
    }

    pub(super) fn clear(&self) {
        // table-id
        self.t_id.borrow_mut().clear();

        // table-unit
        self.t_unit.clear();
    }

    pub(super) fn len(&self) -> usize {
        self.t_id.borrow().len()
    }

    pub(super) fn ready_len(&self) -> usize {
        self.t_unit.ready_len()
    }

    pub(super) fn get(&self, id: u32) -> Option<JobInfo> {
        match self.t_id.borrow().get(&id) {
            Some(job) => Some(JobInfo::map(job)),
            None => None,
        }
    }

    pub(super) fn get_suspend(&self, unit: &UnitX, kind: JobKind) -> Option<JobInfo> {
        match self.t_unit.get_suspend(unit, kind) {
            Some(job) => Some(JobInfo::map(&job)),
            None => None,
        }
    }

    pub(super) fn get_trigger_info(&self, unit: &UnitX) -> Option<(JobInfo, bool)> {
        match self.t_unit.get_trigger_info(unit) {
            Some((job, pause)) => Some((JobInfo::map(&job), pause)),
            None => None,
        }
    }

    pub(super) fn is_empty(&self) -> bool {
        self.t_unit.is_empty()
    }

    pub(super) fn is_unit_empty(&self, unit: &UnitX) -> bool {
        self.t_unit.is_unit_empty(unit)
    }

    pub(super) fn is_trigger(&self, id: u32) -> bool {
        if let Some(job_info) = self.get(id) {
            if let Some((t_info, _)) = self.get_trigger_info(&job_info.unit) {
                return t_info.id == job_info.id;
            }
        }
        false
    }

    pub(super) fn is_suspend(&self, id: u32) -> bool {
        if let Some(job_info) = self.get(id) {
            if let Some(s_info) = self.get_suspend(&job_info.unit, job_info.kind) {
                return s_info.id == job_info.id;
            }
        }
        false
    }

    pub(super) fn is_suspends_conflict(&self) -> bool {
        self.t_unit.is_suspends_conflict()
    }

    pub(super) fn is_suspends_conflict_with(&self, other: &Self) -> bool {
        self.t_unit.is_suspends_conflict_with(&other.t_unit)
    }

    pub(super) fn is_suspends_replace_with(&self, other: &Self) -> bool {
        self.t_unit.is_suspends_replace_with(&other.t_unit)
    }

    pub(super) fn is_ready(&self) -> bool {
        self.t_unit.is_ready()
    }

    fn isolate_suspends(&self, other: &Self, mode: JobMode) -> Vec<Rc<Job>> {
        // isolate table-unit
        let del_jobs = match mode {
            JobMode::JobIsolate | JobMode::JobFlush => self.t_unit.isolate_suspends(&other.t_unit),
            _ => Vec::new(), // empty
        };

        // synchronize table-id
        for job in del_jobs.iter() {
            self.t_id.borrow_mut().remove(&job.get_id());
        }

        del_jobs
    }

    fn merge_suspends(&self, other: &Self) -> (Vec<Rc<Job>>, Vec<Rc<Job>>, Vec<Rc<Job>>) {
        // merge table-unit
        let (add_jobs, del_jobs, update_jobs) = self.t_unit.merge_suspends(&other.t_unit);

        // synchronize table-id
        for job in add_jobs.iter() {
            self.t_id.borrow_mut().insert(job.get_id(), Rc::clone(job));
        }
        for job in del_jobs.iter() {
            self.t_id.borrow_mut().remove(&job.get_id());
        }

        (add_jobs, del_jobs, update_jobs)
    }

    fn reshuffle(&self) -> Vec<Rc<Job>> {
        // reshuffle table-unit
        let merge_jobs = self.t_unit.reshuffle();

        // synchronize table-id
        for job in merge_jobs.iter() {
            self.t_id.borrow_mut().remove(&job.get_id());
        }

        merge_jobs
    }

    fn insert_suspend(&self, job: Rc<Job>, operate: bool) -> Result<(), JobErrno> {
        // check job-id
        let id = job.get_id();
        if let Some(_) = self.t_id.borrow().get(&id) {
            return Err(JobErrno::JobErrInternel);
        }

        // table-unit
        self.t_unit.insert_suspend(Rc::clone(&job), operate);

        // table-id
        self.t_id.borrow_mut().insert(id, job);

        Ok(())
    }
}

struct JobUnitTable {
    data: RefCell<JobUnitTableData>,
}

// the declaration "pub(self)" is for identification only.
impl JobUnitTable {
    pub(self) fn new() -> JobUnitTable {
        JobUnitTable {
            data: RefCell::new(JobUnitTableData::new()),
        }
    }

    pub(self) fn insert_suspend(&self, job: Rc<Job>, operate: bool) {
        self.data.borrow_mut().insert_suspend(job, operate)
    }

    pub(self) fn isolate_suspends(&self, other: &Self) -> Vec<Rc<Job>> {
        self.data
            .borrow_mut()
            .isolate_suspends(&other.data.borrow())
    }

    pub(self) fn merge_suspends(&self, other: &Self) -> (Vec<Rc<Job>>, Vec<Rc<Job>>, Vec<Rc<Job>>) {
        self.data.borrow_mut().merge_suspends(&other.data.borrow())
    }

    pub(self) fn reshuffle(&self) -> Vec<Rc<Job>> {
        self.data.borrow_mut().reshuffle()
    }

    pub(self) fn try_trigger(
        &self,
        db: &UnitDb,
    ) -> Option<(Option<(JobInfo, Option<JobResult>)>, Option<Rc<Job>>)> {
        assert!(self.data.borrow().is_sync());

        // table-ready: "pop_last + trigger_last" or "pop_all + trigger_all"
        // the single 'last' operation is better, but 'pop_last' is not currently supported.
        // we select the batch 'all' operation to simulate the 'single' operation now.
        self.data.borrow_mut().readys_fill();

        let uv_try = self.data.borrow_mut().readys_pop();
        if uv_try.is_some() {
            let uv = uv_try.unwrap();
            let (trigger_info, merge_trigger) = self.try_trigger_entry(db, Rc::clone(&uv)); // status(sync): not changed
            assert!(!uv.is_empty());
            return Some((trigger_info, merge_trigger));
        } else {
            return None;
        }
    }

    pub(self) fn finish_trigger(
        &self,
        db: &UnitDb,
        unit: &UnitX,
        result: JobResult,
    ) -> Option<Rc<Job>> {
        self.data.borrow_mut().finish_trigger(db, unit, result)
    }

    pub(self) fn remove_suspends(
        &self,
        db: &UnitDb,
        unit: &UnitX,
        kind1: JobKind,
        kind2: Option<JobKind>,
        result: JobResult,
    ) -> Vec<Rc<Job>> {
        self.data
            .borrow_mut()
            .remove_suspends(db, unit, kind1, kind2, result)
    }

    pub(self) fn resume_unit(&self, unit: &UnitX) {
        self.data.borrow_mut().resume_unit(unit)
    }

    pub(self) fn remove_unit(&self, unit: &UnitX) {
        self.data.borrow_mut().remove_unit(unit)
    }

    pub(self) fn clear(&self) {
        self.data.borrow_mut().clear()
    }

    pub(self) fn ready_len(&self) -> usize {
        self.data.borrow().ready_len()
    }

    pub(self) fn get_suspend(&self, unit: &UnitX, kind: JobKind) -> Option<Rc<Job>> {
        self.data.borrow().get_suspend(unit, kind)
    }

    pub(self) fn get_suspends(&self, unit: &UnitX) -> Vec<Rc<Job>> {
        self.data.borrow().get_suspends(unit)
    }

    pub(self) fn get_trigger_info(&self, unit: &UnitX) -> Option<(Rc<Job>, bool)> {
        self.data.borrow().get_trigger_info(unit)
    }

    pub(self) fn is_empty(&self) -> bool {
        self.data.borrow().is_empty()
    }

    pub(self) fn is_unit_empty(&self, unit: &UnitX) -> bool {
        self.data.borrow().is_unit_empty(unit)
    }

    pub(self) fn is_suspends_conflict(&self) -> bool {
        self.data.borrow().is_suspends_conflict()
    }

    pub(self) fn is_suspends_conflict_with(&self, other: &Self) -> bool {
        self.data
            .borrow()
            .is_suspends_conflict_with(&other.data.borrow())
    }

    pub(self) fn is_suspends_replace_with(&self, other: &Self) -> bool {
        self.data
            .borrow()
            .is_suspends_replace_with(&other.data.borrow())
    }

    pub(self) fn is_ready(&self) -> bool {
        self.data.borrow().is_ready()
    }

    fn try_trigger_entry(
        &self,
        db: &UnitDb,
        value: Rc<JobUnit>,
    ) -> (Option<(JobInfo, Option<JobResult>)>, Option<Rc<Job>>) {
        let uv = value;

        assert!(!uv.is_dirty());

        // try to trigger unit: trigger (order-allowed)it or pause (order-non-allowed)it
        let (trigger_info, merge_trigger) =
            match self.data.borrow().is_uv_runnable(Rc::clone(&uv), db) {
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
}

//#[derive(Debug)]
struct JobUnitTableData {
    // key: unit, value: jobs with order
    // data
    t_data: HashMap<Rc<UnitX>, Rc<JobUnit>>,   // quick search
    t_ready: BTreeMap<Rc<UnitX>, Rc<JobUnit>>, // quick sort for readies

    // status
    /* t_ready */
    readys: Vec<Rc<JobUnit>>, // simulate 'BTreeMap->pop_last'
    /* the entire entry */
    sync: bool, // sync flag of the entire table, including data and ready.
}

// the declaration "pub(self)" is for identification only.
impl JobUnitTableData {
    pub(self) fn new() -> JobUnitTableData {
        JobUnitTableData {
            t_data: HashMap::new(),
            t_ready: BTreeMap::new(),

            readys: Vec::new(),
            sync: false,
        }
    }

    pub(self) fn insert_suspend(&mut self, job: Rc<Job>, operate: bool) {
        // t_data
        let uv = self.get_mut_uv_pad(Rc::clone(job.get_unit()));
        uv.insert_suspend(Rc::clone(&job), operate);

        // t_ready: wait to sync in 'reshuffle', just remark it in unit-value
        assert!(uv.is_dirty());

        // status
        self.sync = false;
    }

    pub(self) fn isolate_suspends(&mut self, other: &Self) -> Vec<Rc<Job>> {
        let mut del_jobs = Vec::new();

        for (unit, uv) in self.t_data.iter() {
            // condition
            if let UnitConfigItem::UcItemIgnoreOnIsolate(true) =
                unit.get_config(&UnitConfigItem::UcItemIgnoreOnIsolate(false))
            {
                continue;
            }

            if let Some(_) = other.t_data.get(unit) {
                continue;
            }

            // t_data
            del_jobs.append(&mut uv.flush_suspends()); // flush job
                                                       // the uv should be retained until 'reshuffle', keeping the 'dirty' infomation.

            // t_ready: wait to sync in 'reshuffle', just remark it in unit-value
            assert!(uv.is_dirty());
        }

        // status
        self.sync = false; // make it simple

        del_jobs
    }

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

    pub(self) fn finish_trigger(
        &mut self,
        db: &UnitDb,
        unit: &UnitX,
        result: JobResult,
    ) -> Option<Rc<Job>> {
        assert!(self.sync);

        let (ur, uvr) = self
            .t_data
            .get_key_value(unit)
            .expect("guaranteed by caller.");
        let (u, uv) = (Rc::clone(ur), Rc::clone(uvr));
        assert!(uv.get_trigger().is_some(), "guaranteed by caller.");
        let del_trigger = self.finish_entry(db, &(&u, &uv), result); // status(sync): not changed
        self.try_gc_empty_unit(&(&u, &uv));

        del_trigger
    }

    pub(self) fn remove_suspends(
        &mut self,
        db: &UnitDb,
        unit: &UnitX,
        kind1: JobKind,
        kind2: Option<JobKind>,
        result: JobResult,
    ) -> Vec<Rc<Job>> {
        assert!(self.sync);

        let mut del_jobs = Vec::new();
        if let Some((ur, uvr)) = self.t_data.get_key_value(unit) {
            let (u, uv) = (Rc::clone(ur), Rc::clone(uvr));
            del_jobs.append(&mut self.remove_entry(db, &(&u, &uv), kind1, kind2, result));
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

    pub(self) fn clear(&mut self) {
        // data
        self.t_data.clear();
        self.t_ready.clear();

        // status
        self.sync = true;
    }

    pub(self) fn readys_fill(&mut self) {
        if self.readys.is_empty() {
            // t_ready -> readys: data + status
            self.readys = self
                .t_ready
                .values()
                .map(|uvr| Rc::clone(uvr))
                .collect::<Vec<_>>();
            self.t_ready.clear();
            for uv in self.readys.iter() {
                uv.clear_up_ready();
            }
        }
    }

    pub(self) fn readys_pop(&mut self) -> Option<Rc<JobUnit>> {
        self.readys.pop()
    }

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

    pub(self) fn get_trigger_info(&self, unit: &UnitX) -> Option<(Rc<Job>, bool)> {
        if let Some(uv) = self.t_data.get(unit) {
            if let Some(trigger) = uv.get_trigger() {
                Some((Rc::clone(&trigger), uv.is_pause()))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub(self) fn is_sync(&self) -> bool {
        self.sync
    }

    pub(self) fn is_empty(&self) -> bool {
        self.t_data.is_empty()
    }

    pub(self) fn is_unit_empty(&self, unit: &UnitX) -> bool {
        self.t_data.contains_key(unit)
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

    pub(self) fn is_ready(&self) -> bool {
        match (self.sync, self.t_ready.is_empty()) {
            (false, _) => false,     // the data has not been synchronized, not ready
            (true, empty) => !empty, // the data has been synchronized, nothing -> not ready and something -> ready
        }
    }

    pub(self) fn is_uv_runnable(&self, uv: Rc<JobUnit>, db: &UnitDb) -> bool {
        let unit = uv.get_unit();
        for other in db
            .dep_gets_atom(&unit, UnitRelationAtom::UnitAtomAfter)
            .iter()
        {
            if let Some(other_uv) = self.t_data.get(other) {
                if !uv.is_next_trigger_order_with(other_uv, UnitRelationAtom::UnitAtomAfter) {
                    return false;
                }
            }
        }
        for other in db
            .dep_gets_atom(&unit, UnitRelationAtom::UnitAtomBefore)
            .iter()
        {
            if let Some(other_uv) = self.t_data.get(other) {
                if !uv.is_next_trigger_order_with(other_uv, UnitRelationAtom::UnitAtomBefore) {
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
        db: &UnitDb,
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

        // resume order-related units
        for other in db.dep_gets_atom(u, UnitRelationAtom::UnitAtomAfter).iter() {
            self.resume_unit(other);
        }
        for other in db.dep_gets_atom(u, UnitRelationAtom::UnitAtomBefore).iter() {
            self.resume_unit(other);
        }

        del_trigger
    }

    fn remove_entry(
        &mut self,
        db: &UnitDb,
        entry: &(&Rc<UnitX>, &Rc<JobUnit>),
        kind1: JobKind,
        kind2: Option<JobKind>,
        result: JobResult,
    ) -> Vec<Rc<Job>> {
        let (u, uv) = entry;

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

        // synchronize t_ready
        self.ready_sync(Rc::clone(u), Rc::clone(uv));

        // dirty
        uv.clear_dirty(); // the 'dirty' entry has been synced, it's not dirty now.

        // resume order-related units
        for other in db.dep_gets_atom(u, UnitRelationAtom::UnitAtomAfter).iter() {
            self.resume_unit(other);
        }
        for other in db.dep_gets_atom(u, UnitRelationAtom::UnitAtomBefore).iter() {
            self.resume_unit(other);
        }

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

    fn readys_backfill(&mut self) {
        if !self.readys.is_empty() {
            // readys -> t_ready: data + status
            let readys = self
                .readys
                .iter()
                .map(|uvr| Rc::clone(uvr))
                .collect::<Vec<_>>();
            self.readys.clear();
            for uv in readys.iter() {
                self.ready_sync(uv.get_unit(), Rc::clone(uv));
            }
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
            self.readys_backfill(); // something changes

            // data
            self.t_ready.insert(unit, Rc::clone(&uv));

            // status
            uv.set_up_ready();
        }
    }

    fn ready_remove(&mut self, unit: Rc<UnitX>, uv: Rc<JobUnit>) {
        if uv.is_up_ready() {
            self.readys_backfill(); // something changes

            // data
            self.t_ready.remove(&unit);

            // status
            uv.clear_up_ready();
        }
    }

    fn try_gc_empty_unit(&mut self, entry: &(&Rc<UnitX>, &Rc<JobUnit>)) {
        let (u, uv) = entry;
        if uv.is_empty() {
            self.t_data.remove(*u);
            self.t_ready.remove(*u);
        }
    }

    fn get_mut_uv_pad(&mut self, unit: Rc<UnitX>) -> &Rc<JobUnit> {
        // verify existance
        if let None = self.t_data.get(&unit) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manager::data::{DataManager, UnitType};
    use crate::manager::unit::unit_file::UnitFile;
    use crate::manager::unit::unit_parser_mgr::UnitParserMgr;
    use crate::plugin::Plugin;
    use std::path::PathBuf;
    use utils::logger;

    #[test]
    fn job_table_record_suspend() {
        let name_test1 = String::from("test1.service");
        let unit_test1 = create_unit(&name_test1);
        let mut ja = JobAlloc::new();
        let table = JobTable::new();

        let new = table.record_suspend(
            &mut ja,
            JobConf::new(Rc::clone(&unit_test1), JobKind::JobNop),
            JobMode::JobReplace,
            false,
        );
        assert_eq!(new, true);
    }

    fn create_unit(name: &str) -> Rc<UnitX> {
        logger::init_log_with_console("test_unit_load", 4);
        log::info!("test");
        let dm = Rc::new(DataManager::new());
        let file = Rc::new(UnitFile::new());
        let unit_conf_parser_mgr = Rc::new(UnitParserMgr::default());
        let unit_type = UnitType::UnitService;
        let plugins = Rc::clone(&Plugin::get_instance());
        let mut config_path1 = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        config_path1.push("../target/debug");
        plugins
            .borrow_mut()
            .set_library_dir(&config_path1.to_str().unwrap());
        plugins.borrow_mut().load_lib();
        let mut config_path2 = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        config_path2.push("../target/release");
        plugins
            .borrow_mut()
            .set_library_dir(&config_path2.to_str().unwrap());
        plugins.borrow_mut().load_lib();
        let subclass = plugins.borrow().create_unit_obj(unit_type).unwrap();
        Rc::new(UnitX::new(
            dm,
            file,
            unit_conf_parser_mgr,
            unit_type,
            name,
            subclass.into_unitobj(),
        ))
    }
}
