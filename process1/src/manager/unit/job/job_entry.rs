#![warn(unused_imports)]
use crate::manager::data::{UnitActiveState, UnitNotifyFlags};
use crate::manager::unit::unit_base::UnitActionError;
use crate::manager::unit::unit_base::{JobMode, UnitRelationAtom};
use crate::manager::unit::unit_entry::UnitX;
use std::cell::RefCell;
use std::fmt;
use std::hash::Hash;
use std::rc::Rc;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(in crate::manager) enum JobKind {
    // 'type' is better, but it's keyword in rust
    // basic kind
    /* mut: the stage of unit can be changed */
    JobStart,
    JobStop,
    JobReload,
    JobRestart,

    /* non-mut: the stage of unit can not be changed */
    JobVerify,
    JobNop,

    // compound kind
    JobTryReload,
    JobTryRestart,
    JobReloadOrStart,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::manager) enum JobResult {
    JobDone,
    JobCancelled,
    JobTimeOut,
    JobFailed,
    JobDependency,
    JobSkipped,
    JobInvalid,
    JobAssert,
    JobUnSupported,
    JobCollected,
    JobOnce,
    JobMerged,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub(in crate::manager) enum JobStage {
    JobInit,
    JobWait,
    JobRunning,
    JobEnd(JobResult),
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
            .field("unit", &self.unit.get_id())
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
            run_kind: job.get_runkind(),
            stage: job.get_stage(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) enum JobAttrKind {
    JobIgnoreOrder,
    JobIrreversible,
}

pub(super) struct Job {
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

impl Drop for Job {
    fn drop(&mut self) {
        //todo!();
    }
}

impl fmt::Debug for Job {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Job")
            .field("id", &self.id)
            .field("unit", &self.unit.get_id())
            .field("kind", &self.kind)
            .field("attr", &self.attr)
            .field("run_kind", &self.run_kind)
            .field("stage", &self.stage)
            .finish()
    }
}

impl Job {
    pub(super) fn new(id: u32, unit: Rc<UnitX>, kind: JobKind) -> Job {
        Job {
            id,
            unit,
            kind,
            attr: RefCell::new(JobAttr::new()),
            run_kind: RefCell::new(job_rkind_new(kind)),
            stage: RefCell::new(JobStage::JobInit),
        }
    }

    pub(super) fn init_attr(&self, mode: JobMode) {
        if mode == JobMode::JobReplaceIrreversible {
            self.attr
                .borrow_mut()
                .set(JobAttrKind::JobIrreversible, true);
        }

        if mode == JobMode::JobIgnoreDependencies || self.kind == JobKind::JobNop {
            self.attr
                .borrow_mut()
                .set(JobAttrKind::JobIgnoreOrder, true);
        }
    }

    pub(super) fn merge_attr(&self, other: &Self) {
        self.attr.borrow_mut().merge(&other.attr.borrow());
    }

    pub(super) fn wait(&self) {
        assert!(*self.stage.borrow() == JobStage::JobInit);
        *self.stage.borrow_mut() = JobStage::JobWait;
    }

    pub(super) fn run(&self) -> Result<(), Option<JobResult>> {
        assert!(*self.stage.borrow() == JobStage::JobWait);
        *self.stage.borrow_mut() = JobStage::JobRunning;
        job_trigger_unit(self.get_runkind(), &self.unit)?;
        Ok(())
    }

    pub(super) fn finish(&self, result: JobResult) -> bool {
        assert!(
            *self.stage.borrow() == JobStage::JobWait
                || *self.stage.borrow() == JobStage::JobRunning
        );

        // record
        *self.stage.borrow_mut() = JobStage::JobEnd(result);

        // try to get next run-kind
        let retry = match result {
            JobResult::JobDone => self.update_runkind(), // the run-kind could be updated in success only
            _ => false,
        };

        // update
        if retry {
            *self.stage.borrow_mut() = JobStage::JobWait; // wait again
        }

        retry
    }

    pub(super) fn get_attr(&self, attr_kind: JobAttrKind) -> bool {
        self.attr.borrow().get(attr_kind)
    }

    pub(super) fn get_id(&self) -> u32 {
        self.id
    }

    pub(super) fn get_unit(&self) -> &Rc<UnitX> {
        &self.unit
    }

    pub(super) fn get_kind(&self) -> JobKind {
        self.kind
    }

    pub(super) fn get_runkind(&self) -> JobKind {
        *self.run_kind.borrow()
    }

    pub(super) fn get_stage(&self) -> JobStage {
        self.stage.borrow().clone()
    }

    pub(super) fn is_basic_op(&self) -> bool {
        match self.kind {
            JobKind::JobStart | JobKind::JobStop | JobKind::JobReload | JobKind::JobRestart => true,
            JobKind::JobVerify | JobKind::JobNop => true,
            JobKind::JobTryReload | JobKind::JobTryRestart | JobKind::JobReloadOrStart => false, // compound kind
        }
    }

    pub(super) fn is_order_with(&self, other: &Self, atom: UnitRelationAtom) -> i8 {
        assert!(
            atom == UnitRelationAtom::UnitAtomAfter || atom == UnitRelationAtom::UnitAtomBefore
        );
        if self.get_attr(JobAttrKind::JobIgnoreOrder) || other.get_attr(JobAttrKind::JobIgnoreOrder)
        {
            return 0;
        }

        job_order_compare(self.kind, other.kind, atom)
    }

    fn update_runkind(&self) -> bool {
        let last_rkind = self.get_runkind();
        let rkind = job_rkind_map(self.kind, last_rkind);

        // update
        *self.run_kind.borrow_mut() = rkind;

        // is there anything else to do?
        last_rkind != rkind
    }
}

#[derive(Debug)]
struct JobAttr {
    ignore_order: bool,
    irreversible: bool,
}

// the declaration "pub(self)" is for identification only.
impl JobAttr {
    pub(self) fn new() -> JobAttr {
        JobAttr {
            ignore_order: false,
            irreversible: false,
        }
    }

    pub(self) fn set(&mut self, kind: JobAttrKind, value: bool) -> bool {
        match kind {
            JobAttrKind::JobIgnoreOrder => self.ignore_order = value,
            JobAttrKind::JobIrreversible => self.irreversible = value,
        };

        value
    }

    pub(self) fn or(&mut self, kind: JobAttrKind, value: bool) -> bool {
        match kind {
            JobAttrKind::JobIgnoreOrder => {
                self.ignore_order |= value;
                self.ignore_order
            }
            JobAttrKind::JobIrreversible => {
                self.irreversible |= value;
                self.irreversible
            }
        }
    }

    pub(self) fn merge(&mut self, other: &Self) {
        self.merge_kind(other, JobAttrKind::JobIgnoreOrder);
        self.merge_kind(other, JobAttrKind::JobIrreversible);
    }

    pub(self) fn get(&self, kind: JobAttrKind) -> bool {
        match kind {
            JobAttrKind::JobIgnoreOrder => self.ignore_order,
            JobAttrKind::JobIrreversible => self.irreversible,
        }
    }

    fn merge_kind(&mut self, other: &Self, kind: JobAttrKind) {
        self.or(kind, other.get(kind));
    }
}

pub(super) fn job_process_unit(
    run_kind: JobKind,
    ns: UnitActiveState,
    flags: isize,
) -> (Option<JobResult>, bool) {
    match run_kind {
        JobKind::JobStart => job_process_unit_start(ns),
        JobKind::JobStop => job_process_unit_stop(ns),
        JobKind::JobReload => job_process_unit_reload(ns, flags),
        _ => unreachable!("Invalid job run-kind."),
    }
}

pub(super) fn job_is_unit_applicable(kind: JobKind, unit: &UnitX) -> bool {
    match kind {
        JobKind::JobStart | JobKind::JobVerify | JobKind::JobNop => true,
        JobKind::JobStop => !unit.get_perpetual(),
        JobKind::JobRestart | JobKind::JobTryRestart => unit.can_start() && unit.can_stop(),
        JobKind::JobReload | JobKind::JobTryReload => unit.can_reload(),
        JobKind::JobReloadOrStart => unit.can_start() && unit.can_reload(),
    }
}

fn job_rkind_new(kind: JobKind) -> JobKind {
    match kind {
        JobKind::JobRestart => JobKind::JobStop,
        _ => kind,
    }
}

fn job_rkind_map(kind: JobKind, last_rkind: JobKind) -> JobKind {
    match (kind, last_rkind) {
        (JobKind::JobRestart, JobKind::JobStop) => JobKind::JobStart,
        _ => kind,
    }
}

fn job_trigger_unit(run_kind: JobKind, unit: &UnitX) -> Result<(), Option<JobResult>> {
    let ret = match run_kind {
        JobKind::JobStart => unit.start(),
        JobKind::JobStop => unit.stop(),
        JobKind::JobReload => unit.reload(),
        JobKind::JobVerify => match unit.get_state() {
            UnitActiveState::UnitActive | UnitActiveState::UnitReloading => {
                Err(UnitActionError::UnitActionEAlready)
            }
            UnitActiveState::UnitActivating => Err(UnitActionError::UnitActionEAgain),
            _ => Err(UnitActionError::UnitActionEBadR),
        },
        JobKind::JobNop => Err(UnitActionError::UnitActionEAlready), // do nothing
        _ => unreachable!("Invalid job run-kind."),
    };

    match ret {
        Ok(_) => Ok(()),
        Err(err) => Err(job_trigger_err_to_result(err)),
    }
}

fn job_trigger_err_to_result(err: UnitActionError) -> Option<JobResult> {
    match err {
        UnitActionError::UnitActionEAgain => None, // re-trigger again
        UnitActionError::UnitActionEAlready => Some(JobResult::JobDone), // over already
        UnitActionError::UnitActionEComm => Some(JobResult::JobDone), // convention
        UnitActionError::UnitActionEBadR => Some(JobResult::JobSkipped),
        UnitActionError::UnitActionENoExec => Some(JobResult::JobInvalid),
        UnitActionError::UnitActionEProto => Some(JobResult::JobAssert),
        UnitActionError::UnitActionEOpNotSupp => Some(JobResult::JobUnSupported),
        UnitActionError::UnitActionENolink => Some(JobResult::JobDependency),
        UnitActionError::UnitActionEStale => Some(JobResult::JobOnce),
        UnitActionError::UnitActionEFailed => Some(JobResult::JobFailed),
        UnitActionError::UnitActionEInval => Some(JobResult::JobFailed),
    }
}

fn job_process_unit_start(ns: UnitActiveState) -> (Option<JobResult>, bool) {
    match ns {
        // something generated from the job has been done
        UnitActiveState::UnitActive => (Some(JobResult::JobDone), true),
        // something generated from the job is doing
        UnitActiveState::UnitActivating => (None, true),
        // something not generated from the job has been done
        UnitActiveState::UnitInActive => (Some(JobResult::JobDone), false),
        UnitActiveState::UnitFailed => (Some(JobResult::JobFailed), false),
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
            (Some(JobResult::JobDone), true)
        }
        // something generated from the job is doing
        UnitActiveState::UnitDeActivating => (None, true),
        // something not generated from the job has been done
        UnitActiveState::UnitActive => (Some(JobResult::JobFailed), false),
        // something not generated from the job is doing
        UnitActiveState::UnitReloading
        | UnitActiveState::UnitActivating
        | UnitActiveState::UnitMaintenance => (Some(JobResult::JobFailed), false),
    }
}

fn job_process_unit_reload(ns: UnitActiveState, flags: isize) -> (Option<JobResult>, bool) {
    let mut result = JobResult::JobDone;
    if flags & UnitNotifyFlags::UnitNotifyReloadFailure as isize != 0 {
        result = JobResult::JobFailed;
    }
    match ns {
        // something generated from the job has been done
        UnitActiveState::UnitActive => (Some(result), true),
        // something generated from the job is doing
        UnitActiveState::UnitReloading => (None, true),
        // something not generated from the job has been done
        UnitActiveState::UnitInActive => (Some(JobResult::JobDone), false),
        UnitActiveState::UnitFailed => (Some(JobResult::JobFailed), false),
        // something not generated from the job is doing
        UnitActiveState::UnitActivating
        | UnitActiveState::UnitDeActivating
        | UnitActiveState::UnitMaintenance => (None, false),
    }
}

fn job_merge_unit(kind: JobKind, unit: &UnitX) -> JobKind {
    let us_is_active_or_reloading = match unit.get_state() {
        UnitActiveState::UnitActive | UnitActiveState::UnitReloading => true,
        _ => false,
    };
    match (kind, us_is_active_or_reloading) {
        (JobKind::JobTryReload, false) => JobKind::JobNop,
        (JobKind::JobTryReload, true) => JobKind::JobReload,
        (JobKind::JobTryRestart, false) => JobKind::JobNop,
        (JobKind::JobTryRestart, true) => JobKind::JobRestart,
        (JobKind::JobReloadOrStart, false) => JobKind::JobStart,
        (JobKind::JobReloadOrStart, true) => JobKind::JobReload,
        (kind, _) => kind,
    }
}

fn job_order_compare(a: JobKind, b: JobKind, atom: UnitRelationAtom) -> i8 {
    if a == JobKind::JobNop || b == JobKind::JobNop {
        return 0; // independent
    }

    if atom == UnitRelationAtom::UnitAtomAfter {
        return -job_order_compare(b, a, UnitRelationAtom::UnitAtomBefore);
    }

    match b {
        JobKind::JobStop | JobKind::JobRestart => 1, // order: b -> a
        _ => -1,                                     // order: a -> b
    }
}

#[cfg(test)]
mod tests {
    // use super::*;

    #[test]
    fn job_test_new() {
        // let unit = Unit::new();
        // let id = 1;
        // let kind = JobKind::JobNop;
        // let job = Job::new(id, unit, kind);
        // assert_eq!(job.unit, &unit);
    }
}
