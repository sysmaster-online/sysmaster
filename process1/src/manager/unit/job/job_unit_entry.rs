#![warn(unused_imports)]
use super::job_entry::{Job, JobAttrKind, JobInfo, JobKind, JobResult, JobStage};
use crate::manager::data::UnitActiveState;
use crate::manager::unit::unit_base::UnitRelationAtom;
use crate::manager::unit::unit_entry::UnitX;
use std::cell::RefCell;
use std::collections::{HashMap, LinkedList};
use std::rc::Rc;

const JOBUNIT_SQ_MUTOP_MAX_NUM: usize = 1; // stop or (restart|start|reload), which can change the unit's stage
const JOBUNIT_SQ_MAX_NUM: usize = 3; // [stop] | [(restart|start|reload)->verify->nop]

pub(super) struct JobUnit {
    data: RefCell<JobUnitData>,
}

impl JobUnit {
    pub(super) fn new(unit: Rc<UnitX>) -> JobUnit {
        JobUnit {
            data: RefCell::new(JobUnitData::new(unit)),
        }
    }

    pub(super) fn insert_suspend(&self, job: Rc<Job>, operate: bool) {
        self.data.borrow_mut().insert_suspend(job, operate)
    }

    pub(super) fn remove_suspend(&self, kind: JobKind, result: JobResult) -> Option<Rc<Job>> {
        self.data.borrow_mut().remove_suspend(kind, result)
    }

    pub(super) fn flush_suspends(&self) -> Vec<Rc<Job>> {
        self.data.borrow_mut().flush_suspends()
    }

    pub(super) fn merge_suspends(
        &self,
        other: &Self,
    ) -> (Vec<Rc<Job>>, Vec<Rc<Job>>, Vec<Rc<Job>>) {
        self.data.borrow_mut().merge_suspends(&other.data.borrow())
    }

    pub(super) fn reshuffle(&self) -> Vec<Rc<Job>> {
        self.data.borrow_mut().reshuffle()
    }

    pub(super) fn pause(&self) {
        self.data.borrow_mut().pause()
    }

    pub(super) fn resume(&self) {
        self.data.borrow_mut().resume()
    }

    pub(super) fn do_trigger(&self) -> (Option<(JobInfo, Option<JobResult>)>, Option<Rc<Job>>) {
        let (cur_trigger, merge_trigger) = self.data.borrow_mut().do_next_trigger();

        // record current infomation of the trigger first, which(run_kind + stage) could be changed after running.
        let t_jinfo = JobInfo::map(&cur_trigger);

        // operate job
        let tfinish_result = match cur_trigger.run() {
            // run the trigger
            Ok(_) => None, // trigger successful
            Err(None) => {
                self.data.borrow_mut().pause();
                self.data.borrow_mut().retrigger_trigger();
                None
            } // trigger failed, but need to be retriggered again
            Err(Some(tfinish_r)) => Some(tfinish_r), // trigger failed, and need to be finished
        };

        if let Some(job) = &merge_trigger {
            // finish merged job
            job.finish(JobResult::JobMerged);
        }

        (Some((t_jinfo, tfinish_result)), merge_trigger)
    }

    pub(super) fn finish_trigger(&self, result: JobResult) -> Option<Rc<Job>> {
        self.data.borrow_mut().finish_trigger(result)
    }

    pub(super) fn clear_dirty(&self) {
        self.data.borrow_mut().clear_dirty()
    }

    pub(super) fn set_up_ready(&self) {
        self.data.borrow_mut().set_up_ready()
    }

    pub(super) fn clear_up_ready(&self) {
        self.data.borrow_mut().clear_up_ready()
    }

    pub(super) fn len(&self) -> usize {
        self.data.borrow().len()
    }

    pub(super) fn get_suspend(&self, kind: JobKind) -> Option<Rc<Job>> {
        self.data.borrow().get_suspend(kind)
    }

    pub(super) fn get_suspends(&self) -> Vec<Rc<Job>> {
        self.data.borrow().get_suspends()
    }

    pub(super) fn get_trigger(&self) -> Option<Rc<Job>> {
        self.data.borrow().get_trigger()
    }

    pub(super) fn get_unit(&self) -> Rc<UnitX> {
        self.data.borrow().get_unit()
    }

    pub(super) fn is_empty(&self) -> bool {
        self.data.borrow().is_empty()
    }

    pub(super) fn is_ready(&self) -> bool {
        self.data.borrow().is_ready()
    }

    pub(super) fn is_up_ready(&self) -> bool {
        self.data.borrow().is_up_ready()
    }

    pub(super) fn is_dirty(&self) -> bool {
        self.data.borrow().is_dirty()
    }

    pub(super) fn is_pause(&self) -> bool {
        self.data.borrow().is_pause()
    }

    pub(super) fn is_suspends_conflict(&self) -> bool {
        self.data.borrow().is_suspends_conflict()
    }

    pub(super) fn is_suspends_conflict_with(&self, other: &Self) -> bool {
        self.data
            .borrow()
            .is_suspends_conflict_with(&other.data.borrow())
    }

    pub(super) fn is_suspends_replace_with(&self, other: &Self) -> bool {
        self.data
            .borrow()
            .is_suspends_replace_with(&other.data.borrow())
    }

    pub(super) fn is_next_trigger_order_with(&self, other: &Self, atom: UnitRelationAtom) -> bool {
        self.data
            .borrow()
            .is_next_trigger_order_with(&other.data.borrow(), atom)
    }
}

//#[derive(Debug)]
struct JobUnitData {
    // key
    unit: Rc<UnitX>,

