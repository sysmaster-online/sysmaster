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

use super::rentry::{self, JobAttr, JobKind, JobRe};
use crate::unit::DataManager;
use crate::unit::JobMode;
use crate::unit::UnitX;
use core::error::*;
use core::rel::Reliability;
use core::unit::{UnitActiveState, UnitNotifyFlags, UnitRelationAtom};
use event::{EventState, EventType, Events, Source};
use std::cell::RefCell;
use std::fmt;
use std::os::unix::prelude::RawFd;
use std::rc::{Rc, Weak};

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum JobResult {
    Done,
    Cancelled,
    TimeOut,
    Failed,
    Dependency,
    Skipped,
    Invalid,
    Assert,
    UnSupported,
    Collected,
    Once,
    Merged,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum JobStage {
    Init,
    Wait,
    Running,
    End(JobResult),
}

#[derive(Clone)]
pub(crate) struct JobConf {
    unit: Rc<UnitX>,
    kind: JobKind,
}

impl JobConf {
    pub(crate) fn new(unitr: &Rc<UnitX>, kind: JobKind) -> JobConf {
        JobConf {
            unit: Rc::clone(unitr),
            kind,
        }
    }

    pub(super) fn map(input: &Self) -> JobConf {
        let k = job_merge_unit(input.kind, &input.unit);
        JobConf::new(&input.unit, k)
    }

    pub(super) fn get_unit(&self) -> &Rc<UnitX> {
        &self.unit
    }

    pub(super) fn get_kind(&self) -> JobKind {
        self.kind
    }
}

pub(crate) struct JobTimer {
    time_usec: RefCell<u64>,
    job: RefCell<Weak<Job>>,
}

impl JobTimer {
    pub fn new(usec: u64) -> Self {
        JobTimer {
            time_usec: RefCell::new(usec),
            job: RefCell::new(Weak::new()),
        }
    }

    pub(super) fn attach_job(&self, job: &Rc<Job>) {
        *self.job.borrow_mut() = Rc::downgrade(job);
    }

    pub(super) fn set_time(&self, usec: u64) {
        *self.time_usec.borrow_mut() = usec
    }

    pub(super) fn get_time_usec(&self) -> u64 {
        *self.time_usec.borrow()
    }

    pub(self) fn job(&self) -> Option<Rc<Job>> {
        self.job.borrow().upgrade()
    }

    fn do_dispatch(&self) -> i32 {
        let job = match self.job() {
            None => {
                log::info!("The job has already been removed, skipping.");
                return 0;
            }
            Some(v) => v,
        };
        let unit_id = job.unit().unit().id();
        log::info!("Job {:?} of unit {} timeout", job.kind(), unit_id);
        job.dm.insert_job_result(unit_id, JobResult::TimeOut);
        0
    }
}

impl Source for JobTimer {
    fn fd(&self) -> RawFd {
        0
    }

    fn event_type(&self) -> EventType {
        EventType::TimerMonotonic
    }

    fn epoll_event(&self) -> u32 {
        (libc::EPOLLIN) as u32
    }

    fn time_relative(&self) -> u64 {
        *self.time_usec.borrow()
    }

    fn dispatch(&self, _: &Events) -> i32 {
        self.do_dispatch()
    }

    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }

    fn priority(&self) -> i8 {
        0i8
    }

    fn description(&self) -> String {
        String::from("JobTimer")
    }
}

#[derive(Clone)]
pub(crate) struct JobInfo {
    pub(crate) id: u128,
    pub(crate) unit: Rc<UnitX>,
    pub(crate) kind: JobKind,
    pub(crate) attr: JobAttr,
    pub(crate) run_kind: JobKind,
    pub(crate) stage: JobStage,
}

impl fmt::Debug for JobInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Job")
            .field("id", &self.id)
            .field("unit", &self.unit.id())
            .field("kind", &self.kind)
            .field("attr", &self.attr)
            .field("run_kind", &self.run_kind)
            .field("stage", &self.stage)
            .finish()
    }
}

