use super::job_entry::{Job, JobResult, JobStage};
use super::job_rentry::JobKind;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug)]
pub(super) struct JobStat {
    data: RefCell<JobStatData>,
}

impl JobStat {
    pub(super) fn new() -> JobStat {
        JobStat {
            data: RefCell::new(JobStatData::new()),
        }
    }

    pub(super) fn clear(&self) {
        self.data.borrow_mut().clear();
    }

    #[allow(clippy::type_complexity)]
    pub(super) fn update_change(
        &self,
        change: &(&Option<Rc<Job>>, &Option<Rc<Job>>, &Option<Rc<Job>>),
    ) {
        self.data.borrow_mut().update_change(change)
    }

    #[allow(clippy::type_complexity)]
    pub(super) fn update_changes(&self, changes: &(&Vec<Rc<Job>>, &Vec<Rc<Job>>, &Vec<Rc<Job>>)) {
        self.data.borrow_mut().update_changes(changes)
    }

    pub(super) fn update_stage_wait(&self, change: usize, inc: bool) {
        self.data.borrow_mut().update_stage_wait(change, inc)
    }

    pub(super) fn update_stage_run(&self, change: usize, inc: bool) {
        self.data.borrow_mut().update_stage_run(change, inc)
    }

    pub(super) fn clear_cnt(&self) {
        self.data.borrow_mut().clear_cnt()
    }
}

#[derive(Debug)]
struct JobStatData {
    num: JobNum, // snapshot
    cnt: JobCnt, // history
}

// the declaration "pub(self)" is for identification only.
impl JobStatData {
    pub(self) fn new() -> JobStatData {
        JobStatData {
            num: JobNum::new(),
            cnt: JobCnt::new(),
        }
    }

    pub(self) fn clear(&mut self) {
        self.num.clear();
        self.cnt.clear();
    }

    #[allow(clippy::type_complexity)]
    pub(self) fn update_change(
        &mut self,
        change: &(&Option<Rc<Job>>, &Option<Rc<Job>>, &Option<Rc<Job>>),
    ) {
        let (add, del, update) = change;
        let changes = (
            &change_to_changes(add),
            &change_to_changes(del),
            &change_to_changes(update),
        );
        self.update_changes(&changes);
    }

    #[allow(clippy::type_complexity)]
    pub(self) fn update_changes(
        &mut self,
        changes: &(&Vec<Rc<Job>>, &Vec<Rc<Job>>, &Vec<Rc<Job>>),
    ) {
        self.num.update(changes);
        let overflow = self.cnt.update(changes);
        if overflow {
            // debug
        }
    }

    pub(self) fn update_stage_wait(&mut self, change: usize, inc: bool) {
        self.num.update_stage_wait(change, inc);
    }

    pub(self) fn update_stage_run(&mut self, change: usize, inc: bool) {
        self.num.update_stage_run(change, inc);
    }

    pub(self) fn clear_cnt(&mut self) {
        self.cnt.clear();
    }
}

#[derive(Debug)]
struct JobNum {
    kind: JobKindNum,
    stage: JobStageNum,
}

// the declaration "pub(self)" is for identification only.
impl JobNum {
    pub(self) fn new() -> JobNum {
        JobNum {
            kind: JobKindNum::new(),
            stage: JobStageNum::new(),
        }
    }

    pub(self) fn clear(&mut self) {
        self.kind.clear();
        self.stage.clear();
    }

    #[allow(clippy::type_complexity)]
    pub(self) fn update(&mut self, changes: &(&Vec<Rc<Job>>, &Vec<Rc<Job>>, &Vec<Rc<Job>>)) {
        self.kind.update(changes);
    }

    pub(self) fn update_stage_wait(&mut self, change: usize, inc: bool) {
        self.stage.update_wait(change, inc);
    }

    pub(self) fn update_stage_run(&mut self, change: usize, inc: bool) {
        self.stage.update_run(change, inc);
    }
}

#[derive(Debug)]
struct JobKindNum {
    start: usize,
    stop: usize,
    reload: usize,
    restart: usize,
    verify: usize,
    nop: usize,
}

// the declaration "pub(self)" is for identification only.
impl JobKindNum {
    pub(self) fn new() -> JobKindNum {
        JobKindNum {
            start: 0,
            stop: 0,
            reload: 0,
            restart: 0,
            verify: 0,
            nop: 0,
        }
    }

    pub(self) fn clear(&mut self) {
        *self = JobKindNum {
            start: 0,
            stop: 0,
            reload: 0,
            restart: 0,
            verify: 0,
            nop: 0,
        };
    }