    // data
    /* jobs: the uniqueness of job-id is guaranteed by upper level like JobTable */
    suspends: HashMap<JobKind, Rc<Job>>, // key: kind, value: the 'init' or 'wait' one
    trigger: Option<Rc<Job>>,            // the 'running' or 'end' one, which has been triggered

    // status
    /* suspends */
    order: bool,
    sq: LinkedList<Rc<Job>>, // order: [stop] | [(restart|start|reload)->verify->nop]
    /* trigger */
    interrupt: bool, // interrupt flag of the triggered one coming from the first suspended one
    retrigger: bool,
    /* the entire entry */
    dirty: bool,
    pause: bool,
    ready: bool,
    up_ready: bool, // 'ready' status in up-level
}

// the declaration "pub(self)" is for identification only.
impl JobUnitData {
    pub(self) fn new(unit: Rc<UnitX>) -> JobUnitData {
        JobUnitData {
            unit,

            suspends: HashMap::new(),
            trigger: None,

            order: false,
            sq: LinkedList::new(),
            interrupt: false,
            retrigger: false,
            dirty: false,
            pause: false,
            ready: false,
            up_ready: false,
        }
    }

    pub(self) fn insert_suspend(&mut self, job: Rc<Job>, operate: bool) {
        assert!(job.is_basic_op());
        assert_eq!(job.get_stage(), JobStage::JobInit);
        assert!(!self.is_trigger(&job));

        // suspends
        /* data */
        let old = self.suspends.insert(job.get_kind(), Rc::clone(&job));
        assert_eq!(old, None);
        /* status */
        self.order = false;

        // trigger: do nothing
        // the entire entry: data(delayed) + status
        self.dirty = true;
        self.update_ready();

        // operate job
        if operate {
            job.wait(); // wait suspended job
        }
    }

    pub(self) fn remove_suspend(&mut self, kind: JobKind, result: JobResult) -> Option<Rc<Job>> {
        // suspends
        /* data */
        let del_job = self.suspends.remove(&kind);
        /* status */
        self.order = false;

        // trigger: do nothing
        // the entire entry: data(delayed) + status
        self.dirty = true;
        self.update_ready();

        // operate job
        if let Some(job) = &del_job {
            // finish deleted job
            job.finish(result);
        }

        del_job
    }

    pub(self) fn flush_suspends(&mut self) -> Vec<Rc<Job>> {
        // suspends
        /* data */
        let del_jobs = self
            .suspends
            .values()
            .map(|jr| Rc::clone(jr))
            .collect::<Vec<_>>();
        self.suspends.clear();
        /* status */
        self.order = false;

        // trigger: do nothing
        // the entire entry: data(delayed) + status
        self.dirty = true;
        self.update_ready();

        // operate job
        for job in del_jobs.iter() {
            // finish deleted job
            job.finish(JobResult::JobCancelled);
        }

        del_jobs
    }

    pub(self) fn merge_suspends(
        &mut self,
        other: &Self,
    ) -> (Vec<Rc<Job>>, Vec<Rc<Job>>, Vec<Rc<Job>>) {
        // conflicting, replace(flush and add) it; non-conflicting, union it;
        let mut add_jobs = Vec::new();
        let mut del_jobs = Vec::new();
        let mut update_jobs = Vec::new();

        // flush on conflict
        if self.is_suspends_conflict_with(other) {
            del_jobs.append(&mut self.flush_suspends());
        }

        // union
        for (_, o_job) in other.suspends.iter() {
            if let Some(job) = self.get_suspend(o_job.get_kind()) {
                // merge other-job
                job.merge_attr(o_job);
                update_jobs.push(job);
            } else {
                // add other-job
                self.insert_suspend(Rc::clone(&o_job), true);
                add_jobs.push(Rc::clone(&o_job));
            }
        }

        (add_jobs, del_jobs, update_jobs)
    }

    pub(self) fn reshuffle(&mut self) -> Vec<Rc<Job>> {
        assert!(!self.is_suspends_conflict()); // only the non-suspends-conflicting unit can be reshuffled

        let mut merge_jobs = Vec::new();

        // suspends
        if !self.order {
            // data
            self.jobs_merge_suspend(&mut merge_jobs); // merge jobs between suspends
            self.sq_order(); // order sq
                             // status
            self.order = true;
        }

        // trigger: data(delayed) + status
        self.jobs_merge_trigger_prepare(); // merge job between trigger and suspends

        // the entire entry: status-only
        /* dirty: not changed */
        self.update_ready();

        // operate job
        for job in merge_jobs.iter() {
            // finish merged job
            job.finish(JobResult::JobMerged);
        }

        merge_jobs
    }

    pub(self) fn pause(&mut self) {
        // suspends: do nothing
        assert!(self.order);
        // trigger: do nothing
        // the entire entry: status-only
        self.dirty = true; // make it simple
        self.pause = true;
        self.update_ready();
    }

    pub(self) fn resume(&mut self) {
        // suspends: do nothing
        assert!(self.order);
        // trigger: do nothing
        // the entire entry: status-only
        self.dirty = true; // make it simple
        self.pause = false;
        self.update_ready();
    }

    pub(self) fn do_next_trigger(&mut self) -> (Rc<Job>, Option<Rc<Job>>) {
        // trigger the next
        match self.calc_ready() {
            Some(s) if s => self.do_next_trigger_suspend(),
            Some(s) if !s => self.do_next_trigger_retrigger(),
            _ => unreachable!("the non-ready entry is triggered."),
        }
    }

