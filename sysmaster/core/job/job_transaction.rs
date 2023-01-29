use super::job_alloc::JobAlloc;
use super::job_entry::{self, Job, JobConf, JobResult};
use super::job_rentry::JobKind;
use super::job_table::JobTable;
use super::JobErrno;
use crate::core::unit::JobMode;
use crate::core::unit::UnitDb;
use crate::core::unit::UnitRelationAtom;
use crate::core::unit::UnitX;
use std::rc::Rc;
use sysmaster::unit::UnitActionError;

pub(super) fn job_trans_expand(
    stage: &JobTable,
    ja: &JobAlloc,
    db: &UnitDb,
    config: &JobConf,
    mode: JobMode,
) -> Result<(), JobErrno> {
    // check input
    //trans_expand_check_input(config)?;

    // record
    let conf = JobConf::map(config);
    let new = stage.record_suspend(ja, &conf, mode);

    // expand
    if trans_is_expand(&conf, new, mode) {
        match conf.get_kind() {
            JobKind::Start => trans_expand_start(stage, ja, db, &conf, mode)?,
            JobKind::Stop => trans_expand_stop(stage, ja, db, &conf, mode)?,
            JobKind::Reload => trans_expand_reload(stage, ja, db, &conf, mode)?,
            JobKind::Restart => {
                trans_expand_start(stage, ja, db, &conf, mode)?;
                trans_expand_stop(stage, ja, db, &conf, mode)?
            }
            JobKind::Verify | JobKind::Nop => {}
            _ => unreachable!("Invalid job kind."),
        }
    }

    Ok(())
    // the jobs expanded do not need to be reverted separately, which are reverted in the up-level caller 'JobManagerData->exec()' uniformly.
}

pub(super) fn job_trans_affect(
    stage: &JobTable,
    ja: &JobAlloc,
    db: &UnitDb,
    config: &JobConf,
    mode: JobMode,
) -> Result<(), JobErrno> {
    match mode {
        JobMode::Isolate => trans_affect_isolate(stage, ja, db, mode),
        JobMode::Trigger => trans_affect_trigger(stage, ja, db, config, mode),
        _ => Ok(()), // do nothing
    }
}

pub(super) fn job_trans_verify(
    stage: &JobTable,
    jobs: &JobTable,
    mode: JobMode,
) -> Result<(), JobErrno> {
    // job-list + unit-list(from db) -> job-list' => stage
    // todo!(); transaction_activate: the other parts is waiting for future support

    trans_verify_is_conflict(stage)?;
    trans_verify_is_destructive(stage, jobs, mode)?;

    Ok(())
}

pub(super) fn job_trans_fallback(
    jobs: &JobTable,
    db: &UnitDb,
    unit: &UnitX,
    run_kind: JobKind,
    f_result: JobResult,
) -> Vec<Rc<Job>> {
    let mut del_jobs = Vec::new();
    trans_fallback_body(jobs, db, unit, run_kind, f_result, &mut del_jobs);
    del_jobs
}

#[allow(dead_code)]
fn trans_expand_check_input(config: &JobConf) -> Result<(), JobErrno> {
    let kind = config.get_kind();
    let unit = config.get_unit();

    if !unit.is_load_complete() {
        return Err(JobErrno::Input);
    }

    if kind != JobKind::Stop {
        let err = match unit.try_load() {
            Ok(()) => Ok(()),
            Err(UnitActionError::UnitActionEBadR) => Err(JobErrno::BadRequest),
            Err(_) => Err(JobErrno::Input),
        };
        return err;
    }

    if !job_entry::job_is_unit_applicable(kind, unit) {
        return Err(JobErrno::Input);
    }

    Ok(())
}

