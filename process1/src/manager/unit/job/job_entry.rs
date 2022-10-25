#![warn(unused_imports)]
use super::job_rentry::{self, JobAttr, JobKind, JobRe};
use crate::manager::unit::data::{UnitActiveState, UnitNotifyFlags};
use crate::manager::unit::unit_base::{UnitActionError, UnitRelationAtom};
use crate::manager::unit::unit_entry::UnitX;
use crate::manager::unit::unit_rentry::JobMode;
use crate::reliability::Reliability;
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::manager) enum JobResult {
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
pub(in crate::manager) enum JobStage {
    Init,
    Wait,
    Running,
    End(JobResult),
}

#[derive(Clone)]
pub(in crate::manager::unit) struct JobConf {
    unit: Rc<UnitX>,
    kind: JobKind,
}

impl JobConf {
    pub(in crate::manager::unit) fn new(unit: Rc<UnitX>, kind: JobKind) -> JobConf {
        JobConf { unit, kind }
    }

    pub(super) fn map(input: &Self) -> JobConf {
        let k = job_merge_unit(input.kind, &input.unit);
        JobConf::new(Rc::clone(&input.unit), k)
    }

    pub(super) fn get_unit(&self) -> &Rc<UnitX> {
        &self.unit
    }

    pub(super) fn get_kind(&self) -> JobKind {
        self.kind
    }
}

#[derive(Clone)]
pub(in crate::manager) struct JobInfo {
    pub(in crate::manager) id: u32,
    pub(in crate::manager) unit: Rc<UnitX>,
    pub(in crate::manager) kind: JobKind,
    pub(in crate::manager) run_kind: JobKind,
    pub(in crate::manager) stage: JobStage,
}

impl fmt::Debug for JobInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Job")
            .field("id", &self.id)
            .field("unit", &self.unit.id())
            .field("kind", &self.kind)
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
            run_kind: job.run_kind(),
            stage: job.get_stage(),
        }
    }
}

pub(super) struct Job {
    // associated objects
    reli: Rc<Reliability>,
    rentry: Rc<JobRe>,

    // owned objects
    // key: input
    id: u32,

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
        id: u32,
        unit: Rc<UnitX>,
        kind: JobKind,
    ) -> Job {
        Job {
            reli: Rc::clone(relir),
            rentry: Rc::clone(rentryr),
            id,
            unit,
            kind,
            attr: RefCell::new(JobAttr::new(false, false, false)),
            run_kind: RefCell::new(job_rkind_new(kind)),
            stage: RefCell::new(JobStage::Init),
        }
    }

    pub(super) fn clear(&self) {
        // release external connection, like: timer, ...
        // do nothing now
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
        // do nothing now
    }

    pub(super) fn coldplug_suspend(&self) {
        // rebuild external connections, like: timer, ...
        // do nothing now
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

        // update reliability
        self.rentry_suspends_update();
    }

    pub(super) fn merge_attr(&self, other: &Self) {
        assert!(*self.stage.borrow() == JobStage::Wait);

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

        // update reliability
        self.reli.set_last_unit(self.unit.id());
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

    pub(super) fn get_id(&self) -> u32 {
        self.id
    }

    pub(super) fn unit(&self) -> &Rc<UnitX> {
        &self.unit
    }

    pub(super) fn get_kind(&self) -> JobKind {
        self.kind
    }

    pub(super) fn run_kind(&self) -> JobKind {
        *self.run_kind.borrow()
    }

    pub(super) fn get_stage(&self) -> JobStage {
        *self.stage.borrow()
    }

    pub(super) fn is_basic_op(&self) -> bool {
        job_rentry::job_is_basic_op(self.kind)
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

    fn rentry_trigger_insert(&self) {
        self.rentry
            .trigger_insert(self.unit.id(), self.kind, &self.attr.borrow());
    }

    fn rentry_trigger_remove(&self) {
        self.rentry.trigger_remove(self.unit.id());
    }

    fn rentry_trigger_get(&self) -> Option<(JobKind, JobAttr)> {
        self.rentry.trigger_get(self.unit.id())
    }

    fn rentry_suspends_insert(&self) {
        self.rentry
            .suspends_insert(self.unit.id(), self.kind, &self.attr.borrow());
    }

    fn rentry_suspends_remove(&self) {
        self.rentry.suspends_remove(self.unit.id(), self.kind);
    }

    fn rentry_suspends_get(&self) -> Option<JobAttr> {
        self.rentry.suspends_get(self.unit.id(), self.kind)
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
            UnitActiveState::UnitActive | UnitActiveState::UnitReloading => {
                Err(UnitActionError::UnitActionEAlready)
            }
            UnitActiveState::UnitActivating => Err(UnitActionError::UnitActionEAgain),
            _ => Err(UnitActionError::UnitActionEBadR),
        },
        JobKind::Nop => Err(UnitActionError::UnitActionEAlready), // do nothing
        _ => unreachable!("Invalid job run-kind: {:?}.", run_kind),
    };

    match ret {
        Ok(_) => Ok(()),
        Err(err) => Err(job_trigger_err_to_result(err)),
    }
}