    pub(self) fn retrigger_trigger(&mut self) {
        assert!(self.trigger.is_some());

        // trigger: status-only
        self.retrigger = true;

        // the entire entry: status-only
        self.dirty = true;
        self.update_ready();

        // suspends: do nothing
        assert!(self.order);
    }

    pub(self) fn finish_trigger(&mut self, result: JobResult) -> Option<Rc<Job>> {
        assert!(self.trigger.is_some());

        // trigger: data + status
        let del_trigger = match self.trigger.as_ref().cloned().unwrap().finish(result) {
            // finish the trigger
            true => {
                self.retrigger_trigger();
                None
            } // the trigger one is needed to re-triggered, which could not be deleted now.
            false => {
                self.interrupt = false;
                self.trigger.take()
            } // it really needs to be finished, finish(delete) it.
        };

        // the entire entry: status-only
        self.dirty = true;
        self.update_ready();

        // suspends: do nothing
        assert!(self.order);

        del_trigger
    }

    pub(self) fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    pub(self) fn set_up_ready(&mut self) {
        self.up_ready = true;
    }

    pub(self) fn clear_up_ready(&mut self) {
        self.up_ready = false;
    }

    pub(self) fn len(&self) -> usize {
        let num_trigger: usize = self.trigger.is_some().into();
        let num_suspend = self.suspends.len();
        num_trigger + num_suspend
    }

    pub(self) fn get_suspend(&self, kind: JobKind) -> Option<Rc<Job>> {
        self.suspends.get(&kind).cloned()
    }

    pub(self) fn get_suspends(&self) -> Vec<Rc<Job>> {
        self.suspends
            .iter()
            .map(|(_, jr)| Rc::clone(jr))
            .collect::<Vec<_>>()
    }

    pub(self) fn get_trigger(&self) -> Option<Rc<Job>> {
        self.trigger.as_ref().cloned()
    }

    pub(self) fn get_unit(&self) -> Rc<UnitX> {
        Rc::clone(&self.unit)
    }

    pub(self) fn is_empty(&self) -> bool {
        self.trigger.is_none() && self.suspends.is_empty()
    }

    pub(self) fn is_ready(&self) -> bool {
        self.ready
    }

    pub(self) fn is_up_ready(&self) -> bool {
        self.up_ready
    }

    pub(self) fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub(self) fn is_pause(&self) -> bool {
        self.pause
    }

    pub(self) fn is_suspends_conflict(&self) -> bool {
        // 'stop' can be not conflicting with 'nop' only
        let num_stop = self.suspends_kind_len(JobKind::JobStop);
        let num_others = self.suspends.len() - num_stop - self.suspends_kind_len(JobKind::JobNop);
        match (num_stop, num_others) {
            (s, o) if s > 0 && o > 0 => true, // 'stop' exists, and others except 'nop' exist
            _ => false,                       // no 'stop' exists, or no others except 'nop' exist
        }
    }

    pub(self) fn is_suspends_conflict_with(&self, other: &Self) -> bool {
        // 'stop' can be not conflicting with 'nop' only
        let stop_s = self.suspends_kind_len(JobKind::JobStop);
        let others_s = self.suspends.len() - stop_s - self.suspends_kind_len(JobKind::JobNop);
        let stop_o = other.suspends_kind_len(JobKind::JobStop);
        let others_o = other.suspends.len() - stop_o - other.suspends_kind_len(JobKind::JobNop);
        match (stop_s + stop_o, others_s + others_o) {
            (s, o) if s > 0 && o > 0 => true, // 'stop' exists, and others except 'nop' exist
            _ => false,                       // no 'stop' exists, or no others except 'nop' exist
        }
    }

    pub(self) fn is_suspends_replace_with(&self, other: &Self) -> bool {
        assert!(!self.is_suspends_conflict() && !other.is_suspends_conflict()); // both sides are not conflicting

        // can 'other' replace 'self'?
        let stop_s = self.suspends_kind_len(JobKind::JobStop);
        let others_s = self.suspends.len() - stop_s - self.suspends_kind_len(JobKind::JobNop);
        let stop_o = other.suspends_kind_len(JobKind::JobStop);
        let others_o = other.suspends.len() - stop_o - other.suspends_kind_len(JobKind::JobNop);
        match (stop_s, others_s, stop_o, others_o) {
            (_, os, so, _) if os > 0 && so > 0 => !self.is_suspends_irreversible(),
            (ss, _, _, oo) if ss > 0 && oo > 0 => !self.is_suspends_irreversible(),
            _ => true,
        }
    }

    pub(self) fn is_next_trigger_order_with(&self, other: &Self, atom: UnitRelationAtom) -> bool {
        assert!(self.ready && other.order);

        let job = self.get_next_trigger();
        if job.get_attr(JobAttrKind::JobIgnoreOrder) {
            return true;
        }

        if job.get_kind() == JobKind::JobNop {
            return true;
        }

        // compare order
        for other_job in other.get_suspends().iter() {
            // suspends
            if job.is_order_with(other_job, atom) > 0 {
                return false;
            }
        }
        if let Some(other_job) = &other.trigger {
            // trigger
            if job.is_order_with(other_job, atom) > 0 {
                return false;
            }
        }

        true
    }

    fn do_next_trigger_retrigger(&mut self) -> (Rc<Job>, Option<Rc<Job>>) {
        assert!(self.trigger.is_some() && self.retrigger);

        // trigger: status-only
        let next_trigger = self.trigger.as_ref().cloned().unwrap(); // trigger again
        assert!(!self.interrupt);
        self.retrigger = false; // it has been re-triggered above

        // the entire entry: status-only
        self.dirty = true; // make it simple
        self.update_ready();

        // suspends: do nothing
        assert!(self.order);

        (next_trigger, None)
    }