impl JobInfo {
    pub(super) fn map(job: &Job) -> JobInfo {
        JobInfo {
            id: job.id,
            unit: Rc::clone(&job.unit),
            kind: job.kind,
            attr: job.attr(),
            run_kind: job.run_kind(),
            stage: job.get_stage(),
        }
    }
}

pub(super) struct Job {
    // associated objects
    reli: Rc<Reliability>,
    rentry: Rc<JobRe>,
    events: Rc<Events>,
    dm: Rc<DataManager>,

    // owned objects
    // key: input
    id: u128,
    timer: Rc<JobTimer>,

    // data
    /* config: input */
    unit: Rc<UnitX>,
    kind: JobKind,
    attr: RefCell<JobAttr>,

    /* status: self-generated */
    run_kind: RefCell<JobKind>,
    stage: RefCell<JobStage>,
}

impl PartialEq for Job {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Job {
    // nothing
}

impl fmt::Debug for Job {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Job")
            .field("id", &self.id)
            .field("unit", &self.unit.id())
            .field("kind", &self.kind)
            .field("attr", &self.attr)
            .field("run_kind", &self.run_kind)
            .field("stage", &self.stage)
            .finish()
    }
}

impl Job {
    pub(super) fn new(
        relir: &Rc<Reliability>,
        rentryr: &Rc<JobRe>,
        eventsr: &Rc<Events>,
        dmr: &Rc<DataManager>,
        id: u128,
        unit: Rc<UnitX>,
        kind: JobKind,
    ) -> Job {
        Job {
            reli: Rc::clone(relir),
            rentry: Rc::clone(rentryr),
            events: Rc::clone(eventsr),
            dm: Rc::clone(dmr),
            id,
            timer: Rc::new(JobTimer::new(0)),
            unit,
            kind,
            attr: RefCell::new(JobAttr::new(false, false, false, false)),
            run_kind: RefCell::new(job_rkind_new(kind)),
            stage: RefCell::new(JobStage::Init),
        }
    }

    pub(super) fn clear(&self) {
        // release external connection, like: timer, ...
        self.events.del_source(self.get_timer()).unwrap();
    }

    pub(super) fn get_timer(&self) -> Rc<JobTimer> {
        self.timer.clone()
    }

    pub(super) fn set_timer(&self) {
        let sec = self
            .unit()
            .get_config()
            .config_data()
            .borrow()
            .Unit
            .JobTimeoutSec;
        // No need to enable timer if JobTimeoutSec is set to 0
        if sec == 0 {
            return;
        }
        self.timer.set_time(sec * 1000000);
    }

    pub(super) fn rentry_map_trigger(&self) {
        let (kind, attr) = self.rentry_trigger_get().unwrap();
        assert_eq!(kind, self.kind);
        *self.stage.borrow_mut() = JobStage::Running;
        *self.attr.borrow_mut() = attr;
    }

    pub(super) fn rentry_map_suspend(&self) {
        let attr = self.rentry_suspends_get().unwrap();
        *self.stage.borrow_mut() = JobStage::Wait;
        *self.attr.borrow_mut() = attr;
    }

    pub(super) fn coldplug_trigger(&self) {
        // rebuild external connections, like: timer, ...
        self.events.del_source(self.get_timer()).unwrap();

        self.events.add_source(self.get_timer()).unwrap();
        self.events
            .set_enabled(self.get_timer(), EventState::OneShot)
            .unwrap();
    }

    pub(super) fn coldplug_suspend(&self) {
        // rebuild external connections, like: timer, ...
        self.events.del_source(self.get_timer()).unwrap();

        self.events.add_source(self.get_timer()).unwrap();
        self.events
            .set_enabled(self.get_timer(), EventState::OneShot)
            .unwrap();
    }

