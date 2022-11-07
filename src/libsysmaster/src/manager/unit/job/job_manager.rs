use super::job_alloc::JobAlloc;
use super::job_entry::{self, Job, JobConf, JobInfo, JobResult};
use super::job_notify::{self};
use super::job_rentry::{JobAttr, JobKind, JobRe};
use super::job_stat::JobStat;
use super::job_table::{self, JobTable};
use super::job_transaction::{self};
use super::job_unit_entry::{self};
use super::JobErrno;
use crate::manager::rentry::ReliLastFrame;
use crate::manager::table::{TableOp, TableSubscribe};
use crate::manager::unit::data::{UnitActiveState, UnitNotifyFlags};
use crate::manager::unit::unit_base::UnitRelationAtom;
use crate::manager::unit::unit_datastore::UnitDb;
use crate::manager::unit::unit_entry::UnitX;
use crate::manager::unit::unit_rentry::JobMode;
use crate::reliability::{ReStation, Reliability};
use libevent::{EventState, EventType, Events, Source};
use libutils::{Error, Result};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug)]
pub(in crate::manager::unit) struct JobAffect {
    // data
    pub(in crate::manager::unit) adds: Vec<JobInfo>,
    pub(in crate::manager::unit) dels: Vec<JobInfo>,
    pub(in crate::manager::unit) updates: Vec<JobInfo>,

    // control
    interested: bool,
}

impl JobAffect {
    pub(in crate::manager::unit) fn new(interested: bool) -> JobAffect {
        JobAffect {
            adds: Vec::new(),
            dels: Vec::new(),
            updates: Vec::new(),

            interested,
        }
    }

    #[allow(clippy::type_complexity)]
    fn record(&mut self, jobs: &(Vec<Rc<Job>>, Vec<Rc<Job>>, Vec<Rc<Job>>)) {
        if self.interested {
            let (adds, dels, updates) = jobs;
            self.adds.append(&mut jobs_2_jobinfo(adds));
            self.dels.append(&mut jobs_2_jobinfo(dels));
            self.updates.append(&mut jobs_2_jobinfo(updates));
        }
    }
}

pub(in crate::manager::unit) struct JobManager {
    // associated objects
    event: Rc<Events>,

    // owned objects
    // data
    sub_name: String, // key for table-subscriber: UnitSets
    data: Rc<JobManagerData>,
}

impl ReStation for JobManager {
    // input: do nothing

    // compensate
    fn db_compensate_last(&self, _lframe: (u32, Option<u32>, Option<u32>), lunit: Option<&String>) {
        if let Some(unit_id) = lunit {
            // merge to trigger
            self.rentry_trigger_merge(unit_id, true);
        }
    }

    fn db_compensate_history(&self) {
        // merge all triggers
        for unit_id in self.data.rentry_trigger_keys().iter() {
            self.rentry_trigger_merge(unit_id, false);
        }
    }

    fn do_compensate_last(&self, _lframe: (u32, Option<u32>, Option<u32>), lunit: Option<&String>) {
        if let Some(unit_id) = lunit {
            // re-run
            self.trigger_unit(unit_id);
        }
    }

    fn do_compensate_others(&self, lunit: Option<&String>) {
        // run all triggers
        for unit_id in self.data.rentry_trigger_keys().iter() {
            if Some(unit_id) != lunit {
                // other: all excluding the last
                self.trigger_unit(unit_id);
            }
        }
    }

    // data
    fn db_map(&self) {
        self.data.db_map();
    }

    // reload: special entry_coldplug
    // repeating protection
    fn entry_clear(&self) {
        self.data.entry_clear();
    }
}

impl Drop for JobManager {
    fn drop(&mut self) {
        log::debug!("JobManager drop, clear.");
        // repeating protection
        self.entry_clear();
        self.data.db.clear();
        self.data.reli.clear();
        self.event.clear();
    }
}

impl JobManager {
    pub(in crate::manager::unit) fn new(
        eventr: &Rc<Events>,
        relir: &Rc<Reliability>,
        dbr: &Rc<UnitDb>,
    ) -> JobManager {
        let jm = JobManager {
            event: Rc::clone(eventr),
            sub_name: String::from("JobManager"),
            data: Rc::new(JobManagerData::new(relir, dbr)),
        };
        jm.register(eventr, dbr);
        jm
    }

    pub(in crate::manager::unit) fn coldplug_unit(&self, unit: &UnitX) {
        self.data.coldplug_unit(unit);
    }

    pub(in crate::manager::unit) fn rentry_trigger_merge(&self, unit_id: &String, force: bool) {
        self.data.rentry_trigger_merge(unit_id, force);
    }

    pub(in crate::manager::unit) fn trigger_unit(&self, lunit: &str) {
        let unit = self.data.db.units_get(lunit).unwrap();
        let cnt = self.run(Some(&unit));
        assert_ne!(cnt, 0); // something must be triggered
    }

    pub(in crate::manager::unit) fn exec(
        &self,
        config: &JobConf,
        mode: JobMode,
        affect: &mut JobAffect,
    ) -> Result<(), JobErrno> {
        self.data.exec(config, mode, affect)?;
        self.try_enable();
        Ok(())
    }

    #[allow(dead_code)]
    pub(in crate::manager::unit) fn notify(
        &self,
        config: &JobConf,
        mode: JobMode,
    ) -> Result<(), JobErrno> {
        self.data.notify(config, mode)?;
        self.try_enable();
        Ok(())
    }