    fn get_next_trigger_retrigger(&self) -> Rc<Job> {
        assert!(self.trigger.is_some() && self.retrigger);
        self.trigger.as_ref().cloned().unwrap()
    }

    fn do_next_trigger_suspend(&mut self) -> (Rc<Job>, Option<Rc<Job>>) {
        assert!(!self.sq.is_empty());

        // trigger: data + status(interrupt)
        let merge_trigger = match self.interrupt {
            true => {
                self.interrupt = false;
                self.trigger.take()
            } // interrupt the triggered job, trigger => the first suspend('stop' | 'restart')
            false => None,
        };

        // the entire entry: data + status
        let next_trigger = Rc::clone(self.trigger.insert(self.sq.pop_front().unwrap())); // trigger the first suspend one
        self.suspends.remove(&next_trigger.get_kind()); // remove the first suspend one
        self.dirty = true; // make it simple
        self.update_ready();

        // suspends: status-only
        assert!(self.order);

        (next_trigger, merge_trigger)
    }

    fn get_next_trigger_suspend(&self) -> Rc<Job> {
        assert!(!self.sq.is_empty());
        self.sq.front().cloned().unwrap()
    }

    fn jobs_merge_suspend(&mut self, del_jobs: &mut Vec<Rc<Job>>) {
        assert!(!self.order); // the suspends can only be merged before ordering

        // merge jobs between suspends
        if !self.suspends.contains_key(&JobKind::JobStop) {
            // no 'stop' exists
            let restart = self.suspends.contains_key(&JobKind::JobRestart);
            let start = self.suspends.contains_key(&JobKind::JobStart);
            let reload = self.suspends.contains_key(&JobKind::JobReload);
            match (restart, start, reload) {
                (true, _, _) => self.jobs_ms_start_and_reload(del_jobs), // 'restart' exists, ('reload' | 'start') => 'restart'
                (false, true, true) => self.jobs_ms_start_or_reload(del_jobs), // no 'restart' exists, 'start' <=or=> 'reload'
                _ => {}                                                        // nothing to merge
            }
        } else { // 'stop' exists
             // 'stop' exists, others are all conflicting
        }
    }

    fn sq_order(&mut self) {
        // order: [stop] | [(restart|start|reload)->verify->nop]
        self.sq.clear();

        if !self.suspends.contains_key(&JobKind::JobStop) {
            // no 'stop' exists
            // (restart|start|reload)->verify->nop
            self.sq_order_pushback(JobKind::JobRestart);
            self.sq_order_pushback(JobKind::JobStart);
            self.sq_order_pushback(JobKind::JobReload);
            assert!(
                self.sq.len() <= JOBUNIT_SQ_MUTOP_MAX_NUM,
                "The merge mechanism is not working properly."
            );

            self.sq_order_pushback(JobKind::JobVerify);
            self.sq_order_pushback(JobKind::JobNop);
            assert!(
                self.sq.len() <= JOBUNIT_SQ_MAX_NUM,
                "The merge mechanism is not working properly."
            );
        } else {
            // 'stop' exists
            self.sq_order_pushback(JobKind::JobStop);
        }
    }

    fn jobs_merge_trigger_prepare(&mut self) {
        assert!(self.order); // the triggered one can only be merged after ordering

        // merge job between the trigger job and the first suspend job
        // status-only: the triggered job could be interrupted at next trigger time only, so we remark it now.
        if self.trigger.is_some() && self.sq.front().is_some() {
            // both jobs involved exist
            self.interrupt = match self.sq.front().unwrap().get_kind() {
                JobKind::JobStop | JobKind::JobRestart => true, // the first(ready) suspend one has 'stop' ability, it's allowed.
                _ => false,                                     // other kinds are not allowed
            };
        }
    }

    fn jobs_ms_start_and_reload(&mut self, del_jobs: &mut Vec<Rc<Job>>) {
        // ('reload' | 'start') => 'restart'
        self.jobs_suspends_remove(JobKind::JobStart, del_jobs);
        self.jobs_suspends_remove(JobKind::JobReload, del_jobs);
    }

    fn jobs_ms_start_or_reload(&mut self, del_jobs: &mut Vec<Rc<Job>>) {
        // 'start' <=or=> 'reload'
        let us_is_active_or_reloading = match self.unit.active_state() {
            UnitActiveState::UnitActive | UnitActiveState::UnitReloading => true,
            _ => false,
        };
        if us_is_active_or_reloading {
            // 'start' => 'reload'
            self.jobs_suspends_remove(JobKind::JobStart, del_jobs);
        } else {
            // 'reload' => 'start'
            self.jobs_suspends_remove(JobKind::JobReload, del_jobs);
        }
    }

    fn jobs_suspends_remove(&mut self, kind: JobKind, del_jobs: &mut Vec<Rc<Job>>) {
        if let Some(job) = self.suspends.remove(&kind) {
            // something has been removed
            del_jobs.push(job);
        }
    }

    fn sq_order_pushback(&mut self, kind: JobKind) {
        // copy the job to self.sq
        if let Some(job) = self.suspends.get(&kind) {
            self.sq.push_back(Rc::clone(job));
        }
    }

    fn suspends_kind_len(&self, kind: JobKind) -> usize {
        self.suspends.contains_key(&kind).into() // bool -> 0 or 1
    }