    pub(super) fn init_attr(&self, mode: JobMode) {
        assert!(*self.stage.borrow() == JobStage::Init);

        // update attr
        if mode == JobMode::IgnoreDependencies || self.kind == JobKind::Nop {
            self.attr.borrow_mut().ignore_order = true;
        }

        if mode == JobMode::ReplaceIrreversible {
            self.attr.borrow_mut().irreversible = true;
        }

        // update reliability: nothing to update in 'Init'
    }

    pub(super) fn merge_attr(&self, other: &Self) {
        assert!(*self.stage.borrow() == JobStage::Wait);
        assert!(self.kind == other.kind);

        // update attr
        self.attr.borrow_mut().or(&other.attr.borrow());

        // update reliability
        self.rentry_suspends_update();
    }

    pub(super) fn wait(&self) {
        assert!(*self.stage.borrow() == JobStage::Init);

        // update stage
        *self.stage.borrow_mut() = JobStage::Wait;

        // update reliability
        self.rentry_suspends_insert();
    }

    pub(super) fn run(&self) -> Result<(), Option<JobResult>> {
        let stage = *self.stage.borrow();
        assert!(stage == JobStage::Wait || stage == JobStage::Running);

        // update stage
        *self.stage.borrow_mut() = JobStage::Running;

        // Only enable the JobTimer event source if JobTimeoutSec is not 0
        if self.get_timer().get_time_usec() != 0 {
            if self.events.add_source(self.get_timer()).is_err() {
                log::error!("Failed to add JobTimer event source, skipping.");
            } else if self
                .events
                .set_enabled(self.get_timer(), EventState::OneShot)
                .is_err()
            {
                log::error!("Failed to enable JobTimer event source, skipping.");
            }
        }

        // update reliability
        self.reli.set_last_unit(&self.unit.id());
        if stage == JobStage::Wait {
            // wait -> running
            self.rentry_suspends_remove();
            self.rentry_trigger_insert();
        } else { // re-running
             // nothing needs update
        }

        // action
        let force = self.attr().force;
        job_trigger_unit(&self.unit, self.run_kind(), force)?;
        Ok(())
    }

    pub(super) fn finish(&self, result: JobResult) -> bool {
        let stage = *self.stage.borrow();
        assert!(stage == JobStage::Wait || stage == JobStage::Running);

        // try to get next run-kind
        let retry = match result {
            JobResult::Done => self.update_runkind(), // the run-kind could be updated in success only
            _ => false,
        };

        if retry {
            // re-running
            // update stage
            *self.stage.borrow_mut() = JobStage::Running;

            // update reliability: nothing needs update.
        } else {
            // wait | running -> end
            // update stage
            *self.stage.borrow_mut() = JobStage::End(result);

            // update reliability
            if stage == JobStage::Wait {
                // wait -> end
                self.rentry_suspends_remove();
            } else {
                // running -> end
                self.rentry_trigger_remove();
            }
        }

        retry
    }

    pub(super) fn attr(&self) -> JobAttr {
        self.attr.borrow().clone()
    }

    pub(super) fn get_id(&self) -> u128 {
        self.id
    }

    pub(super) fn unit(&self) -> &Rc<UnitX> {
        &self.unit
    }

    pub(super) fn kind(&self) -> JobKind {
        self.kind
    }

    pub(super) fn run_kind(&self) -> JobKind {
        *self.run_kind.borrow()
    }

    pub(super) fn get_stage(&self) -> JobStage {
        *self.stage.borrow()
    }

    pub(super) fn is_basic_op(&self) -> bool {
        rentry::job_is_basic_op(self.kind)
    }

    pub(super) fn is_order_with(&self, other: &Self, atom: UnitRelationAtom) -> i8 {
        assert!(
            atom == UnitRelationAtom::UnitAtomAfter || atom == UnitRelationAtom::UnitAtomBefore
        );
        if self.attr().ignore_order || other.attr().ignore_order {
            return 0;
        }

        job_order_compare(self.run_kind(), other.run_kind(), atom)
    }