fn trans_expand_start(
    stage: &JobTable,
    ja: &JobAlloc,
    db: &UnitDb,
    config: &JobConf,
    mode: JobMode,
) -> Result<(), JobErrno> {
    let unit = config.get_unit();

    let atom = UnitRelationAtom::UnitAtomPullInStart;
    for other in db.dep_gets_atom(unit, atom).iter() {
        let conf = JobConf::new(other, JobKind::Start);
        if let Err(err) = job_trans_expand(stage, ja, db, &conf, mode) {
            // debug
            if JobErrno::BadRequest != err {
                return Err(err);
            }
        }
    }

    let atom = UnitRelationAtom::UnitAtomPullInStartIgnored;
    for other in db.dep_gets_atom(unit, atom).iter() {
        let conf = JobConf::new(other, JobKind::Start);
        if let Err(_err) = job_trans_expand(stage, ja, db, &conf, mode) {
            // debug
        }
    }

    let atom = UnitRelationAtom::UnitAtomPullInVerify;
    for other in db.dep_gets_atom(unit, atom).iter() {
        let conf = JobConf::new(other, JobKind::Verify);
        if let Err(err) = job_trans_expand(stage, ja, db, &conf, mode) {
            // debug
            if JobErrno::BadRequest != err {
                return Err(err);
            }
        }
    }

    let atom = UnitRelationAtom::UnitAtomPullInStop;
    for other in db.dep_gets_atom(unit, atom).iter() {
        let conf = JobConf::new(other, JobKind::Stop);
        if let Err(err) = job_trans_expand(stage, ja, db, &conf, mode) {
            // debug
            if JobErrno::BadRequest != err {
                return Err(err);
            }
        }
    }

    let atom = UnitRelationAtom::UnitAtomPullInStopIgnored;
    for other in db.dep_gets_atom(unit, atom).iter() {
        let conf = JobConf::new(other, JobKind::Stop);
        if let Err(_err) = job_trans_expand(stage, ja, db, &conf, mode) {
            // debug
        }
    }

    Ok(())
}

fn trans_expand_stop(
    stage: &JobTable,
    ja: &JobAlloc,
    db: &UnitDb,
    config: &JobConf,
    mode: JobMode,
) -> Result<(), JobErrno> {
    let unit = config.get_unit();

    let (expand_atom, expand_kind) = match config.get_kind() {
        JobKind::Stop => (UnitRelationAtom::UnitAtomPropagateStop, JobKind::Stop),
        JobKind::Restart => (
            UnitRelationAtom::UnitAtomPropagateRestart,
            JobKind::TryRestart,
        ),
        _ => unreachable!("invalid configuration."),
    };

    for other in db.dep_gets_atom(unit, expand_atom).iter() {
        let conf = JobConf::new(other, expand_kind);
        if let Err(err) = job_trans_expand(stage, ja, db, &conf, mode) {
            // debug
            if JobErrno::BadRequest != err {
                return Err(err);
            }
        }
    }

    Ok(())
}

fn trans_expand_reload(
    stage: &JobTable,
    ja: &JobAlloc,
    db: &UnitDb,
    config: &JobConf,
    mode: JobMode,
) -> Result<(), JobErrno> {
    let unit = config.get_unit();

    let atom = UnitRelationAtom::UnitAtomPropagatesReloadTo;
    for other in db.dep_gets_atom(unit, atom).iter() {
        let conf = JobConf::new(other, JobKind::TryReload);
        if let Err(_err) = job_trans_expand(stage, ja, db, &conf, mode) {
            // debug
        }
    }

    Ok(())
}

fn trans_is_expand(config: &JobConf, new: bool, mode: JobMode) -> bool {
    // the job is a 'nop', nothing needs to be expanded.
    if config.get_kind() == JobKind::Nop {
        return false;
    }

    // the job is not a new one, it has been expanded already.
    if !new {
        return false;
    }

    // the configuration tells us that expanding is ignored.
    if mode == JobMode::IgnoreDependencies || mode == JobMode::IgnoreRequirements {
        return false;
    }

    // all conditions are satisfied
    true
}

fn trans_affect_isolate(
    stage: &JobTable,
    ja: &JobAlloc,
    db: &UnitDb,
    mode: JobMode,
) -> Result<(), JobErrno> {
    assert_eq!(mode, JobMode::Isolate);

    for other in db.units_get_all(None).iter() {
        // it is allowed not to be affected by isolation
        if let true = other
            .get_config()
            .config_data()
            .borrow()
            .Unit
            .IgnoreOnIsolate
        {
            continue;
        }

        // there is something assigned, not affected
        if !stage.is_unit_empty(other) {
            continue;
        }

        // isolate(stop)
        let conf = JobConf::new(other, JobKind::Stop);
        if let Err(_err) = job_trans_expand(stage, ja, db, &conf, mode) {
            // debug
        }
    }

    Ok(())
    // the jobs expanded do not need to be reverted separately, which are reverted in the up-level caller 'JobManagerData->exec()' uniformly.
}