    fn update_ready(&mut self) {
        self.ready = match self.calc_ready() {
            Some(_) => true,
            None => false,
        };
    }

    fn get_next_trigger(&self) -> Rc<Job> {
        match self.calc_ready() {
            Some(s) if s => self.get_next_trigger_suspend(),
            Some(s) if !s => self.get_next_trigger_retrigger(),
            _ => unreachable!("the non-ready entry is triggered."),
        }
    }

    fn calc_ready(&self) -> Option<bool> {
        if !self.pause {
            // the entry has not been paused
            self.calc_natural_ready()
        } else {
            // the entry has been paused. Keep waiting.
            None
        }
    }

    fn calc_natural_ready(&self) -> Option<bool> {
        if self.order {
            // the things waiting to be triggered have been ordered
            match (!self.trigger.is_none(), !self.sq.is_empty()) {
                (true, true) => self.calc_natural_ready_ts(),
                (true, false) => self.calc_natural_ready_t(),
                (false, true) => self.calc_natural_ready_s(),
                (false, false) => None, // nothing triggered or waiting to be triggered exists.
            }
        } else {
            // suspends have not been ordered. Keep waiting.
            None
        }
    }

    fn calc_natural_ready_ts(&self) -> Option<bool> {
        // something triggered is not over yet, and something is waiting to be triggered.
        match (self.interrupt, self.retrigger) {
            (true, _) => Some(true), // the triggered one should be interrupted. The first suspend is ready now.
            (false, true) => Some(false), // the triggered one needs to be re-triggered. The trigger one is ready again.
            (false, false) => None, // the triggered one does not need to be re-triggered. Keep waiting.
        }
    }

    fn calc_natural_ready_t(&self) -> Option<bool> {
        // something triggered is not over yet, but nothing is waiting to be triggered.
        assert!(!self.interrupt); // there is no interrupter
        match self.retrigger {
            true => Some(false), // the triggered one needs to be re-triggered. The trigger one is ready again.
            false => None, // the triggered one does not need to be re-triggered. Keep waiting.
        }
    }

    fn calc_natural_ready_s(&self) -> Option<bool> {
        // nothing triggered exists, but something is waiting to be triggered.
        assert!(!self.interrupt); // there is nothing to interrupt
        Some(true) // there is nothing blocking suspends. The first syspend is ready now.
    }

    fn is_suspends_irreversible(&self) -> bool {
        for (_, job) in self.suspends.iter() {
            if job.get_attr(JobAttrKind::JobIrreversible) {
                return true;
            }
        }

        false
    }