    #[allow(clippy::type_complexity)]
    pub(self) fn update(&mut self, changes: &(&Vec<Rc<Job>>, &Vec<Rc<Job>>, &Vec<Rc<Job>>)) {
        let (adds, dels, _) = changes;

        // del
        for job in dels.iter() {
            let overflow = match job.kind() {
                JobKind::Start => value_try_sub(&mut self.start, 1),
                JobKind::Stop => value_try_sub(&mut self.stop, 1),
                JobKind::Reload => value_try_sub(&mut self.reload, 1),
                JobKind::Restart => value_try_sub(&mut self.restart, 1),
                JobKind::Verify => value_try_sub(&mut self.verify, 1),
                JobKind::Nop => value_try_sub(&mut self.nop, 1),
                _ => unreachable!("There is an error in the transaction exchange mechanism."),
            };
            assert!(!overflow);
        }

        // add
        for job in adds.iter() {
            let overflow = match job.kind() {
                JobKind::Start => value_try_add(&mut self.start, 1),
                JobKind::Stop => value_try_add(&mut self.stop, 1),
                JobKind::Reload => value_try_add(&mut self.reload, 1),
                JobKind::Restart => value_try_add(&mut self.restart, 1),
                JobKind::Verify => value_try_add(&mut self.verify, 1),
                JobKind::Nop => value_try_add(&mut self.nop, 1),
                _ => unreachable!("There is an error in the transaction exchange mechanism."),
            };
            assert!(!overflow);
        }

        // update: do nothing
    }

    #[allow(dead_code)]
    fn total(&self) -> usize {
        self.start + self.stop + self.reload + self.restart + self.verify + self.nop
    }
}

#[derive(Debug)]
struct JobStageNum {
    wait: usize,
    running: usize,
}

// the declaration "pub(self)" is for identification only.
impl JobStageNum {
    pub(self) fn new() -> JobStageNum {
        JobStageNum {
            wait: 0,
            running: 0,
        }
    }

    pub(self) fn clear(&mut self) {
        *self = JobStageNum {
            wait: 0,
            running: 0,
        };
    }

    pub(self) fn update_wait(&mut self, change: usize, inc: bool) {
        let overflow = match inc {
            true => value_try_add(&mut self.wait, change),
            false => value_try_sub(&mut self.wait, change),
        };
        assert!(!overflow);
    }

    pub(self) fn update_run(&mut self, change: usize, inc: bool) {
        let overflow = match inc {
            true => value_try_add(&mut self.running, change),
            false => value_try_sub(&mut self.running, change),
        };
        assert!(!overflow);
    }

    #[allow(dead_code)]
    fn total(&self) -> usize {
        self.wait + self.running
    }
}

#[derive(Debug)]
struct JobCnt {
    result: JobResultCnt,
    op: JobOpCnt,
}

// the declaration "pub(self)" is for identification only.
impl JobCnt {
    pub(self) fn new() -> JobCnt {
        JobCnt {
            result: JobResultCnt::new(),
            op: JobOpCnt::new(),
        }
    }

    pub(self) fn clear(&mut self) {
        self.result.clear();
        self.op.clear();
    }

    #[allow(clippy::type_complexity)]
    pub(self) fn update(
        &mut self,
        changes: &(&Vec<Rc<Job>>, &Vec<Rc<Job>>, &Vec<Rc<Job>>),
    ) -> bool {
        let mut overflow = false;
        overflow |= self.result.update(changes);
        overflow |= self.op.update(changes);
        overflow
    }
}

#[derive(Debug)]
struct JobResultCnt {
    done: usize,
    cancelled: usize,
    timeout: usize,
    failed: usize,
    dependency: usize,
    skipped: usize,
    invalid: usize,
    assert: usize,
    unsupported: usize,
    collected: usize,
    once: usize,
    merged: usize,
}

// the declaration "pub(self)" is for identification only.
impl JobResultCnt {
    pub(self) fn new() -> JobResultCnt {
        JobResultCnt {
            done: 0,
            cancelled: 0,
            timeout: 0,
            failed: 0,
            dependency: 0,
            skipped: 0,
            invalid: 0,
            assert: 0,
            unsupported: 0,
            collected: 0,
            once: 0,
            merged: 0,
        }
    }

    pub(self) fn clear(&mut self) {
        *self = JobResultCnt {
            done: 0,
            cancelled: 0,
            timeout: 0,
            failed: 0,
            dependency: 0,
            skipped: 0,
            invalid: 0,
            assert: 0,
            unsupported: 0,
            collected: 0,
            once: 0,
            merged: 0,
        };
    }

    #[allow(clippy::type_complexity)]
    pub(self) fn update(
        &mut self,
        changes: &(&Vec<Rc<Job>>, &Vec<Rc<Job>>, &Vec<Rc<Job>>),
    ) -> bool {
        let (_, dels, _) = changes;

        let mut overflow = false;

        // del
        for job in dels.iter() {
            if let JobStage::End(result) = job.get_stage() {
                overflow |= match result {
                    JobResult::Done => value_try_add(&mut self.done, 1),
                    JobResult::Cancelled => value_try_add(&mut self.cancelled, 1),
                    JobResult::TimeOut => value_try_add(&mut self.timeout, 1),
                    JobResult::Failed => value_try_add(&mut self.failed, 1),
                    JobResult::Dependency => value_try_add(&mut self.dependency, 1),
                    JobResult::Skipped => value_try_add(&mut self.skipped, 1),
                    JobResult::Invalid => value_try_add(&mut self.invalid, 1),
                    JobResult::Assert => value_try_add(&mut self.assert, 1),
                    JobResult::UnSupported => value_try_add(&mut self.unsupported, 1),
                    JobResult::Collected => value_try_add(&mut self.collected, 1),
                    JobResult::Once => value_try_add(&mut self.once, 1),
                    JobResult::Merged => value_try_add(&mut self.merged, 1),
                };
            }
        }

        overflow
    }