fn trans_affect_trigger(
    stage: &JobTable,
    ja: &JobAlloc,
    db: &UnitDb,
    config: &JobConf,
    mode: JobMode,
) -> Result<(), JobErrno> {
    assert_eq!(config.get_kind(), JobKind::Stop); // guaranteed by 'job_trans_check_input'
    assert_eq!(mode, JobMode::Trigger);

    let unit = config.get_unit();
    let atom = UnitRelationAtom::UnitAtomTriggeredBy;
    for other in db.dep_gets_atom(unit, atom).iter() {
        // there is something assigned, not affected
        if !stage.is_unit_empty(unit) {
            continue;
        }

        // trigger(stop)
        let conf = JobConf::new(other, JobKind::Stop);
        if let Err(_err) = job_trans_expand(stage, ja, db, &conf, mode) {
            // debug
        }
    }

    Ok(())
    // the jobs expanded do not need to be reverted separately, which are reverted in the up-level caller 'JobManagerData->exec()' uniformly.
}

fn trans_verify_is_conflict(stage: &JobTable) -> Result<(), JobErrno> {
    if stage.is_suspends_conflict() {
        return Err(JobErrno::Conflict);
    }

    Ok(())
}

fn trans_verify_is_destructive(
    stage: &JobTable,
    jobs: &JobTable,
    mode: JobMode,
) -> Result<(), JobErrno> {
    assert!(!jobs.is_suspends_conflict());

    // non-conflicting
    if !jobs.is_suspends_conflict_with(stage) {
        return Ok(());
    }

    // conflicting, but replaceable
    if mode != JobMode::Fail && jobs.is_suspends_replace_with(stage) {
        return Ok(());
    }

    // conflicting, and non-replaceable
    Err(JobErrno::Conflict)
}

fn trans_fallback_body(
    jobs: &JobTable,
    db: &UnitDb,
    unit: &UnitX,
    run_kind: JobKind,
    f_result: JobResult,
    del_jobs: &mut Vec<Rc<Job>>,
) {
    // explore one step
    let mut dels = trans_fallback_action(jobs, db, unit, run_kind, f_result);

    // explore one step more?
    if !dels.is_empty() {
        for job in dels.iter() {
            trans_fallback_body(jobs, db, job.unit(), job.run_kind(), f_result, del_jobs);
        }
    }

    // record
    del_jobs.append(&mut dels);
}

fn trans_fallback_action(
    jobs: &JobTable,
    db: &UnitDb,
    unit: &UnitX,
    run_kind: JobKind,
    f_result: JobResult,
) -> Vec<Rc<Job>> {
    match run_kind {
        JobKind::Start => trans_fallback_start(jobs, db, unit, f_result),
        JobKind::Stop => trans_fallback_stop(jobs, db, unit, f_result),
        JobKind::Verify => trans_fallback_start(jobs, db, unit, f_result),
        _ => Vec::new(), // nothing to fallback
    }
}

fn trans_fallback_start(
    jobs: &JobTable,
    db: &UnitDb,
    unit: &UnitX,
    f_result: JobResult,
) -> Vec<Rc<Job>> {
    let atom = UnitRelationAtom::UnitAtomPropagateStartFailure;
    trans_fallback(jobs, db, unit, f_result, atom)
}

fn trans_fallback_stop(
    jobs: &JobTable,
    db: &UnitDb,
    unit: &UnitX,
    f_result: JobResult,
) -> Vec<Rc<Job>> {
    let atom = UnitRelationAtom::UnitAtomPropagateStopFailure;
    trans_fallback(jobs, db, unit, f_result, atom)
}

fn trans_fallback(
    jobs: &JobTable,
    db: &UnitDb,
    unit: &UnitX,
    f_result: JobResult,
    atom: UnitRelationAtom,
) -> Vec<Rc<Job>> {
    let mut del_jobs = Vec::new();
    let kind1 = JobKind::Start;
    let kind2 = JobKind::Verify;
    for other in db.dep_gets_atom(unit, atom) {
        del_jobs.append(&mut jobs.remove_suspends(&other, kind1, Some(kind2), f_result));
    }
    del_jobs
}

#[cfg(test)]
mod tests {
    use super::super::job_rentry::JobRe;
    use super::*;
    use crate::core::manager::rentry::RELI_HISTORY_MAX_DBS;
    use crate::core::unit::test_utils;
    use crate::core::unit::DataManager;
    use crate::core::unit::{UnitRe, UnitRelations};
    use libutils::logger;
    use sysmaster::reliability::Reliability;