    fn is_trigger(&self, job: &Job) -> bool {
        match &self.trigger {
            Some(t) => t.as_ref() == job,
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manager::data::DataManager;
    use crate::manager::unit::uload_util::UnitFile;
    use crate::manager::unit::unit_base::{JobMode, UnitType};
    use crate::manager::unit::unit_entry::UnitX;
    use crate::plugin::Plugin;
    use utils::logger;

    #[test]
    fn juv_api_len() {
        let name_test1 = String::from("test1.service");
        let unit_test1 = create_unit(&name_test1);
        let mut id: u32 = 0;
        id = id.wrapping_add(1); // ++
        let job = Rc::new(Job::new(id, Rc::clone(&unit_test1), JobKind::JobStart));
        let uv = JobUnit::new(Rc::clone(&unit_test1));

        // nothing exists
        assert_eq!(uv.len(), 0);

        // something exists
        uv.insert_suspend(job, false);
        assert_eq!(uv.len(), 1);
    }

    #[test]
    fn juv_api_merge() {
        let name_test1 = String::from("test1.service");
        let unit_test1 = create_unit(&name_test1);
        let (_, job_start, _, _, _) = prepare_jobs(&unit_test1, JobMode::JobReplace);
        let (_, stage_start, _, _, _) = prepare_jobs(&unit_test1, JobMode::JobReplace);
        let jobs = JobUnit::new(Rc::clone(&unit_test1));

        // merge nothing
        let stage = JobUnit::new(Rc::clone(&unit_test1));
        stage.insert_suspend(Rc::clone(&job_start), false);
        assert_eq!(stage.len(), 1);
        let (add_jobs, del_jobs, update_jobs) = jobs.merge_suspends(&stage);
        let ret = jobs.reshuffle();
        assert_eq!(ret.len(), 0);
        assert_eq!(jobs.len(), 1);
        assert_eq!(add_jobs.len(), 1);
        assert_eq!(del_jobs.len(), 0);
        assert_eq!(update_jobs.len(), 0);

        // merge something
        let stage = JobUnit::new(Rc::clone(&unit_test1));
        stage.insert_suspend(Rc::clone(&stage_start), false);
        assert_eq!(stage.len(), 1);
        let (add_jobs, del_jobs, update_jobs) = jobs.merge_suspends(&stage);
        let ret = jobs.reshuffle();
        assert_eq!(ret.len(), 0);
        assert_eq!(jobs.len(), 1);
        assert_eq!(add_jobs.len(), 0);
        assert_eq!(del_jobs.len(), 0);
        assert_eq!(update_jobs.len(), 1);
    }

    #[test]
    fn juv_api_reshuffle() {
        let name_test1 = String::from("test1.service");
        let unit_test1 = create_unit(&name_test1);
        let (job_nop, job_start, job_reload, job_restart, _) =
            prepare_jobs(&unit_test1, JobMode::JobReplace);
        let uv = JobUnit::new(Rc::clone(&unit_test1));

        // nothing
        let ret = uv.reshuffle();
        assert_eq!(ret.len(), 0);

        // reload+nop
        uv.insert_suspend(Rc::clone(&job_nop), true);
        uv.insert_suspend(Rc::clone(&job_reload), true);
        let ret = uv.reshuffle();
        assert_eq!(ret.len(), 0);
        assert_eq!(uv.len(), 2);
        let job = uv.data.borrow().get_next_trigger();
        assert_eq!(job.get_id(), job_reload.get_id());

        // start+reload
        uv.insert_suspend(Rc::clone(&job_start), true);
        let ret = uv.reshuffle();
        assert_eq!(ret.len(), 1);
        assert_eq!(uv.len(), 2);
        let job = uv.data.borrow().get_next_trigger();
        assert_eq!(job.get_id(), job_start.get_id());

        // restart+start
        uv.insert_suspend(Rc::clone(&job_restart), true);
        let ret = uv.reshuffle();
        assert_eq!(ret.len(), 1);
        assert_eq!(uv.len(), 2);
        let job = uv.data.borrow().get_next_trigger();
        assert_eq!(job.get_id(), job_restart.get_id());
    }

    #[test]
    fn juv_api_replace_with_unirreversible() {
        let name_test1 = String::from("test1.service");
        let unit_test1 = create_unit(&name_test1);
        let mode = JobMode::JobReplace;
        let (_, uv_start, _, _, uv_stop) = prepare_jobs(&unit_test1, mode);
        let (_, o_start, _, _, o_stop) = prepare_jobs(&unit_test1, mode);
        let uv = JobUnit::new(Rc::clone(&unit_test1));
        let other = JobUnit::new(Rc::clone(&unit_test1));

        // nothing vs nothing
        assert_eq!(uv.is_suspends_replace_with(&other), true);

        // stop vs nothing
        uv.insert_suspend(Rc::clone(&uv_stop), true);
        assert_eq!(uv.is_suspends_replace_with(&other), true);

        // stop vs stop
        other.insert_suspend(Rc::clone(&o_stop), true);
        assert_eq!(uv.is_suspends_replace_with(&other), true);

        // non-stop vs stop
        let uv = JobUnit::new(Rc::clone(&unit_test1));
        uv.insert_suspend(Rc::clone(&uv_start), true);
        assert_eq!(uv.is_suspends_replace_with(&other), true);

        // non-stop vs nothing
        let other = JobUnit::new(Rc::clone(&unit_test1));
        assert_eq!(uv.is_suspends_replace_with(&other), true);

        // non-stop vs non-stop
        other.insert_suspend(Rc::clone(&o_start), true);
        assert_eq!(uv.is_suspends_replace_with(&other), true);
    }

    #[test]
    fn juv_api_replace_with_irreversible() {
        let name_test1 = String::from("test1.service");
        let unit_test1 = create_unit(&name_test1);
        let mode = JobMode::JobReplaceIrreversible;
        let (_, uv_start, _, _, uv_stop) = prepare_jobs(&unit_test1, mode);
        let (_, o_start, _, _, o_stop) = prepare_jobs(&unit_test1, mode);
        let uv = JobUnit::new(Rc::clone(&unit_test1));
        let other = JobUnit::new(Rc::clone(&unit_test1));

        // nothing vs nothing
        assert_eq!(uv.is_suspends_replace_with(&other), true);

        // stop vs nothing
        uv.insert_suspend(Rc::clone(&uv_stop), true);
        assert_eq!(uv.is_suspends_replace_with(&other), true);

        // stop vs stop
        other.insert_suspend(Rc::clone(&o_stop), true);
        assert_eq!(uv.is_suspends_replace_with(&other), true);

        // non-stop vs stop
        let uv = JobUnit::new(Rc::clone(&unit_test1));
        uv.insert_suspend(Rc::clone(&uv_start), true);
        assert_eq!(uv.is_suspends_replace_with(&other), false);

        // non-stop vs nothing
        let other = JobUnit::new(Rc::clone(&unit_test1));
        assert_eq!(uv.is_suspends_replace_with(&other), true);

        // non-stop vs non-stop
        other.insert_suspend(Rc::clone(&o_start), true);
        assert_eq!(uv.is_suspends_replace_with(&other), true);
    }

    #[test]
    fn juv_api_order_with_unignore() {
        let name_test1 = String::from("test1.service");
        let unit_test1 = create_unit(&name_test1);
        let mode = JobMode::JobReplace;
        let (uv_nop, uv_start, _, _, uv_stop) = prepare_jobs(&unit_test1, mode);
        let uv = JobUnit::new(Rc::clone(&unit_test1));
        let other = JobUnit::new(Rc::clone(&unit_test1));
        let before = UnitRelationAtom::UnitAtomBefore;
        let after = UnitRelationAtom::UnitAtomAfter;

        // nop
        let (o_nop, o_start, _, _, o_stop) = prepare_jobs(&unit_test1, mode);

        /* nop vs nop */
        uv.insert_suspend(Rc::clone(&uv_nop), true);
        uv.reshuffle();
        other.insert_suspend(Rc::clone(&o_nop), true);
        other.reshuffle();
        assert_eq!(uv.is_next_trigger_order_with(&other, before), true);
        assert_eq!(uv.is_next_trigger_order_with(&other, after), true);

        /* nop vs start */
        other.insert_suspend(Rc::clone(&o_start), true);
        other.reshuffle();
        assert_eq!(uv.is_next_trigger_order_with(&other, before), true);
        assert_eq!(uv.is_next_trigger_order_with(&other, after), true);

        /* nop vs stop */
        let other = JobUnit::new(Rc::clone(&unit_test1));
        other.insert_suspend(Rc::clone(&o_stop), true);
        other.reshuffle();
        assert_eq!(uv.is_next_trigger_order_with(&other, before), true);
        assert_eq!(uv.is_next_trigger_order_with(&other, after), true);

        // start
        let (o_nop, o_start, _, _, o_stop) = prepare_jobs(&unit_test1, mode);

        /* start vs nop */
        let uv = JobUnit::new(Rc::clone(&unit_test1));
        uv.insert_suspend(Rc::clone(&uv_start), true);
        uv.reshuffle();
        let other = JobUnit::new(Rc::clone(&unit_test1));
        other.insert_suspend(Rc::clone(&o_nop), true);
        other.reshuffle();
        assert_eq!(uv.is_next_trigger_order_with(&other, before), true);
        assert_eq!(uv.is_next_trigger_order_with(&other, after), true);

        /* start vs start */
        other.insert_suspend(Rc::clone(&o_start), true);
        other.reshuffle();
        assert_eq!(uv.is_next_trigger_order_with(&other, before), true);
        assert_eq!(uv.is_next_trigger_order_with(&other, after), false);

        /* start vs stop */
        let other = JobUnit::new(Rc::clone(&unit_test1));
        other.insert_suspend(Rc::clone(&o_stop), true);
        other.reshuffle();
        assert_eq!(uv.is_next_trigger_order_with(&other, before), false);
        assert_eq!(uv.is_next_trigger_order_with(&other, after), false);

        // stop
        let (o_nop, o_start, _, _, o_stop) = prepare_jobs(&unit_test1, mode);

        /* stop vs nop */
        let uv = JobUnit::new(Rc::clone(&unit_test1));
        uv.insert_suspend(Rc::clone(&uv_stop), true);
        uv.reshuffle();
        let other = JobUnit::new(Rc::clone(&unit_test1));
        other.insert_suspend(Rc::clone(&o_nop), true);
        other.reshuffle();
        assert_eq!(uv.is_next_trigger_order_with(&other, before), true);
        assert_eq!(uv.is_next_trigger_order_with(&other, after), true);

        /* stop vs start */
        other.insert_suspend(Rc::clone(&o_start), true);
        other.reshuffle();
        assert_eq!(uv.is_next_trigger_order_with(&other, before), true);
        assert_eq!(uv.is_next_trigger_order_with(&other, after), true);

        /* stop vs stop */
        let other = JobUnit::new(Rc::clone(&unit_test1));
        other.insert_suspend(Rc::clone(&o_stop), true);
        other.reshuffle();
        assert_eq!(uv.is_next_trigger_order_with(&other, before), false);
        assert_eq!(uv.is_next_trigger_order_with(&other, after), true);
    }

    #[test]
    fn juv_api_order_with_ignore() {
        let name_test1 = String::from("test1.service");
        let unit_test1 = create_unit(&name_test1);
        let mode = JobMode::JobIgnoreDependencies;
        let (uv_nop, uv_start, _, _, uv_stop) = prepare_jobs(&unit_test1, mode);
        let uv = JobUnit::new(Rc::clone(&unit_test1));
        let other = JobUnit::new(Rc::clone(&unit_test1));
        let before = UnitRelationAtom::UnitAtomBefore;
        let after = UnitRelationAtom::UnitAtomAfter;

        // nop
        let (o_nop, o_start, _, _, o_stop) = prepare_jobs(&unit_test1, mode);

        /* nop vs nop */
        uv.insert_suspend(Rc::clone(&uv_nop), true);
        uv.reshuffle();
        other.insert_suspend(Rc::clone(&o_nop), true);
        other.reshuffle();
        assert_eq!(uv.is_next_trigger_order_with(&other, before), true);
        assert_eq!(uv.is_next_trigger_order_with(&other, after), true);

        /* nop vs start */
        other.insert_suspend(Rc::clone(&o_start), true);
        other.reshuffle();
        assert_eq!(uv.is_next_trigger_order_with(&other, before), true);
        assert_eq!(uv.is_next_trigger_order_with(&other, after), true);

        /* nop vs stop */
        let other = JobUnit::new(Rc::clone(&unit_test1));
        other.insert_suspend(Rc::clone(&o_stop), true);
        other.reshuffle();
        assert_eq!(uv.is_next_trigger_order_with(&other, before), true);
        assert_eq!(uv.is_next_trigger_order_with(&other, after), true);

        // start
        let (o_nop, o_start, _, _, o_stop) = prepare_jobs(&unit_test1, mode);

        /* start vs nop */
        let uv = JobUnit::new(Rc::clone(&unit_test1));
        uv.insert_suspend(Rc::clone(&uv_start), true);
        uv.reshuffle();
        let other = JobUnit::new(Rc::clone(&unit_test1));
        other.insert_suspend(Rc::clone(&o_nop), true);
        other.reshuffle();
        assert_eq!(uv.is_next_trigger_order_with(&other, before), true);
        assert_eq!(uv.is_next_trigger_order_with(&other, after), true);

        /* start vs start */
        other.insert_suspend(Rc::clone(&o_start), true);
        other.reshuffle();
        assert_eq!(uv.is_next_trigger_order_with(&other, before), true);
        assert_eq!(uv.is_next_trigger_order_with(&other, after), true);

        /* start vs stop */
        let other = JobUnit::new(Rc::clone(&unit_test1));
        other.insert_suspend(Rc::clone(&o_stop), true);
        other.reshuffle();
        assert_eq!(uv.is_next_trigger_order_with(&other, before), true);
        assert_eq!(uv.is_next_trigger_order_with(&other, after), true);

        let (o_nop, o_start, _, _, o_stop) = prepare_jobs(&unit_test1, mode);

        /* stop vs nop */
        let uv = JobUnit::new(Rc::clone(&unit_test1));
        uv.insert_suspend(Rc::clone(&uv_stop), true);
        uv.reshuffle();
        let other = JobUnit::new(Rc::clone(&unit_test1));
        other.insert_suspend(Rc::clone(&o_nop), true);
        other.reshuffle();
        assert_eq!(uv.is_next_trigger_order_with(&other, before), true);
        assert_eq!(uv.is_next_trigger_order_with(&other, after), true);

        /* stop vs start */
        other.insert_suspend(Rc::clone(&o_start), true);
        other.reshuffle();
        assert_eq!(uv.is_next_trigger_order_with(&other, before), true);
        assert_eq!(uv.is_next_trigger_order_with(&other, after), true);

        /* stop vs stop */
        let other = JobUnit::new(Rc::clone(&unit_test1));
        other.insert_suspend(Rc::clone(&o_stop), true);
        other.reshuffle();
        assert_eq!(uv.is_next_trigger_order_with(&other, before), true);
        assert_eq!(uv.is_next_trigger_order_with(&other, after), true);
    }

    #[test]
    fn juv_calc_ready() {
        let name_test1 = String::from("test1.service");
        let unit_test1 = create_unit(&name_test1);
        let (job_nop, job_start, _, _, _) = prepare_jobs(&unit_test1, JobMode::JobReplace);
        let uv = JobUnit::new(Rc::clone(&unit_test1));

        // nothing
        assert_eq!(uv.data.borrow().calc_ready(), None);

        // suspend
        uv.insert_suspend(Rc::clone(&job_nop), true);
        let ret = uv.reshuffle();
        assert_eq!(uv.len(), 1);
        assert_eq!(ret.len(), 0);
        assert_eq!(uv.data.borrow().calc_ready(), Some(true));

        // trigger
        /* trigger */
        let (next_trigger, merge_trigger) = uv.data.borrow_mut().do_next_trigger();
        assert_eq!(next_trigger.get_id(), job_nop.get_id());
        assert!(merge_trigger.is_none());
        assert_eq!(uv.data.borrow().calc_ready(), None);
        /* retrigger */
        uv.data.borrow_mut().retrigger_trigger();
        assert_eq!(uv.data.borrow().calc_ready(), Some(false));
        /* pause+resume */
        uv.pause();
        assert_eq!(uv.data.borrow().calc_ready(), None);
        uv.resume();
        assert_eq!(uv.data.borrow().calc_ready(), Some(false));
        /* trigger-again */
        let (next_trigger, merge_trigger) = uv.data.borrow_mut().do_next_trigger();
        assert_eq!(next_trigger.get_id(), job_nop.get_id());
        assert!(merge_trigger.is_none());
        assert_eq!(uv.data.borrow().calc_ready(), None);

        // trigger + suspend
        /* trigger */
        uv.insert_suspend(Rc::clone(&job_start), true);
        let ret = uv.reshuffle();
        assert_eq!(uv.len(), 2);
        assert_eq!(ret.len(), 0);
        assert_eq!(uv.data.borrow().calc_ready(), None);
        /* retrigger */
        uv.data.borrow_mut().retrigger_trigger();
        assert_eq!(uv.data.borrow().calc_ready(), Some(false));
        /* trigger-again */
        let (next_trigger, merge_trigger) = uv.data.borrow_mut().do_next_trigger();
        assert_eq!(next_trigger.get_id(), job_nop.get_id());
        assert!(merge_trigger.is_none());
        assert_eq!(uv.data.borrow().calc_ready(), None);
    }

    fn prepare_jobs(
        unit: &Rc<UnitX>,
        mode: JobMode,
    ) -> (Rc<Job>, Rc<Job>, Rc<Job>, Rc<Job>, Rc<Job>) {
        let mut id: u32 = 0;

        id = id.wrapping_add(1); // ++
        let job_nop = Rc::new(Job::new(id, Rc::clone(unit), JobKind::JobNop));
        job_nop.init_attr(mode);

        id = id.wrapping_add(1); // ++
        let job_start = Rc::new(Job::new(id, Rc::clone(unit), JobKind::JobStart));
        job_start.init_attr(mode);

        id = id.wrapping_add(1); // ++
        let job_reload = Rc::new(Job::new(id, Rc::clone(unit), JobKind::JobReload));
        job_reload.init_attr(mode);

        id = id.wrapping_add(1); // ++
        let job_restart = Rc::new(Job::new(id, Rc::clone(unit), JobKind::JobRestart));
        job_restart.init_attr(mode);

        id = id.wrapping_add(1); // ++
        let job_stop = Rc::new(Job::new(id, Rc::clone(unit), JobKind::JobStop));
        job_stop.init_attr(mode);

        (job_nop, job_start, job_reload, job_restart, job_stop)
    }

    fn create_unit(name: &str) -> Rc<UnitX> {
        logger::init_log_with_console("test_unit_load", 4);
        log::info!("test");
        let dm = Rc::new(DataManager::new());
        let file = Rc::new(UnitFile::new());
        let unit_type = UnitType::UnitService;
        let plugins = Plugin::get_instance();
        let subclass = plugins.create_unit_obj(unit_type).unwrap();
        Rc::new(UnitX::new(
            &dm,
            &file,
            unit_type,
            name,
            subclass.into_unitobj(),
        ))
    }
}