    #[allow(dead_code)]
    fn total(&self) -> usize {
        self.done
            + self.cancelled
            + self.timeout
            + self.failed
            + self.dependency
            + self.skipped
            + self.invalid
            + self.assert
            + self.unsupported
            + self.collected
            + self.once
            + self.merged
    }
}

#[derive(Debug)]
struct JobOpCnt {
    add: usize,
    update: usize,
    del: usize,
}

// the declaration "pub(self)" is for identification only.
impl JobOpCnt {
    pub(self) fn new() -> JobOpCnt {
        JobOpCnt {
            add: 0,
            update: 0,
            del: 0,
        }
    }

    pub(self) fn clear(&mut self) {
        *self = JobOpCnt {
            add: 0,
            del: 0,
            update: 0,
        };
    }

    #[allow(clippy::type_complexity)]
    pub(self) fn update(
        &mut self,
        changes: &(&Vec<Rc<Job>>, &Vec<Rc<Job>>, &Vec<Rc<Job>>),
    ) -> bool {
        let (adds, dels, updates) = changes;

        let mut overflow = false;

        // add
        overflow |= value_try_add(&mut self.add, adds.len());

        // del
        overflow |= value_try_add(&mut self.del, dels.len());

        // update
        overflow |= value_try_add(&mut self.update, updates.len());

        overflow
    }

    #[allow(dead_code)]
    fn total(&self) -> usize {
        self.add + self.del + self.update
    }
}

fn change_to_changes(job: &Option<Rc<Job>>) -> Vec<Rc<Job>> {
    let mut jobs = Vec::new();
    if let Some(j) = job {
        jobs.push(Rc::clone(j));
    }
    jobs
}

fn value_try_add(value: &mut usize, add: usize) -> bool {
    let (v, o) = value.overflowing_add(add);
    if !o {
        *value = v;
    }
    o
}

fn value_try_sub(value: &mut usize, sub: usize) -> bool {
    let (v, o) = value.overflowing_sub(sub);
    if !o {
        *value = v;
    }
    o
}

#[cfg(test)]
mod tests {
    use super::super::job_rentry::JobRe;
    use super::*;
    use crate::core::manager::rentry::RELI_HISTORY_MAX_DBS;
    use crate::core::unit::test_utils;
    use crate::core::unit::DataManager;
    use crate::core::unit::UnitDb;
    use crate::core::unit::UnitRe;
    use crate::core::unit::UnitX;
    use libutils::logger;
    use sysmaster::reliability::Reliability;

    #[test]
    fn js_api() {
        let (reli, _, unit_test1) = prepare_unit_single();
        let rentry = Rc::new(JobRe::new(&reli));
        let stat = JobStat::new();
        let mut id: u32 = 0;
        id = id.wrapping_add(1); // ++
        let kind = JobKind::Start;
        let job = Rc::new(Job::new(&reli, &rentry, id, Rc::clone(&unit_test1), kind));

        // nothing exists
        assert_eq!(stat.data.borrow().num.kind.total(), 0);

        // something exists
        stat.update_change(&(&Some(job), &None, &None));
        assert_eq!(stat.data.borrow().num.kind.total(), 1);
        assert_eq!(stat.data.borrow().cnt.op.total(), 1);

        // clear
        stat.clear_cnt();
        assert_eq!(stat.data.borrow().num.kind.total(), 1);
        assert_eq!(stat.data.borrow().cnt.op.total(), 0);
    }

    fn prepare_unit_single() -> (Rc<Reliability>, Rc<UnitDb>, Rc<UnitX>) {
        let dm = Rc::new(DataManager::new());
        let reli = Rc::new(Reliability::new(RELI_HISTORY_MAX_DBS));
        let rentry = Rc::new(UnitRe::new(&reli));
        let db = Rc::new(UnitDb::new(&rentry));
        let name_test1 = String::from("test1.service");
        let unit_test1 = create_unit(&dm, &reli, &rentry, &name_test1);
        db.units_insert(name_test1, Rc::clone(&unit_test1));
        (reli, db, unit_test1)
    }

    fn create_unit(
        dmr: &Rc<DataManager>,
        relir: &Rc<Reliability>,
        rentryr: &Rc<UnitRe>,
        name: &str,
    ) -> Rc<UnitX> {
        logger::init_log_with_console("test_unit_load", 4);
        log::info!("test");
        test_utils::create_unit_for_test_pub(dmr, relir, rentryr, name)
    }
}