    pub(in crate::manager::unit) fn run(&self, unit: Option<&UnitX>) -> usize {
        let cnt = self.data.run(unit);
        self.try_enable();
        cnt
    }

    pub(in crate::manager::unit) fn try_finish(
        &self,
        unit: &Rc<UnitX>,
        os: UnitActiveState,
        ns: UnitActiveState,
        flags: UnitNotifyFlags,
    ) -> Result<(), JobErrno> {
        self.data.try_finish(unit, os, ns, flags)?;
        self.try_enable();
        Ok(())
    }

    #[allow(dead_code)]
    pub(in crate::manager::unit) fn remove(&self, id: u32) -> Result<(), JobErrno> {
        self.data.remove(id)?;
        self.try_enable();
        Ok(())
    }

    #[allow(dead_code)]
    pub(in crate::manager::unit) fn get_jobinfo(&self, id: u32) -> Option<JobInfo> {
        self.data.get_jobinfo(id)
    }

    pub(in crate::manager::unit) fn has_stop_job(&self, unit: &Rc<UnitX>) -> bool {
        self.data.get_suspends(unit).is_some()
    }

    fn try_enable(&self) {
        // prepare for async-running
        if self.data.calc_jobs_ready() && !self.data.up_ready() {
            // somethings new comes in, it should be enabled again.
            self.enable(&self.event);
        }

        // update up_ready
        self.data.update_up_ready();
    }

    fn register(&self, eventr: &Rc<Events>, dbr: &Rc<UnitDb>) {
        // event
        let source = Rc::clone(&self.data);
        eventr.add_source(source).unwrap();

        // db
        let subscriber = Rc::clone(&self.data);
        dbr.units_register(&self.sub_name, subscriber);
    }

    fn enable(&self, eventr: &Rc<Events>) {
        let source = Rc::clone(&self.data);
        eventr.set_enabled(source, EventState::OneShot).unwrap();
    }
}

impl Source for JobManagerData {
    fn event_type(&self) -> EventType {
        EventType::Defer
    }

    fn epoll_event(&self) -> u32 {
        0
    }

    fn priority(&self) -> i8 {
        -10
    }

    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }

    fn dispatch(&self, _event: &Events) -> Result<i32, Error> {
        log::debug!("job manager data dispatch");

        self.reli.set_last_frame1(ReliLastFrame::JobManager as u32);
        self.run(None);
        self.reli.clear_last_frame();

        self.update_up_ready();
        assert!(!self.calc_jobs_ready());

        Ok(0)
    }
}

impl TableSubscribe<String, Rc<UnitX>> for JobManagerData {
    fn notify(&self, op: &TableOp<String, Rc<UnitX>>) {
        match op {
            TableOp::TableInsert(_, _) => {} // do nothing
            TableOp::TableRemove(_, unit) => self.remove_unit(unit),
        }
    }
}

//#[derive(Debug)]
struct JobManagerData {
    // associated objects
    reli: Rc<Reliability>,
    db: Rc<UnitDb>,

    // owned objects
    // control
    rentry: Rc<JobRe>,
    ja: JobAlloc,

    // data
    /* job */
    jobs: JobTable,  // (relative) stable
    stage: JobTable, // temporary

    // status
    running: RefCell<bool>,
    #[allow(clippy::type_complexity)]
    text: RefCell<Option<(Rc<UnitX>, UnitActiveState, UnitActiveState, UnitNotifyFlags)>>, // (unit, os, ns, flags) for synchronous finish

    // statistics
    stat: JobStat,
}

// the declaration "pub(self)" is for identification only.
impl JobManagerData {
    pub(self) fn new(relir: &Rc<Reliability>, dbr: &Rc<UnitDb>) -> JobManagerData {
        let _rentry = Rc::new(JobRe::new(relir));
        JobManagerData {
            reli: Rc::clone(relir),
            db: Rc::clone(dbr),

            rentry: Rc::clone(&_rentry),
            ja: JobAlloc::new(relir, &_rentry),

            jobs: JobTable::new(dbr),
            stage: JobTable::new(dbr),

            running: RefCell::new(false),
            text: RefCell::new(None),

            stat: JobStat::new(),
        }
    }

    pub(self) fn entry_clear(&self) {
        self.jobs.clear();
        self.stage.clear();
        self.ja.clear();
        *self.running.borrow_mut() = false;
        *self.text.borrow_mut() = None;
        self.stat.clear();
    }

    pub(self) fn rentry_trigger_merge(&self, unit_id: &String, force: bool) {
        // get old
        let (k_d, a_d) = (JobKind::Restart, JobAttr::new(true, true, force)); // default
        let (k_o, a_o) = self
            .rentry
            .trigger_get(unit_id)
            .unwrap_or((k_d, a_d.clone()));

        // build new
        let k_n = job_unit_entry::job_merge_trigger(k_o);
        let mut a_n = a_d;
        a_n.or(&a_o);

        // insert rentry
        self.rentry.trigger_insert(unit_id, k_n, &a_n);
    }

