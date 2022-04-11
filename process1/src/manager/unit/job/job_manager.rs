#![warn(unused_imports)]
use super::job_alloc::JobAlloc;
use super::job_entry::{self, Job, JobConf, JobInfo, JobKind, JobResult};
use super::job_notify::{self};
use super::job_stat::JobStat;
use super::job_table::JobTable;
use super::job_transaction::{self};
use super::JobErrno;
use crate::manager::data::{JobMode, UnitActiveState, UnitConfigItem, UnitNotifyFlags};
use crate::manager::unit::unit_datastore::UnitDb;
use crate::manager::unit::unit_entry::UnitX;
use crate::manager::unit::unit_relation_atom::UnitRelationAtom;
use std::rc::Rc;

pub struct JobAffect {
    // data
    pub adds: Vec<JobInfo>,
    pub dels: Vec<JobInfo>,
    pub updates: Vec<JobInfo>,

    // control
    interested: bool,
}

impl JobAffect {
    pub fn new(interested: bool) -> JobAffect {
        JobAffect {
            adds: Vec::new(),
            dels: Vec::new(),
            updates: Vec::new(),

            interested,
        }
    }

    fn record(&mut self, jobs: &(Vec<Rc<Job>>, Vec<Rc<Job>>, Vec<Rc<Job>>)) {
        if self.interested {
            let (adds, dels, updates) = jobs;
            self.adds.append(&mut jobs_2_jobinfo(adds));
            self.dels.append(&mut jobs_2_jobinfo(dels));
            self.updates.append(&mut jobs_2_jobinfo(updates));
        }
    }
}

//#[derive(Debug)]
pub struct JobManager {
    // associated objects
    db: Rc<UnitDb>,

    // control
    ja: JobAlloc,

    // data
    /* job */
    jobs: JobTable,  // (relative) stable
    stage: JobTable, // temporary

    // status
    running: bool,
    text: Option<(Rc<UnitX>, UnitActiveState, UnitActiveState, isize)>, // (unit, os, ns, flags) for synchronous finish

    // statistics
    stat: JobStat,
}

impl JobManager {
    pub fn exec(
        &mut self,
        config: &JobConf,
        mode: JobMode,
        affect: &mut JobAffect,
    ) -> Result<(), JobErrno> {
        job_trans_check_input(config, mode)?;

        self.stage.clear(); // clear stage first: make rollback simple

        // build changes in stage
        job_transaction::job_trans_expand(&mut self.stage, &mut self.ja, &self.db, config, mode)?;
        job_transaction::job_trans_affect(&mut self.stage, &mut self.ja, &self.db, config, mode)?;
        job_transaction::job_trans_verify(&mut self.stage, &self.jobs, mode)?;

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

        // prepare for async-running
        if self.jobs.is_ready() {
            // todo!(); enable-event
        }

        Ok(())
        // if it's successful, all jobs expanded would be inserted in 'self.jobs', otherwise(failed) they would be cleared next time.
    }

    pub fn notify(&mut self, config: &JobConf, mode: JobMode) -> Result<(), JobErrno> {
        if config.get_kind() != JobKind::JobReload {
            return Err(JobErrno::JobErrInput);
        }

        self.do_notify(config, Some(mode));
        Ok(())
    }

    pub(in crate::manager::unit) fn new(db: Rc<UnitDb>) -> JobManager {
        JobManager {
            db,

            ja: JobAlloc::new(),

            jobs: JobTable::new(),
            stage: JobTable::new(),

            running: false,
            text: None,

            stat: JobStat::new(),
        }
    }

    pub(in crate::manager::unit) fn run(&mut self) {
        loop {
            // try to trigger something to run
            self.text = None; // reset every time
            self.running = true;
            let trigger_ret = self.jobs.try_trigger(&self.db);
            self.running = false;

            if let Some((trigger_info, merge_trigger)) = trigger_ret {
                // something is triggered in this round
                // update statistics
                self.stat.update_change(&(&None, &merge_trigger, &None));
                self.stat
                    .update_stage_wait(trigger_info.is_some().into(), false); // trigger-someone[wait->run]: decrease 'wait'
                self.stat
                    .update_stage_run(trigger_info.is_some().into(), true); // trigger-someone[wait->run]: increase 'run'

                // try to finish it now in two case, and the case coming from unit has higher priority
                // case 1. the job has been finished synchronously in context;
                // case 2. the trigger is finished(failed or over);
                if let Some((unit, os, ns, flags)) = self.text.take() {
                    // case 1
                    self.do_try_finish(&unit, os, ns, flags);
                    self.text = None;
                }

                if let Some((t_jinfo, Some(tfinish_r))) = trigger_info {
                    // case 2
                    self.do_remove(&t_jinfo, tfinish_r, false);
                }
            } else {
                // nothing is triggered in this round
                break;
            }
        }

        // prepare for next round
        if self.jobs.is_ready() {
            // todo!(); enable-event
        }
    }