    fn update_runkind(&self) -> bool {
        let last_rkind = self.run_kind();
        let rkind = job_rkind_map(self.kind, last_rkind);

        // update
        *self.run_kind.borrow_mut() = rkind;

        // is there anything else to do?
        last_rkind != rkind
    }

    pub(super) fn rentry_trigger_insert(&self) {
        self.rentry
            .trigger_insert(&self.unit.id(), self.kind, &self.attr.borrow());
    }

    fn rentry_trigger_remove(&self) {
        self.rentry.trigger_remove(&self.unit.id());
    }

    fn rentry_trigger_get(&self) -> Option<(JobKind, JobAttr)> {
        self.rentry.trigger_get(&self.unit.id())
    }

    pub(super) fn rentry_suspends_insert(&self) {
        self.rentry
            .suspends_insert(&self.unit.id(), self.kind, &self.attr.borrow());
    }

    fn rentry_suspends_remove(&self) {
        self.rentry.suspends_remove(&self.unit.id(), self.kind);
    }

    fn rentry_suspends_get(&self) -> Option<JobAttr> {
        self.rentry.suspends_get(&self.unit.id(), self.kind)
    }

    fn rentry_suspends_update(&self) {
        self.rentry_suspends_insert();
    }
}

pub(super) fn job_process_unit(
    run_kind: JobKind,
    ns: UnitActiveState,
    flags: UnitNotifyFlags,
) -> (Option<JobResult>, bool) {
    match run_kind {
        JobKind::Start => job_process_unit_start(ns),
        JobKind::Stop => job_process_unit_stop(ns),
        JobKind::Reload => job_process_unit_reload(ns, flags),
        _ => unreachable!("Invalid job run-kind."),
    }
}

pub(super) fn job_is_unit_applicable(kind: JobKind, unit: &UnitX) -> bool {
    match kind {
        JobKind::Start | JobKind::Verify | JobKind::Nop => true,
        JobKind::Stop => !unit.get_perpetual(),
        JobKind::Restart | JobKind::TryRestart => unit.can_start() && unit.can_stop(),
        JobKind::Reload | JobKind::TryReload => unit.can_reload(),
        JobKind::ReloadOrStart => unit.can_start() && unit.can_reload(),
    }
}

fn job_rkind_new(kind: JobKind) -> JobKind {
    match kind {
        JobKind::Restart => JobKind::Stop,
        _ => kind,
    }
}

fn job_rkind_map(kind: JobKind, last_rkind: JobKind) -> JobKind {
    match (kind, last_rkind) {
        (JobKind::Restart, JobKind::Stop) => JobKind::Start, // next: start
        (JobKind::Restart, JobKind::Start) => JobKind::Start, // next: nothing
        _ => kind,
    }
}

fn job_trigger_unit(unit: &UnitX, run_kind: JobKind, force: bool) -> Result<(), Option<JobResult>> {
    let ret = match run_kind {
        JobKind::Start => unit.start(),
        JobKind::Stop => unit.stop(force),
        JobKind::Reload => unit.reload(),
        JobKind::Verify => match unit.active_state() {
            UnitActiveState::Active | UnitActiveState::Reloading => Err(Error::UnitActionEAlready),
            UnitActiveState::Activating => Err(Error::UnitActionEAgain),
            _ => Err(Error::UnitActionEBadR),
        },
        JobKind::Nop => Err(Error::UnitActionEAlready), // do nothing
        _ => unreachable!("Invalid job run-kind: {:?}.", run_kind),
    };

    match ret {
        Ok(_) => Ok(()),
        Err(err) => Err(job_trigger_err_to_result(err)),
    }
}