    pub(self) fn db_map(&self) {
        // table(with job_entry)
        /* trigger */
        let mut triggers = Vec::new();
        for (unit_id, kind, _) in self.rentry.trigger_entries().iter() {
            let unit = self.db.units_get(unit_id).unwrap();
            let config = JobConf::new(unit, *kind);
            triggers.push(self.jobs.rentry_map_trigger(&self.ja, &config));
        }
        /* suspends */
        let mut suspends = Vec::new();
        for (unit_id, kind, _) in self.rentry.suspends_entries().iter() {
            let unit = self.db.units_get(unit_id).unwrap();
            let config = JobConf::new(unit, *kind);
            suspends.push(self.jobs.rentry_map_suspend(&self.ja, &config));
        }
        self.jobs.reshuffle();

        // stat
        self.stat
            .update_changes(&(&triggers, &Vec::new(), &Vec::new()));
        self.stat
            .update_changes(&(&suspends, &Vec::new(), &Vec::new()));
        self.stat.update_stage_run(triggers.len(), true); // trigger-add[init->run]: increase 'run'
        self.stat.update_stage_wait(suspends.len(), true); // suspend-add[init->wait]: increase 'wait'
        self.stat.clear_cnt(); // no history
    }

    pub(self) fn coldplug_unit(&self, unit: &UnitX) {
        // trigger
        self.jobs.coldplug_trigger(unit);

        // suspends
        self.jobs.coldplug_suspend(unit);
    }

    pub(self) fn exec(
        &self,
        config: &JobConf,
        mode: JobMode,
        affect: &mut JobAffect,
    ) -> Result<(), JobErrno> {
        job_trans_check_input(config, mode)?;

        self.stage.clear(); // clear stage first: make rollback simple

        // build changes in stage
        job_transaction::job_trans_expand(&self.stage, &self.ja, &self.db, config, mode)?;
        job_transaction::job_trans_affect(&self.stage, &self.ja, &self.db, config, mode)?;
        job_transaction::job_trans_verify(&self.stage, &self.jobs, mode)?;

        // commit stage to jobs
        let (add_jobs, del_jobs, update_jobs) = self.jobs.commit(&self.stage, mode)?;

        // clear stage
        self.stage.clear();

        // update statistics
        self.stat
            .update_changes(&(&add_jobs, &del_jobs, &update_jobs));
        self.stat.update_stage_wait(del_jobs.len(), false); // commit-del[wait->end]: decrease 'wait'
        self.stat.update_stage_wait(add_jobs.len(), true); // commit-add[init->wait]: increase 'wait'

        // output
        affect.record(&(add_jobs, del_jobs, update_jobs));

        Ok(())
        // if it's successful, all jobs expanded would be inserted in 'self.jobs', otherwise(failed) they would be cleared next time.
    }

    #[allow(dead_code)]
    pub(self) fn notify(&self, config: &JobConf, mode: JobMode) -> Result<(), JobErrno> {
        if config.get_kind() != JobKind::Reload {
            return Err(JobErrno::Input);
        }

        self.do_notify(config, Some(mode));
        Ok(())
    }

    pub(self) fn run(&self, unit: Option<&UnitX>) -> usize {
        let mut cnt: usize = 0;
        loop {
            // pop(JobTable.try_trigger()) + {record + action}(Job.run())
            // try to trigger something to run
            *self.text.borrow_mut() = None; // reset every time
            *self.running.borrow_mut() = true;
            let trigger_ret = self.jobs.try_trigger(unit);
            *self.running.borrow_mut() = false;

            if let Some((trigger_info, merge_trigger)) = trigger_ret {
                // something is triggered in this round
                (cnt, _) = cnt.overflowing_add(1); // ++

                // update statistics
                self.stat.update_change(&(&None, &merge_trigger, &None));
                if trigger_info.is_some() {
                    let t_jinfo = trigger_info.as_ref().cloned().unwrap().0;
                    let not_retrigger = t_jinfo.kind == t_jinfo.run_kind;
                    self.stat.update_stage_wait(not_retrigger.into(), false); // trigger-non-retrigger[wait->run]: decrease 'wait'
                    self.stat.update_stage_run(not_retrigger.into(), true); // trigger-non-retrigger[wait->run]: increase 'run'
                }

                // try to finish it now in two case, and the case coming from unit has higher priority
                // case 1. the job has been finished synchronously in context, which is derived from outside('unit') directly.
                // case 2. the trigger is ended(failed or over), which is derived from 'job' mechanism itself.
                if let Some((unit, os, ns, flags)) = self.text.take() {
                    // case 1: finish it
                    self.do_try_finish(&unit, os, ns, flags);
                    *self.text.borrow_mut() = None;
                }

                if let Some((t_jinfo, Some(tend_r))) = trigger_info {
                    // case 2: remove it
                    self.do_remove(&t_jinfo, tend_r, true);
                }
            } else {
                // nothing is triggered in this round
                break;
            }

            self.reli.clear_last_unit();
        }

        cnt
    }