    pub(in crate::manager::unit) fn try_finish(
        &mut self,
        unit: &Rc<UnitX>,
        os: UnitActiveState,
        ns: UnitActiveState,
        flags: isize,
    ) -> Result<(), JobErrno> {
        // in order to simplify the mechanism, the running(trigger) and ending(finish) processes need to be isolated.
        if self.running {
            // (synchronous)finish in context
            if self.text.is_some() {
                // the unit has been finished already
                return Err(JobErrno::JobErrInput);
            }

            self.text = Some((Rc::clone(unit), os.clone(), ns.clone(), flags.clone()));
        // update and record it.
        } else {
            // (asynchronous)finish not in context
            self.do_try_finish(unit, os, ns, flags); // do it

            // prepare for async-running
            if self.jobs.is_ready() {
                // todo!(); enable-event
            }
        }

        Ok(())
    }

    pub(in crate::manager::unit) fn remove(&mut self, id: u32) -> Result<(), JobErrno> {
        assert!(!self.running);

        let job_info = self.jobs.get(id);
        if job_info.is_none() {
            return Err(JobErrno::JobErrNotExisted);
        }

        if self.jobs.is_trigger(id) {
            return Err(JobErrno::JobErrNotSupported);
        }

        if !self.jobs.is_suspend(id) {
            return Err(JobErrno::JobErrInternel);
        }

        self.do_remove(&job_info.unwrap(), JobResult::JobCancelled, true);
        Ok(())
    }

    pub(in crate::manager::unit) fn get_jobinfo(&self, id: u32) -> Option<JobInfo> {
        self.jobs.get(id)
    }

    fn do_try_finish(
        &mut self,
        unit: &Rc<UnitX>,
        os: UnitActiveState,
        ns: UnitActiveState,
        flags: isize,
    ) {
        let mut generated = false;
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
                        // finish it when 'finish' is suggested
                        self.del_trigger(&trigger, result);
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
    }

    fn do_remove(&mut self, job_info: &JobInfo, result: JobResult, force: bool) {
        // delete itself
        if self.jobs.is_trigger(job_info.id) {
            self.del_trigger(job_info, result);
        } else {
            self.del_suspends(job_info, result);
        }

        // simulate and notify unit events, which are not generated by the unit.
        self.simulate_unit_notify(&job_info.unit, result, force);
    }

    fn del_trigger(&mut self, job_info: &JobInfo, result: JobResult) {
        // delete itself
        let del_trigger = self.jobs.finish_trigger(&self.db, &job_info.unit, result);

        // remove relational jobs on failure
        let remove_jobs = match result {
            JobResult::JobDone => Vec::new(),
            _ => job_transaction::job_trans_fallback(
                &mut self.jobs,
                &self.db,
                &job_info.unit,
                job_info.run_kind,
            ),
        };

        // update statistics
        self.stat.update_change(&(&None, &del_trigger, &None));
        self.stat
            .update_changes(&(&Vec::new(), &remove_jobs, &Vec::new()));
        self.stat
            .update_stage_wait(del_trigger.is_none().into(), true); // finish-retrigger(the trigger has not been deleted)[run->wait]: increase 'wait'
        self.stat.update_stage_wait(remove_jobs.len(), false); // finish-remove[wait->end]: decrease 'wait'
        self.stat.update_stage_run(1, false); // finish-someone[run->wait|end]: decrease 'run'
    }

    fn del_suspends(&mut self, job_info: &JobInfo, result: JobResult) {
        let mut del_jobs = Vec::new();

        // delete itself
        del_jobs.append(&mut self.jobs.remove_suspends(
            &self.db,
            &job_info.unit,
            job_info.kind,
            None,
            result,
        ));

        // remove relational jobs on failure
        if result != JobResult::JobDone {
            del_jobs.append(&mut job_transaction::job_trans_fallback(
                &mut self.jobs,
                &self.db,
                &job_info.unit,
                job_info.run_kind,
            ));
        }

        // update statistics
        self.stat
            .update_changes(&(&Vec::new(), &del_jobs, &Vec::new()));
        self.stat.update_stage_wait(del_jobs.len(), false); // remove-del[wait->end]: decrease 'wait'
    }