fn job_trigger_err_to_result(err: Error) -> Option<JobResult> {
    match err {
        Error::UnitActionEAgain => None, // re-trigger again
        Error::UnitActionEAlready => Some(JobResult::Done), // over already
        Error::UnitActionEComm => Some(JobResult::Done), // convention
        Error::UnitActionEBadR => Some(JobResult::Skipped),
        Error::UnitActionENoExec => Some(JobResult::Invalid),
        Error::UnitActionEProto => Some(JobResult::Assert),
        Error::UnitActionEOpNotSupp => Some(JobResult::UnSupported),
        Error::UnitActionENolink => Some(JobResult::Dependency),
        Error::UnitActionEStale => Some(JobResult::Once),
        Error::UnitActionEFailed => Some(JobResult::Failed),
        Error::UnitActionEInval => Some(JobResult::Failed),
        Error::UnitActionEBusy => Some(JobResult::Failed),
        Error::UnitActionENoent => Some(JobResult::Failed),
        Error::UnitActionECanceled => Some(JobResult::Failed),
        _ => Some(JobResult::Skipped),
    }
}

fn job_process_unit_start(ns: UnitActiveState) -> (Option<JobResult>, bool) {
    match ns {
        // something generated from the job has been done
        UnitActiveState::Active => (Some(JobResult::Done), true),
        // something generated from the job is doing
        UnitActiveState::Activating => (None, true),
        // something not generated from the job has been done
        UnitActiveState::InActive => (Some(JobResult::Done), false),
        UnitActiveState::Failed => (Some(JobResult::Failed), false),
        // something not generated from the job is doing
        UnitActiveState::Reloading
        | UnitActiveState::DeActivating
        | UnitActiveState::Maintenance => (None, false),
    }
}

fn job_process_unit_stop(ns: UnitActiveState) -> (Option<JobResult>, bool) {
    match ns {
        // something generated from the job has been done
        UnitActiveState::InActive | UnitActiveState::Failed => (Some(JobResult::Done), true),
        // something generated from the job is doing
        UnitActiveState::DeActivating => (None, true),
        // something not generated from the job has been done
        UnitActiveState::Active => (Some(JobResult::Failed), false),
        // something not generated from the job is doing
        UnitActiveState::Reloading | UnitActiveState::Activating | UnitActiveState::Maintenance => {
            (Some(JobResult::Failed), false)
        }
    }
}

fn job_process_unit_reload(
    ns: UnitActiveState,
    flags: UnitNotifyFlags,
) -> (Option<JobResult>, bool) {
    let mut result = JobResult::Done;
    if flags.intersects(UnitNotifyFlags::RELOAD_FAILURE) {
        result = JobResult::Failed;
    }
    match ns {
        // something generated from the job has been done
        UnitActiveState::Active => (Some(result), true),
        // something generated from the job is doing
        UnitActiveState::Reloading => (None, true),
        // something not generated from the job has been done
        UnitActiveState::InActive => (Some(JobResult::Done), false),
        UnitActiveState::Failed => (Some(JobResult::Failed), false),
        // something not generated from the job is doing
        UnitActiveState::Activating
        | UnitActiveState::DeActivating
        | UnitActiveState::Maintenance => (None, false),
    }
}

fn job_merge_unit(kind: JobKind, unit: &UnitX) -> JobKind {
    match (kind, unit.active_state().is_active_or_reloading()) {
        (JobKind::TryReload, false) => JobKind::Nop,
        (JobKind::TryReload, true) => JobKind::Reload,
        (JobKind::TryRestart, false) => JobKind::Nop,
        (JobKind::TryRestart, true) => JobKind::Restart,
        (JobKind::ReloadOrStart, false) => JobKind::Start,
        (JobKind::ReloadOrStart, true) => JobKind::Reload,
        (kind, _) => kind,
    }
}

fn job_order_compare(rk_a: JobKind, rk_b: JobKind, atom: UnitRelationAtom) -> i8 {
    if rk_a == JobKind::Nop || rk_b == JobKind::Nop {
        return 0; // independent
    }

    if atom == UnitRelationAtom::UnitAtomAfter {
        return -job_order_compare(rk_b, rk_a, UnitRelationAtom::UnitAtomBefore);
    }

    match rk_b {
        JobKind::Stop => 1, // order: b -> a
        _ => -1,            // order: a -> b
    }
}