    pub(self) fn try_finish(
        &self,
        unit: &Rc<UnitX>,
        os: UnitActiveState,
        ns: UnitActiveState,
        flags: UnitNotifyFlags,
    ) -> Result<(), JobErrno> {
        // in order to simplify the mechanism, the running(trigger) and ending(finish) processes need to be isolated.
        if *self.running.borrow() {
            // (synchronous)finish in context
            if self.text.borrow().is_some() {
                // the unit has been finished already
                return Err(JobErrno::Input);
            }

            *self.text.borrow_mut() = Some((Rc::clone(unit), os, ns, flags));
        // update and record it.
        } else {
            // (asynchronous)finish not in context
            self.do_try_finish(unit, os, ns, flags); // do it
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub(self) fn remove(&self, id: u32) -> Result<(), JobErrno> {
        assert!(!*self.running.borrow());

        let jinfo = self.jobs.get(id);
        if jinfo.is_none() {
            return Err(JobErrno::NotExisted);
        }
        let job_info = jinfo.unwrap();

        if self.jobs.is_trigger(id) {
            return Err(JobErrno::NotSupported);
        }

        if !self.jobs.is_suspend(id) {
            return Err(JobErrno::Internal);
        }

        // remove it from outside(command) directly
        self.do_remove(&job_info, JobResult::Cancelled, false);
        // mandatory removement is not considered a failure

        Ok(())
    }

    pub(self) fn update_up_ready(&self) {
        self.jobs.update_up_ready();
    }

    #[allow(dead_code)]
    pub(self) fn get_jobinfo(&self, id: u32) -> Option<JobInfo> {
        self.jobs.get(id)
    }

    pub(self) fn up_ready(&self) -> bool {
        self.jobs.up_ready()
    }

    pub(self) fn rentry_trigger_keys(&self) -> Vec<String> {
        self.rentry.trigger_keys()
    }

    pub(self) fn calc_jobs_ready(&self) -> bool {
        self.jobs.calc_ready()
    }

    pub(self) fn get_suspends(&self, unit: &Rc<UnitX>) -> Option<JobInfo> {
        self.jobs.get_suspend(unit, JobKind::Stop)
    }

    fn remove_unit(&self, unit: &UnitX) {
        // delete related jobs
        let (del_trigger, del_suspends) = self.jobs.remove_unit(unit);

        // update statistics
        self.stat.update_change(&(&None, &del_trigger, &None));
        self.stat
            .update_changes(&(&Vec::new(), &del_suspends, &Vec::new()));
        self.stat
            .update_stage_run(del_trigger.is_some().into(), false); // remove-unit[run->end]: decrease 'run'
        self.stat.update_stage_wait(del_suspends.len(), false); // remove-unit[wait->end]: decrease 'wait'
    }

    fn do_try_finish(
        &self,
        unit: &Rc<UnitX>,
        os: UnitActiveState,
        ns: UnitActiveState,
        flags: UnitNotifyFlags,
    ) {
        let mut generated = false;
        let mut del_one = false;
        if let Some((trigger, pause)) = self.jobs.get_trigger_info(unit) {
            generated = match pause {
                true => {
                    self.jobs.resume_unit(unit);
                    true
                }
                false => {
                    let (suggest_r, suggest_g) =
                        job_entry::job_process_unit(trigger.run_kind, ns, flags);
                    if let Some(result) = suggest_r {
                        // remove it when 'finish' is suggested from outside('unit') directly
                        del_one = self.do_remove(&trigger, result, false);
                    }
                    suggest_g
                }
            };
        }

        // simulate job events, which are not generated by the job.
        if !generated {
            self.simulate_job_notify(unit, os, ns);
        }

        // start on previous result
        self.unit_start_on(unit, os, ns, flags);

        // compensate trigger-notify when no job is deleted
        if !del_one {
            let atom = UnitRelationAtom::UnitAtomTriggeredBy;
            for other in self.db.dep_gets_atom(unit, atom) {
                other.trigger(unit);
            }
        }
    }

    fn do_remove(&self, job_info: &JobInfo, result: JobResult, inside: bool) -> bool {
        // delete itself unconditionly
        let del_one = self.do_remove_one(job_info, result, inside);
        if !del_one {
            return del_one; // false
        }

        // delete its relations in failure only
        if result != JobResult::Done {
            self.do_remove_relation(job_info);
        }

        del_one // true
    }

    fn do_remove_one(&self, job_info: &JobInfo, result: JobResult, inside: bool) -> bool {
        let unit = &job_info.unit;
        let kind = job_info.kind;

        // delete itself: trigger or suspend
        let mut del_trigger = None;
        let mut del_suspend = None;
        if self.jobs.is_trigger(job_info.id) {
            del_trigger = self.jobs.finish_trigger(unit, result);
        } else {
            let mut del_s = self.jobs.remove_suspends(unit, kind, None, result);
            assert_eq!(del_s.len(), 1); // only one input
            del_suspend = del_s.pop();
        }
        let del_one = del_trigger.is_some() || del_suspend.is_some();

        // simulate and notify unit events, which are not generated by the unit.
        if del_one {
            self.simulate_unit_notify(unit, result, inside);
        }

        // update statistics
        self.stat.update_change(&(&None, &del_trigger, &None));
        self.stat.update_change(&(&None, &del_suspend, &None));
        self.stat
            .update_stage_run(del_trigger.is_some().into(), false); // finish-non-retrigger(the trigger has not been deleted)[run->end]: decrease 'run'
        self.stat
            .update_stage_wait(del_suspend.is_some().into(), false); // finish-remove[wait->end]: decrease 'wait'

        del_one
    }

    fn do_remove_relation(&self, job_info: &JobInfo) {
        let unit = &job_info.unit;
        let run_kind = job_info.run_kind;

        // delete its relations: suspends
        let result_rel = JobResult::Dependency;
        let del_rel =
            job_transaction::job_trans_fallback(&self.jobs, &self.db, unit, run_kind, result_rel);

        // simulate and notify unit events, which are not generated by the unit.
        for u in job_table::jobs_2_units(&del_rel).iter() {
            if u != unit {
                // the removement is derived from 'job' mechanism itself
                self.simulate_unit_notify(u, result_rel, true);
            }
        }

        // update statistics
        self.stat
            .update_changes(&(&Vec::new(), &del_rel, &Vec::new()));
        self.stat.update_stage_wait(del_rel.len(), false); // finish-remove[wait->end]: decrease 'wait'
    }

    fn simulate_job_notify(&self, unit: &Rc<UnitX>, os: UnitActiveState, ns: UnitActiveState) {
        match (os, ns) {
            (
                UnitActiveState::UnitInActive | UnitActiveState::UnitFailed,
                UnitActiveState::UnitActive | UnitActiveState::UnitActivating,
            ) => self.do_notify(&JobConf::new(Rc::clone(unit), JobKind::Start), None),
            (
                UnitActiveState::UnitActive | UnitActiveState::UnitActivating,
                UnitActiveState::UnitInActive | UnitActiveState::UnitDeActivating,
            ) => self.do_notify(&JobConf::new(Rc::clone(unit), JobKind::Stop), None),
            _ => {} // do nothing
        }
    }

    fn simulate_unit_notify(&self, unit: &Rc<UnitX>, result: JobResult, inside: bool) {
        // OnFailure=
        if inside && result != JobResult::Done {
            if let JobMode::Fail = unit
                .get_config()
                .config_data()
                .borrow()
                .Unit
                .OnFailureJobMode
            {
                self.exec_on(
                    Rc::clone(unit),
                    UnitRelationAtom::UnitAtomOnFailure,
                    JobMode::Fail,
                );
            }
        }
    }

    fn unit_start_on(
        &self,
        unit: &Rc<UnitX>,
        os: UnitActiveState,
        ns: UnitActiveState,
        flags: UnitNotifyFlags,
    ) {
        // OnFailure=
        if ns != os
            && !flags.intersects(UnitNotifyFlags::UNIT_NOTIFY_WILL_AUTO_RESTART)
            && ns == UnitActiveState::UnitFailed
        {
            if let JobMode::Fail = unit
                .get_config()
                .config_data()
                .borrow()
                .Unit
                .OnFailureJobMode
            {
                self.exec_on(
                    Rc::clone(unit),
                    UnitRelationAtom::UnitAtomOnFailure,
                    JobMode::Fail,
                );
            }
        }

        // OnSuccess=
        if ns == UnitActiveState::UnitInActive
            && !flags.intersects(UnitNotifyFlags::UNIT_NOTIFY_WILL_AUTO_RESTART)
        {
            match os {
                UnitActiveState::UnitFailed
                | UnitActiveState::UnitInActive
                | UnitActiveState::UnitMaintenance => {}
                _ => {
                    if let JobMode::Fail = unit
                        .get_config()
                        .config_data()
                        .borrow()
                        .Unit
                        .OnFailureJobMode
                    {
                        self.exec_on(
                            Rc::clone(unit),
                            UnitRelationAtom::UnitAtomOnSuccess,
                            JobMode::Fail,
                        );
                    }
                }
            };
        }
    }

    fn exec_on(&self, unit: Rc<UnitX>, atom: UnitRelationAtom, mode: JobMode) {
        let (configs, mode) = job_notify::job_notify_result(&self.db, unit, atom, mode);
        for config in configs.iter() {
            if let Err(_e) = self.exec(config, mode, &mut JobAffect::new(false)) {
                // debug
            }
        }
    }

    fn do_notify(&self, config: &JobConf, mode_option: Option<JobMode>) {
        let targets = job_notify::job_notify_event(&self.db, config, mode_option);
        for (config, mode) in targets.iter() {
            if let Err(_e) = self.exec(config, *mode, &mut JobAffect::new(false)) {
                // debug
            }
        }
    }
}

fn jobs_2_jobinfo(jobs: &[Rc<Job>]) -> Vec<JobInfo> {
    jobs.iter().map(|jr| JobInfo::map(jr)).collect::<Vec<_>>()
}

fn job_trans_check_input(config: &JobConf, mode: JobMode) -> Result<(), JobErrno> {
    let kind = config.get_kind();
    let unit = config.get_unit();

    if mode == JobMode::Isolate {
        if kind != JobKind::Start {
            return Err(JobErrno::Input);
        }

        if let false = unit.get_config().config_data().borrow().Unit.AllowIsolate {
            return Err(JobErrno::Input);
        }
    }

    if mode == JobMode::Trigger && kind != JobKind::Stop {
        return Err(JobErrno::Input);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manager::rentry::RELI_HISTORY_MAX_DBS;
    use crate::manager::unit::data::DataManager;
    use crate::manager::unit::job::JobStage;
    use crate::manager::unit::uload_util::UnitFile;
    use crate::manager::unit::unit_rentry::{UnitRe, UnitRelations, UnitType};
    use crate::plugin::Plugin;
    use libutils::logger;
    use libutils::path_lookup::LookupPaths;

    //#[test]
    #[allow(dead_code)]
    fn job_reli() {
        logger::init_log_with_console("test_unit_load", 4);
        let reli = Rc::new(Reliability::new(RELI_HISTORY_MAX_DBS));
        let event = Rc::new(Events::new().unwrap());
        let rentry = Rc::new(UnitRe::new(&reli));
        let db = Rc::new(UnitDb::new(&rentry));
        let jm = JobManager::new(&event, &reli, &db);

        log::debug!("job_reli, reli:{}.", Rc::strong_count(&reli)); // 3
        log::debug!("job_reli, event:{}.", Rc::strong_count(&event)); // 2
        log::debug!("job_reli, rentry:{}.", Rc::strong_count(&rentry)); // 3
        log::debug!("job_reli, db:{}.", Rc::strong_count(&db)); // 4

        drop(jm);

        log::debug!("job_reli, reli:{}.", Rc::strong_count(&reli)); // 1
        log::debug!("job_reli, event:{}.", Rc::strong_count(&event)); // 1
        log::debug!("job_reli, rentry:{}.", Rc::strong_count(&rentry)); // 3
        log::debug!("job_reli, db:{}.", Rc::strong_count(&db)); // 1

        drop(event);

        log::debug!("job_reli, reli:{}.", Rc::strong_count(&reli)); // 1
        log::debug!("job_reli, rentry:{}.", Rc::strong_count(&rentry)); // 3
        log::debug!("job_reli, db:{}.", Rc::strong_count(&db)); // 1

        drop(db);
        log::debug!("job_reli, rentry:{}.", Rc::strong_count(&rentry)); // 1
    }

    #[test]
    fn job_exec_input_check() {
        let (event, reli, db, unit_test1, _unit_test2) = prepare_unit_multi(None);
        let jm = JobManager::new(&event, &reli, &db);
        let mut affect = JobAffect::new(true);

        let conf = JobConf::new(Rc::clone(&unit_test1), JobKind::Stop);
        let ret = jm.exec(&conf, JobMode::Isolate, &mut affect);
        assert!(ret.is_err());

        let conf = JobConf::new(Rc::clone(&unit_test1), JobKind::Start);
        let ret = jm.exec(&conf, JobMode::Trigger, &mut affect);
        assert!(ret.is_err());
    }

    #[test]
    fn job_exec_single() {
        let (event, reli, db, unit_test1, _unit_test2) = prepare_unit_multi(None);
        let jm = JobManager::new(&event, &reli, &db);

        let mut affect = JobAffect::new(true);
        let conf = JobConf::new(Rc::clone(&unit_test1), JobKind::Start);
        let ret = jm.exec(&conf, JobMode::Replace, &mut affect);
        assert!(ret.is_ok());
        assert_eq!(jm.data.jobs.len(), 1);
        assert_eq!(jm.data.jobs.ready_len(), 1);

        assert_eq!(affect.adds.len(), 1);
        let job_info = affect.adds.pop().unwrap();
        assert!(Rc::ptr_eq(&job_info.unit, &unit_test1));
        assert_eq!(job_info.kind, JobKind::Start);
        assert_eq!(job_info.run_kind, JobKind::Start);
        assert_eq!(job_info.stage, JobStage::Wait);
    }

    #[test]
    fn job_exec_multi() {
        let relation = Some(UnitRelations::UnitRequires);
        let (event, reli, db, unit_test1, unit_test2) = prepare_unit_multi(relation);
        let jm = JobManager::new(&event, &reli, &db);

        let mut affect = JobAffect::new(true);
        let conf = JobConf::new(Rc::clone(&unit_test1), JobKind::Start);
        let ret = jm.exec(&conf, JobMode::Replace, &mut affect);
        assert!(ret.is_ok());
        assert_eq!(jm.data.jobs.len(), 2);
        assert_eq!(jm.data.jobs.ready_len(), 2);
        assert_eq!(affect.adds.len(), 2);
        let job_info1 = affect.adds.pop().unwrap();
        assert!(
            Rc::ptr_eq(&job_info1.unit, &unit_test1) || Rc::ptr_eq(&job_info1.unit, &unit_test2)
        );
        assert_eq!(job_info1.kind, JobKind::Start);
        assert_eq!(job_info1.run_kind, JobKind::Start);
        assert_eq!(job_info1.stage, JobStage::Wait);
        let job_info2 = affect.adds.pop().unwrap();
        assert!(!Rc::ptr_eq(&job_info1.unit, &job_info2.unit));
        assert!(
            Rc::ptr_eq(&job_info2.unit, &unit_test1) || Rc::ptr_eq(&job_info2.unit, &unit_test2)
        );
        assert_eq!(job_info2.kind, JobKind::Start);
        assert_eq!(job_info2.run_kind, JobKind::Start);
        assert_eq!(job_info2.stage, JobStage::Wait);
    }

    #[test]
    fn job_notify() {
        let (event, reli, db, unit_test1, _unit_test2) = prepare_unit_multi(None);
        let jm = JobManager::new(&event, &reli, &db);

        let conf = JobConf::new(Rc::clone(&unit_test1), JobKind::Start);
        let ret = jm.notify(&conf, JobMode::Replace);
        assert!(ret.is_err());

        let conf = JobConf::new(Rc::clone(&unit_test1), JobKind::Reload);
        let ret = jm.notify(&conf, JobMode::Replace);
        assert!(ret.is_ok());
    }

    #[test]
    fn job_try_finish_async() {
        let (event, reli, db, unit_test1, _unit_test2) = prepare_unit_multi(None);
        let jm = JobManager::new(&event, &reli, &db);
        let os = UnitActiveState::UnitInActive;
        let ns = UnitActiveState::UnitActive;
        let flags = UnitNotifyFlags::empty();

        let ret = jm.try_finish(&unit_test1, os, ns, flags);
        assert!(ret.is_ok());
    }

    #[test]
    fn job_try_finish_sync() {
        let (event, reli, db, unit_test1, _unit_test2) = prepare_unit_multi(None);
        let jm = JobManager::new(&event, &reli, &db);
        let os = UnitActiveState::UnitInActive;
        let ns = UnitActiveState::UnitActive;
        let flags = UnitNotifyFlags::empty();

        *jm.data.text.borrow_mut() = None; // reset every time
        *jm.data.running.borrow_mut() = true;
        let ret = jm.try_finish(&unit_test1, os, ns, flags);
        *jm.data.running.borrow_mut() = false;
        assert!(ret.is_ok());
        assert!(jm.data.text.borrow().is_some());
        let (u, o, n, f) = jm.data.text.take().unwrap();
        assert_eq!(u.id(), unit_test1.id());
        assert_eq!(o, os);
        assert_eq!(n, ns);
        assert_eq!(f, flags);
    }

    #[test]
    fn job_run_finish_single() {
        let (event, reli, db, unit_test1, _unit_test2) = prepare_unit_multi(None);
        let jm = JobManager::new(&event, &reli, &db);

        let conf = JobConf::new(Rc::clone(&unit_test1), JobKind::Nop);
        jm.exec(&conf, JobMode::Replace, &mut JobAffect::new(false))
            .unwrap();
        assert_eq!(jm.data.jobs.len(), 1);
        assert_eq!(jm.data.jobs.ready_len(), 1);

        jm.data.run(None);
        assert_eq!(jm.data.jobs.len(), 0);
        assert_eq!(jm.data.jobs.ready_len(), 0);
    }

    #[test]
    fn job_run_finish_multi() {
        let (event, reli, db, unit_test1, unit_test2) = prepare_unit_multi(None);
        let jm = JobManager::new(&event, &reli, &db);

        let conf1 = JobConf::new(Rc::clone(&unit_test1), JobKind::Nop);
        jm.exec(&conf1, JobMode::Replace, &mut JobAffect::new(false))
            .unwrap();
        let conf2 = JobConf::new(Rc::clone(&unit_test2), JobKind::Nop);
        jm.exec(&conf2, JobMode::Replace, &mut JobAffect::new(false))
            .unwrap();
        assert_eq!(jm.data.jobs.len(), 2);
        assert_eq!(jm.data.jobs.ready_len(), 2);

        jm.data.run(None);
        assert_eq!(jm.data.jobs.len(), 0);
        assert_eq!(jm.data.jobs.ready_len(), 0);
    }

    #[test]
    fn job_run_unit_finish_single() {
        let (event, reli, db, unit_test1, unit_test2) = prepare_unit_multi(None);
        let jm = JobManager::new(&event, &reli, &db);

        let conf = JobConf::new(Rc::clone(&unit_test1), JobKind::Nop);
        jm.exec(&conf, JobMode::Replace, &mut JobAffect::new(false))
            .unwrap();
        assert_eq!(jm.data.jobs.len(), 1);
        assert_eq!(jm.data.jobs.ready_len(), 1);

        jm.data.run(Some(&unit_test2));
        assert_eq!(jm.data.jobs.len(), 1);
        assert_eq!(jm.data.jobs.ready_len(), 1);

        jm.data.run(Some(&unit_test1));
        assert_eq!(jm.data.jobs.len(), 0);
        assert_eq!(jm.data.jobs.ready_len(), 0);
    }

    #[test]
    fn job_run_unit_finish_multi() {
        let (event, reli, db, unit_test1, unit_test2) = prepare_unit_multi(None);
        let jm = JobManager::new(&event, &reli, &db);

        let conf1 = JobConf::new(Rc::clone(&unit_test1), JobKind::Nop);
        jm.exec(&conf1, JobMode::Replace, &mut JobAffect::new(false))
            .unwrap();
        let conf2 = JobConf::new(Rc::clone(&unit_test2), JobKind::Nop);
        jm.exec(&conf2, JobMode::Replace, &mut JobAffect::new(false))
            .unwrap();
        assert_eq!(jm.data.jobs.len(), 2);
        assert_eq!(jm.data.jobs.ready_len(), 2);

        jm.data.run(Some(&unit_test2));
        assert_eq!(jm.data.jobs.len(), 1);
        assert_eq!(jm.data.jobs.ready_len(), 1);

        jm.data.run(Some(&unit_test1));
        assert_eq!(jm.data.jobs.len(), 0);
        assert_eq!(jm.data.jobs.ready_len(), 0);
    }

    #[test]
    fn job_remove() {
        let (event, reli, db, unit_test1, _unit_test2) = prepare_unit_multi(None);
        let jm = JobManager::new(&event, &reli, &db);
        let mut affect = JobAffect::new(true);

        // nothing exists
        let ret = jm.remove(1);
        assert!(ret.is_err());

        // something exists
        let conf = JobConf::new(Rc::clone(&unit_test1), JobKind::Start);
        let ret = jm.exec(&conf, JobMode::Replace, &mut affect);
        assert!(ret.is_ok());
        assert_eq!(jm.data.jobs.len(), 1);
        assert_eq!(affect.adds.len(), 1);
        let job_info = affect.adds.pop().unwrap();
        let ret = jm.remove(job_info.id);
        assert!(ret.is_ok());
    }

    #[test]
    fn job_get_jobinfo() {
        let (event, reli, db, unit_test1, _unit_test2) = prepare_unit_multi(None);
        let jm = JobManager::new(&event, &reli, &db);
        let mut affect = JobAffect::new(true);

        // nothing exists
        let ret = jm.get_jobinfo(1);
        assert!(ret.is_none());

        // something exists
        let conf = JobConf::new(Rc::clone(&unit_test1), JobKind::Start);
        let ret = jm.exec(&conf, JobMode::Replace, &mut affect);
        assert!(ret.is_ok());
        assert_eq!(jm.data.jobs.len(), 1);
        assert_eq!(affect.adds.len(), 1);
        let job_info = affect.adds.pop().unwrap();
        let ret = jm.get_jobinfo(job_info.id);
        assert!(ret.is_some());
        let lkup_info = ret.unwrap();
        assert_eq!(lkup_info.id, job_info.id);
        assert_eq!(lkup_info.unit.id(), job_info.unit.id());
        assert_eq!(lkup_info.kind, job_info.kind);
        assert_eq!(lkup_info.run_kind, job_info.run_kind);
        assert_eq!(lkup_info.stage, job_info.stage);
    }

    #[test]
    fn job_has_stop_job() {
        let (event, reli, db, unit_test1, unit_test2) = prepare_unit_multi(None);
        let jm = JobManager::new(&event, &reli, &db);
        let mut affect = JobAffect::new(true);

        // nothing exists
        let ret = jm.has_stop_job(&unit_test1);
        assert!(!ret);
        let ret = jm.has_stop_job(&unit_test2);
        assert!(!ret);

        // something(non-stop) exists
        let conf = JobConf::new(Rc::clone(&unit_test1), JobKind::Start);
        let ret = jm.exec(&conf, JobMode::Replace, &mut affect);
        assert!(ret.is_ok());
        assert_eq!(jm.data.jobs.len(), 1);
        let ret = jm.has_stop_job(&unit_test1);
        assert!(!ret);

        // something(stop) exists
        let conf = JobConf::new(Rc::clone(&unit_test1), JobKind::Stop);
        let ret = jm.exec(&conf, JobMode::Replace, &mut affect);
        assert!(ret.is_ok());
        assert_eq!(jm.data.jobs.len(), 1);
        let ret = jm.has_stop_job(&unit_test1);
        assert!(ret);
    }

    #[test]
    fn job_remove_unit() {
        let (event, reli, db, unit_test1, unit_test2) = prepare_unit_multi(None);
        let jm = JobManager::new(&event, &reli, &db);
        let mut affect = JobAffect::new(true);

        let conf = JobConf::new(Rc::clone(&unit_test1), JobKind::Start);
        let ret = jm.exec(&conf, JobMode::Replace, &mut affect);
        assert!(ret.is_ok());
        assert_eq!(jm.data.jobs.len(), 1);
        let conf = JobConf::new(Rc::clone(&unit_test2), JobKind::Start);
        let ret = jm.exec(&conf, JobMode::Replace, &mut affect);
        assert!(ret.is_ok());
        assert_eq!(jm.data.jobs.len(), 2);

        jm.data.remove_unit(&unit_test2);
        assert_eq!(jm.data.jobs.len(), 1);
        jm.data.remove_unit(&unit_test1);
        assert_eq!(jm.data.jobs.len(), 0);
    }

    #[allow(clippy::type_complexity)]
    fn prepare_unit_multi(
        relation: Option<UnitRelations>,
    ) -> (
        Rc<Events>,
        Rc<Reliability>,
        Rc<UnitDb>,
        Rc<UnitX>,
        Rc<UnitX>,
    ) {
        let event = Rc::new(Events::new().unwrap());
        let dm = Rc::new(DataManager::new());
        let reli = Rc::new(Reliability::new(RELI_HISTORY_MAX_DBS));
        let rentry = Rc::new(UnitRe::new(&reli));
        let db = Rc::new(UnitDb::new(&rentry));
        let name_test1 = String::from("test1.service");
        let unit_test1 = create_unit(&dm, &reli, &rentry, &name_test1);
        let name_test2 = String::from("test2.service");
        let unit_test2 = create_unit(&dm, &reli, &rentry, &name_test2);
        db.units_insert(name_test1, Rc::clone(&unit_test1));
        db.units_insert(name_test2, Rc::clone(&unit_test2));
        if let Some(r) = relation {
            let u1 = Rc::clone(&unit_test1);
            let u2 = Rc::clone(&unit_test2);
            db.dep_insert(u1, r, u2, true, 0).unwrap();
        }
        (event, reli, db, unit_test1, unit_test2)
    }

    fn create_unit(
        dmr: &Rc<DataManager>,
        relir: &Rc<Reliability>,
        rentryr: &Rc<UnitRe>,
        name: &str,
    ) -> Rc<UnitX> {
        logger::init_log_with_console("test_unit_load", 4);
        log::info!("test");

        let mut l_path = LookupPaths::new();
        l_path.init_lookup_paths();
        let lookup_path = Rc::new(l_path);
        let file = Rc::new(UnitFile::new(&lookup_path));

        let unit_type = UnitType::UnitService;
        let plugins = Plugin::get_instance();
        let subclass = plugins.create_unit_obj(unit_type).unwrap();
        subclass.attach_reli(Rc::clone(relir));
        Rc::new(UnitX::new(
            dmr,
            rentryr,
            &file,
            unit_type,
            name,
            subclass,
        ))
    }
}