    fn simulate_job_notify(&mut self, unit: &Rc<UnitX>, os: UnitActiveState, ns: UnitActiveState) {
        match (os, ns) {
            (
                UnitActiveState::UnitInActive | UnitActiveState::UnitFailed,
                UnitActiveState::UnitActive | UnitActiveState::UnitActivating,
            ) => self.do_notify(&JobConf::new(Rc::clone(unit), JobKind::JobStart), None),
            (
                UnitActiveState::UnitActive | UnitActiveState::UnitActivating,
                UnitActiveState::UnitInActive | UnitActiveState::UnitDeActivating,
            ) => self.do_notify(&JobConf::new(Rc::clone(unit), JobKind::JobStop), None),
            _ => {} // do nothing
        }
    }

    fn simulate_unit_notify(&mut self, unit: &Rc<UnitX>, result: JobResult, force: bool) {
        // OnFailure=
        if !force {
            // is forced removement a failure?
            if result != JobResult::JobDone {
                if let UnitConfigItem::UcItemOnFailJobMode(mode) =
                    unit.get_config(&UnitConfigItem::UcItemOnFailJobMode(JobMode::JobFail))
                {
                    self.exec_on(Rc::clone(unit), UnitRelationAtom::UnitAtomOnFailure, mode);
                }
            }
        }

        // trigger-notify
        for other in self
            .db
            .dep_gets_atom(unit, UnitRelationAtom::UnitAtomTriggeredBy)
        {
            other.trigger(unit);
        }
    }

    fn unit_start_on(
        &mut self,
        unit: &Rc<UnitX>,
        os: UnitActiveState,
        ns: UnitActiveState,
        flags: isize,
    ) {
        // OnFailure=
        if ns != os && flags & UnitNotifyFlags::UnitNotifyWillAutoRestart as isize == 0 {
            match ns {
                UnitActiveState::UnitFailed => {
                    if let UnitConfigItem::UcItemOnFailJobMode(mode) =
                        unit.get_config(&UnitConfigItem::UcItemOnFailJobMode(JobMode::JobFail))
                    {
                        self.exec_on(Rc::clone(unit), UnitRelationAtom::UnitAtomOnFailure, mode);
                    }
                }
                _ => {}
            };
        }

        // OnSuccess=
        if ns == UnitActiveState::UnitInActive
            && flags & UnitNotifyFlags::UnitNotifyWillAutoRestart as isize == 0
        {
            match os {
                UnitActiveState::UnitFailed
                | UnitActiveState::UnitInActive
                | UnitActiveState::UnitMaintenance => {}
                _ => {
                    if let UnitConfigItem::UcItemOnSucJobMode(mode) =
                        unit.get_config(&UnitConfigItem::UcItemOnSucJobMode(JobMode::JobFail))
                    {
                        self.exec_on(Rc::clone(unit), UnitRelationAtom::UnitAtomOnSuccess, mode);
                    }
                }
            };
        }
    }

    fn exec_on(&mut self, unit: Rc<UnitX>, atom: UnitRelationAtom, mode: JobMode) {
        let (configs, mode) = job_notify::job_notify_result(&self.db, unit, atom, mode);
        for config in configs.iter() {
            if let Err(_e) = self.exec(config, mode, &mut JobAffect::new(false)) {
                // debug
            }
        }
    }

    fn do_notify(&mut self, config: &JobConf, mode_option: Option<JobMode>) {
        let targets = job_notify::job_notify_event(&self.db, config, mode_option);
        for (config, mode) in targets.iter() {
            if let Err(_e) = self.exec(config, *mode, &mut JobAffect::new(false)) {
                // debug
            }
        }
    }
}

fn jobs_2_jobinfo(jobs: &Vec<Rc<Job>>) -> Vec<JobInfo> {
    jobs.iter().map(|jr| JobInfo::map(jr)).collect::<Vec<_>>()
}

fn job_trans_check_input(config: &JobConf, mode: JobMode) -> Result<(), JobErrno> {
    let kind = config.get_kind();
    let unit = config.get_unit();

    if kind == JobKind::JobNop {
        return Err(JobErrno::JobErrInput);
    }

    if mode == JobMode::JobIsolate {
        if kind != JobKind::JobStart {
            return Err(JobErrno::JobErrInput);
        }

        if let UnitConfigItem::UcItemAllowIsolate(false) =
            unit.get_config(&UnitConfigItem::UcItemAllowIsolate(false))
        {
            return Err(JobErrno::JobErrInput);
        }
    }

    if mode == JobMode::JobTrigger {
        if kind != JobKind::JobStop {
            return Err(JobErrno::JobErrInput);
        }
    }

    Ok(())
}