fn job_trigger_err_to_result(err: UnitActionError) -> Option<JobResult> {
    match err {
        UnitActionError::UnitActionEAgain => None, // re-trigger again
        UnitActionError::UnitActionEAlready => Some(JobResult::Done), // over already
        UnitActionError::UnitActionEComm => Some(JobResult::Done), // convention
        UnitActionError::UnitActionEBadR => Some(JobResult::Skipped),
        UnitActionError::UnitActionENoExec => Some(JobResult::Invalid),
        UnitActionError::UnitActionEProto => Some(JobResult::Assert),
        UnitActionError::UnitActionEOpNotSupp => Some(JobResult::UnSupported),
        UnitActionError::UnitActionENolink => Some(JobResult::Dependency),
        UnitActionError::UnitActionEStale => Some(JobResult::Once),
        UnitActionError::UnitActionEFailed => Some(JobResult::Failed),
        UnitActionError::UnitActionEInval => Some(JobResult::Failed),
        UnitActionError::UnitActionEBusy => Some(JobResult::Failed),
        UnitActionError::UnitActionENoent => Some(JobResult::Failed),
    }
}

fn job_process_unit_start(ns: UnitActiveState) -> (Option<JobResult>, bool) {
    match ns {
        // something generated from the job has been done
        UnitActiveState::UnitActive => (Some(JobResult::Done), true),
        // something generated from the job is doing
        UnitActiveState::UnitActivating => (None, true),
        // something not generated from the job has been done
        UnitActiveState::UnitInActive => (Some(JobResult::Done), false),
        UnitActiveState::UnitFailed => (Some(JobResult::Failed), false),
        // something not generated from the job is doing
        UnitActiveState::UnitReloading
        | UnitActiveState::UnitDeActivating
        | UnitActiveState::UnitMaintenance => (None, false),
    }
}

fn job_process_unit_stop(ns: UnitActiveState) -> (Option<JobResult>, bool) {
    match ns {
        // something generated from the job has been done
        UnitActiveState::UnitInActive | UnitActiveState::UnitFailed => {
            (Some(JobResult::Done), true)
        }
        // something generated from the job is doing
        UnitActiveState::UnitDeActivating => (None, true),
        // something not generated from the job has been done
        UnitActiveState::UnitActive => (Some(JobResult::Failed), false),
        // something not generated from the job is doing
        UnitActiveState::UnitReloading
        | UnitActiveState::UnitActivating
        | UnitActiveState::UnitMaintenance => (Some(JobResult::Failed), false),
    }
}

fn job_process_unit_reload(
    ns: UnitActiveState,
    flags: UnitNotifyFlags,
) -> (Option<JobResult>, bool) {
    let mut result = JobResult::Done;
    if flags.intersects(UnitNotifyFlags::UNIT_NOTIFY_RELOAD_FAILURE) {
        result = JobResult::Failed;
    }
    match ns {
        // something generated from the job has been done
        UnitActiveState::UnitActive => (Some(result), true),
        // something generated from the job is doing
        UnitActiveState::UnitReloading => (None, true),
        // something not generated from the job has been done
        UnitActiveState::UnitInActive => (Some(JobResult::Done), false),
        UnitActiveState::UnitFailed => (Some(JobResult::Failed), false),
        // something not generated from the job is doing
        UnitActiveState::UnitActivating
        | UnitActiveState::UnitDeActivating
        | UnitActiveState::UnitMaintenance => (None, false),
    }
}

fn job_merge_unit(kind: JobKind, unit: &UnitX) -> JobKind {
    let us_is_active_or_reloading = matches!(
        unit.active_state(),
        UnitActiveState::UnitActive | UnitActiveState::UnitReloading
    );
    match (kind, us_is_active_or_reloading) {
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