    #[test]
    fn jt_api_expand_check() {}

    #[test]
    fn jt_api_expand_start_multi() {
        let relation = UnitRelations::UnitRequires;
        let (reli, db, unit_test1, _unit_test2) = prepare_unit_multi(relation);
        let rentry = Rc::new(JobRe::new(&reli));
        let table = JobTable::new(&db);
        let ja = JobAlloc::new(&reli, &rentry);

        let conf = JobConf::new(&unit_test1, JobKind::Start);
        let ret = job_trans_expand(&table, &ja, &db, &conf, JobMode::Replace);
        assert!(ret.is_ok());
        assert_eq!(table.len(), 2);
    }

    #[test]
    fn jt_api_expand_start_single() {
        let (reli, db, unit_test1) = prepare_unit_single();
        let rentry = Rc::new(JobRe::new(&reli));
        let table = JobTable::new(&db);
        let ja = JobAlloc::new(&reli, &rentry);

        let conf = JobConf::new(&unit_test1, JobKind::Start);
        let ret = job_trans_expand(&table, &ja, &db, &conf, JobMode::Replace);
        assert!(ret.is_ok());
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn jt_api_expand_stop_multi() {
        let relation = UnitRelations::UnitRequires;
        let (reli, db, _unit_test1, unit_test2) = prepare_unit_multi(relation);
        let rentry = Rc::new(JobRe::new(&reli));
        let table = JobTable::new(&db);
        let ja = JobAlloc::new(&reli, &rentry);

        let conf = JobConf::new(&unit_test2, JobKind::Stop);
        let ret = job_trans_expand(&table, &ja, &db, &conf, JobMode::Replace);
        assert!(ret.is_ok());
        assert_eq!(table.len(), 2);
    }

    #[test]
    fn jt_api_expand_stop_single() {
        let (reli, db, unit_test1) = prepare_unit_single();
        let rentry = Rc::new(JobRe::new(&reli));
        let table = JobTable::new(&db);
        let ja = JobAlloc::new(&reli, &rentry);

        let conf = JobConf::new(&unit_test1, JobKind::Stop);
        let ret = job_trans_expand(&table, &ja, &db, &conf, JobMode::Replace);
        assert!(ret.is_ok());
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn jt_api_expand_reload_multi() {
        let relation = UnitRelations::UnitRequires;
        let (reli, db, unit_test1, _unit_test2) = prepare_unit_multi(relation);
        let rentry = Rc::new(JobRe::new(&reli));
        let table = JobTable::new(&db);
        let ja = JobAlloc::new(&reli, &rentry);

        let conf = JobConf::new(&unit_test1, JobKind::Reload);
        let ret = job_trans_expand(&table, &ja, &db, &conf, JobMode::Replace);
        assert!(ret.is_ok());
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn jt_api_expand_reload_single() {
        let (reli, db, unit_test1) = prepare_unit_single();
        let rentry = Rc::new(JobRe::new(&reli));
        let table = JobTable::new(&db);
        let ja = JobAlloc::new(&reli, &rentry);

        let conf = JobConf::new(&unit_test1, JobKind::Reload);
        let ret = job_trans_expand(&table, &ja, &db, &conf, JobMode::Replace);
        assert!(ret.is_ok());
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn jt_api_affect_isolate_multi() {
        let relation = UnitRelations::UnitRequires;
        let (reli, db, unit_test1, _unit_test2) = prepare_unit_multi(relation);
        let rentry = Rc::new(JobRe::new(&reli));
        let table = JobTable::new(&db);
        let ja = JobAlloc::new(&reli, &rentry);

        let conf = JobConf::new(&unit_test1, JobKind::Start);
        let ret = job_trans_expand(&table, &ja, &db, &conf, JobMode::Replace);
        assert!(ret.is_ok());
        let ret = job_trans_affect(&table, &ja, &db, &conf, JobMode::Replace);
        assert!(ret.is_ok());
    }

    #[test]
    fn jt_api_affect_isolate_single() {
        let (reli, db, unit_test1) = prepare_unit_single();
        let rentry = Rc::new(JobRe::new(&reli));
        let table = JobTable::new(&db);
        let ja = JobAlloc::new(&reli, &rentry);

        let conf = JobConf::new(&unit_test1, JobKind::Start);
        let ret = job_trans_expand(&table, &ja, &db, &conf, JobMode::Replace);
        assert!(ret.is_ok());
        let ret = job_trans_affect(&table, &ja, &db, &conf, JobMode::Replace);
        assert!(ret.is_ok());
    }

    #[test]
    fn jt_api_affect_trigger_multi() {
        let relation = UnitRelations::UnitTriggers;
        let (reli, db, unit_test1, _unit_test2) = prepare_unit_multi(relation);
        let rentry = Rc::new(JobRe::new(&reli));
        let table = JobTable::new(&db);
        let ja = JobAlloc::new(&reli, &rentry);

        let conf = JobConf::new(&unit_test1, JobKind::Stop);
        let ret = job_trans_expand(&table, &ja, &db, &conf, JobMode::Replace);
        assert!(ret.is_ok());
        let ret = job_trans_affect(&table, &ja, &db, &conf, JobMode::Replace);
        assert!(ret.is_ok());
    }

    #[test]
    fn jt_api_affect_trigger_single() {
        let (reli, db, unit_test1) = prepare_unit_single();
        let rentry = Rc::new(JobRe::new(&reli));
        let table = JobTable::new(&db);
        let ja = JobAlloc::new(&reli, &rentry);

        let conf = JobConf::new(&unit_test1, JobKind::Stop);
        let ret = job_trans_expand(&table, &ja, &db, &conf, JobMode::Replace);
        assert!(ret.is_ok());
        let ret = job_trans_affect(&table, &ja, &db, &conf, JobMode::Replace);
        assert!(ret.is_ok());
    }

    #[test]
    fn jt_api_fallback_start() {
        let relation = UnitRelations::UnitRequires;
        let (reli, db, unit_test1, unit_test2) = prepare_unit_multi(relation);
        let rentry = Rc::new(JobRe::new(&reli));
        let jobs = JobTable::new(&db);
        let stage = JobTable::new(&db);
        let ja = JobAlloc::new(&reli, &rentry);
        let mode = JobMode::Replace;
        let runkind = JobKind::Start;
        let ret_rel = JobResult::Dependency;

        // nothing exists
        let ret = job_trans_fallback(&stage, &db, &unit_test1, runkind, ret_rel);
        assert_eq!(ret.len(), 0);

        // something exists
        let conf = JobConf::new(&unit_test1, runkind);
        let ret = job_trans_expand(&stage, &ja, &db, &conf, mode);
        assert!(ret.is_ok());
        let ret = jobs.commit(&stage, mode);
        assert!(ret.is_ok());
        assert_eq!(jobs.len(), 2);
        let ret = job_trans_fallback(&jobs, &db, &unit_test2, runkind, ret_rel);
        assert_eq!(ret.len(), 1);
    }

    #[test]
    fn jt_api_fallback_stop() {
        let relation = UnitRelations::UnitRequires;
        let (reli, db, unit_test1, unit_test2) = prepare_unit_multi(relation);
        let rentry = Rc::new(JobRe::new(&reli));
        let jobs = JobTable::new(&db);
        let stage = JobTable::new(&db);
        let ja = JobAlloc::new(&reli, &rentry);
        let mode = JobMode::Replace;
        let runkind = JobKind::Stop;
        let ret_rel = JobResult::Dependency;

        // nothing exists
        let ret = job_trans_fallback(&stage, &db, &unit_test1, runkind, ret_rel);
        assert_eq!(ret.len(), 0);

        // something exists
        let conf = JobConf::new(&unit_test2, runkind);
        let ret = job_trans_expand(&stage, &ja, &db, &conf, mode);
        assert!(ret.is_ok());
        let ret = jobs.commit(&stage, mode);
        assert!(ret.is_ok());
        assert_eq!(jobs.len(), 2);
        let ret = job_trans_fallback(&jobs, &db, &unit_test1, runkind, ret_rel);
        assert_eq!(ret.len(), 0);
    }

    fn prepare_unit_multi(
        relation: UnitRelations,
    ) -> (Rc<Reliability>, Rc<UnitDb>, Rc<UnitX>, Rc<UnitX>) {
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
        let u1 = Rc::clone(&unit_test1);
        let u2 = Rc::clone(&unit_test2);
        db.dep_insert(u1, relation, u2, true, 0).unwrap();
        (reli, db, unit_test1, unit_test2)
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
        logger::init_log_with_console("test_unit_load", log::LevelFilter::Trace);
        log::info!("test");
        test_utils::create_unit_for_test_pub(dmr, relir, rentryr, name)
    }
}
