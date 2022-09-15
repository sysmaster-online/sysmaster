#![warn(unused_imports)]
use super::job_alloc::JobAlloc;
use super::job_entry::{self, Job, JobConf, JobKind, JobResult};
use super::job_table::JobTable;
use super::JobErrno;
use crate::manager::unit::unit_base::UnitActionError;
use crate::manager::unit::unit_base::{JobMode, UnitRelationAtom};
use crate::manager::unit::unit_datastore::UnitDb;
use crate::manager::unit::unit_entry::UnitX;
use std::rc::Rc;

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
    let new = stage.record_suspend(ja, conf.clone(), mode, false);

    // expand
    if trans_is_expand(&conf, new, mode) {
        match conf.get_kind() {
            JobKind::JobStart => trans_expand_start(stage, ja, db, &conf, mode)?,
            JobKind::JobStop => trans_expand_stop(stage, ja, db, &conf, mode)?,
            JobKind::JobReload => trans_expand_reload(stage, ja, db, &conf, mode)?,
            JobKind::JobRestart => {
                trans_expand_start(stage, ja, db, &conf, mode)?;
                trans_expand_stop(stage, ja, db, &conf, mode)?
            }
            JobKind::JobVerify | JobKind::JobNop => {}
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
        JobMode::JobIsolate => trans_affect_isolate(stage, ja, db, mode),
        JobMode::JobTrigger => trans_affect_trigger(stage, ja, db, config, mode),
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
) -> Vec<Rc<Job>> {
    match run_kind {
        JobKind::JobStart => trans_fallback_start(jobs, db, unit),
        JobKind::JobStop => trans_fallback_stop(jobs, db, unit),
        JobKind::JobVerify => trans_fallback_start(jobs, db, unit),
        _ => Vec::new(), // nothing to fallback
    }
}

fn trans_expand_check_input(config: &JobConf) -> Result<(), JobErrno> {
    let kind = config.get_kind();
    let unit = config.get_unit();

    if !unit.is_load_complete() {
        return Err(JobErrno::JobErrInput);
    }

    if kind != JobKind::JobStop {
        let err = match unit.try_load() {
            Ok(()) => Ok(()),
            Err(UnitActionError::UnitActionEBadR) => Err(JobErrno::JobErrBadRequest),
            Err(_) => Err(JobErrno::JobErrInput),
        };
        return err;
    }

    if !job_entry::job_is_unit_applicable(kind, unit) {
        return Err(JobErrno::JobErrInput);
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

    for other in db
        .dep_gets_atom(unit, UnitRelationAtom::UnitAtomPullInStart)
        .iter()
    {
        if let Err(err) = job_trans_expand(
            stage,
            ja,
            db,
            &JobConf::new(Rc::clone(other), JobKind::JobStart),
            mode,
        ) {
            // debug
            if JobErrno::JobErrBadRequest != err {
                return Err(err);
            }
        }
    }
    for other in db
        .dep_gets_atom(unit, UnitRelationAtom::UnitAtomPullInStartIgnored)
        .iter()
    {
        if let Err(_err) = job_trans_expand(
            stage,
            ja,
            db,
            &JobConf::new(Rc::clone(other), JobKind::JobStart),
            mode,
        ) {
            // debug
        }
    }
    for other in db
        .dep_gets_atom(unit, UnitRelationAtom::UnitAtomPullInVerify)
        .iter()
    {
        if let Err(err) = job_trans_expand(
            stage,
            ja,
            db,
            &JobConf::new(Rc::clone(other), JobKind::JobVerify),
            mode,
        ) {
            // debug
            if JobErrno::JobErrBadRequest != err {
                return Err(err);
            }
        }
    }
    for other in db
        .dep_gets_atom(unit, UnitRelationAtom::UnitAtomPullInStop)
        .iter()
    {
        if let Err(err) = job_trans_expand(
            stage,
            ja,
            db,
            &JobConf::new(Rc::clone(other), JobKind::JobStop),
            mode,
        ) {
            // debug
            if JobErrno::JobErrBadRequest != err {
                return Err(err);
            }
        }
    }
    for other in db
        .dep_gets_atom(unit, UnitRelationAtom::UnitAtomPullInStopIgnored)
        .iter()
    {
        if let Err(_err) = job_trans_expand(
            stage,
            ja,
            db,
            &JobConf::new(Rc::clone(other), JobKind::JobStop),
            mode,
        ) {
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
        JobKind::JobStop => (UnitRelationAtom::UnitAtomPropagateStop, JobKind::JobStop),
        JobKind::JobRestart => (
            UnitRelationAtom::UnitAtomPropagateRestart,
            JobKind::JobTryRestart,
        ),
        _ => unreachable!("invalid configuration."),
    };

    for other in db.dep_gets_atom(unit, expand_atom).iter() {
        if let Err(err) = job_trans_expand(
            stage,
            ja,
            db,
            &JobConf::new(Rc::clone(other), expand_kind),
            mode,
        ) {
            // debug
            if JobErrno::JobErrBadRequest != err {
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

    for other in db
        .dep_gets_atom(unit, UnitRelationAtom::UnitAtomPropagatesReloadTo)
        .iter()
    {
        if let Err(_err) = job_trans_expand(
            stage,
            ja,
            db,
            &JobConf::new(Rc::clone(other), JobKind::JobTryReload),
            mode,
        ) {
            // debug
        }
    }

    Ok(())
}

fn trans_is_expand(config: &JobConf, new: bool, mode: JobMode) -> bool {
    // the job is a 'nop', nothing needs to be expanded.
    if config.get_kind() == JobKind::JobNop {
        return false;
    }

    // the job is not a new one, it has been expanded already.
    if !new {
        return false;
    }

    // the configuration tells us that expanding is ignored.
    if mode == JobMode::JobIgnoreDependencies || mode == JobMode::JobIgnoreRequirements {
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
    assert_eq!(mode, JobMode::JobIsolate);

    for other in db.units_get_all().iter() {
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
        if let Err(_err) = job_trans_expand(
            stage,
            ja,
            db,
            &JobConf::new(Rc::clone(other), JobKind::JobStop),
            mode,
        ) {
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
    assert_eq!(config.get_kind(), JobKind::JobStop); // guaranteed by 'job_trans_check_input'
    assert_eq!(mode, JobMode::JobTrigger);

    let unit = config.get_unit();
    for other in db
        .dep_gets_atom(unit, UnitRelationAtom::UnitAtomTriggeredBy)
        .iter()
    {
        // there is something assigned, not affected
        if !stage.is_unit_empty(unit) {
            continue;
        }

        // trigger(stop)
        if let Err(_err) = job_trans_expand(
            stage,
            ja,
            db,
            &JobConf::new(Rc::clone(other), JobKind::JobStop),
            mode,
        ) {
            // debug
        }
    }

    Ok(())
    // the jobs expanded do not need to be reverted separately, which are reverted in the up-level caller 'JobManagerData->exec()' uniformly.
}

fn trans_verify_is_conflict(stage: &JobTable) -> Result<(), JobErrno> {
    if stage.is_suspends_conflict() {
        return Err(JobErrno::JobErrConflict);
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
    if mode != JobMode::JobFail && jobs.is_suspends_replace_with(stage) {
        return Ok(());
    }

    // conflicting, and non-replaceable
    Err(JobErrno::JobErrConflict)
}

fn trans_fallback_start(jobs: &JobTable, db: &UnitDb, unit: &UnitX) -> Vec<Rc<Job>> {
    trans_fallback(
        jobs,
        db,
        unit,
        UnitRelationAtom::UnitAtomPropagateStartFailure,
    )
}

fn trans_fallback_stop(jobs: &JobTable, db: &UnitDb, unit: &UnitX) -> Vec<Rc<Job>> {
    trans_fallback(
        jobs,
        db,
        unit,
        UnitRelationAtom::UnitAtomPropagateStopFailure,
    )
}

fn trans_fallback(
    jobs: &JobTable,
    db: &UnitDb,
    unit: &UnitX,
    atom: UnitRelationAtom,
) -> Vec<Rc<Job>> {
    let mut del_jobs = Vec::new();
    for other in db.dep_gets_atom(unit, atom) {
        del_jobs.append(&mut jobs.remove_suspends(
            db,
            &other,
            JobKind::JobStart,
            Some(JobKind::JobVerify),
            JobResult::JobDependency,
        ));
    }
    del_jobs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manager::data::{DataManager, UnitRelations};
    use crate::manager::unit::uload_util::UnitFile;
    use crate::manager::unit::unit_base::UnitType;
    use crate::plugin::Plugin;
    use utils::logger;

    #[test]
    fn jt_api_expand_check() {}

    #[test]
    fn jt_api_expand_start_multi() {
        let relation = UnitRelations::UnitRequires;
        let (db, unit_test1, _unit_test2) = prepare_unit_multi(relation);
        let table = JobTable::new();
        let ja = JobAlloc::new();

        let conf = JobConf::new(Rc::clone(&unit_test1), JobKind::JobStart);
        let ret = job_trans_expand(&table, &ja, &db, &conf, JobMode::JobReplace);
        assert!(ret.is_ok());
        assert_eq!(table.len(), 2);
    }

    #[test]
    fn jt_api_expand_start_single() {
        let (db, unit_test1) = prepare_unit_single();
        let table = JobTable::new();
        let ja = JobAlloc::new();

        let conf = JobConf::new(Rc::clone(&unit_test1), JobKind::JobStart);
        let ret = job_trans_expand(&table, &ja, &db, &conf, JobMode::JobReplace);
        assert!(ret.is_ok());
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn jt_api_expand_stop_multi() {
        let relation = UnitRelations::UnitRequires;
        let (db, _unit_test1, unit_test2) = prepare_unit_multi(relation);
        let table = JobTable::new();
        let ja = JobAlloc::new();

        let conf = JobConf::new(Rc::clone(&unit_test2), JobKind::JobStop);
        let ret = job_trans_expand(&table, &ja, &db, &conf, JobMode::JobReplace);
        assert!(ret.is_ok());
        assert_eq!(table.len(), 2);
    }

    #[test]
    fn jt_api_expand_stop_single() {
        let (db, unit_test1) = prepare_unit_single();
        let table = JobTable::new();
        let ja = JobAlloc::new();

        let conf = JobConf::new(Rc::clone(&unit_test1), JobKind::JobStop);
        let ret = job_trans_expand(&table, &ja, &db, &conf, JobMode::JobReplace);
        assert!(ret.is_ok());
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn jt_api_expand_reload_multi() {
        let relation = UnitRelations::UnitRequires;
        let (db, unit_test1, _unit_test2) = prepare_unit_multi(relation);
        let table = JobTable::new();
        let ja = JobAlloc::new();

        let conf = JobConf::new(Rc::clone(&unit_test1), JobKind::JobReload);
        let ret = job_trans_expand(&table, &ja, &db, &conf, JobMode::JobReplace);
        assert!(ret.is_ok());
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn jt_api_expand_reload_single() {
        let (db, unit_test1) = prepare_unit_single();
        let table = JobTable::new();
        let ja = JobAlloc::new();

        let conf = JobConf::new(Rc::clone(&unit_test1), JobKind::JobReload);
        let ret = job_trans_expand(&table, &ja, &db, &conf, JobMode::JobReplace);
        assert!(ret.is_ok());
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn jt_api_affect_isolate_multi() {
        let relation = UnitRelations::UnitRequires;
        let (db, unit_test1, _unit_test2) = prepare_unit_multi(relation);
        let table = JobTable::new();
        let ja = JobAlloc::new();

        let conf = JobConf::new(Rc::clone(&unit_test1), JobKind::JobStart);
        let ret = job_trans_expand(&table, &ja, &db, &conf, JobMode::JobReplace);
        assert!(ret.is_ok());
        let ret = job_trans_affect(&table, &ja, &db, &conf, JobMode::JobReplace);
        assert!(ret.is_ok());
    }

    #[test]
    fn jt_api_affect_isolate_single() {
        let (db, unit_test1) = prepare_unit_single();
        let table = JobTable::new();
        let ja = JobAlloc::new();

        let conf = JobConf::new(Rc::clone(&unit_test1), JobKind::JobStart);
        let ret = job_trans_expand(&table, &ja, &db, &conf, JobMode::JobReplace);
        assert!(ret.is_ok());
        let ret = job_trans_affect(&table, &ja, &db, &conf, JobMode::JobReplace);
        assert!(ret.is_ok());
    }

    #[test]
    fn jt_api_affect_trigger_multi() {
        let relation = UnitRelations::UnitTriggers;
        let (db, unit_test1, _unit_test2) = prepare_unit_multi(relation);
        let table = JobTable::new();
        let ja = JobAlloc::new();

        let conf = JobConf::new(Rc::clone(&unit_test1), JobKind::JobStop);
        let ret = job_trans_expand(&table, &ja, &db, &conf, JobMode::JobReplace);
        assert!(ret.is_ok());
        let ret = job_trans_affect(&table, &ja, &db, &conf, JobMode::JobReplace);
        assert!(ret.is_ok());
    }

    #[test]
    fn jt_api_affect_trigger_single() {
        let (db, unit_test1) = prepare_unit_single();
        let table = JobTable::new();
        let ja = JobAlloc::new();

        let conf = JobConf::new(Rc::clone(&unit_test1), JobKind::JobStop);
        let ret = job_trans_expand(&table, &ja, &db, &conf, JobMode::JobReplace);
        assert!(ret.is_ok());
        let ret = job_trans_affect(&table, &ja, &db, &conf, JobMode::JobReplace);
        assert!(ret.is_ok());
    }

    #[test]
    fn jt_api_fallback_start() {
        let relation = UnitRelations::UnitRequires;
        let (db, unit_test1, unit_test2) = prepare_unit_multi(relation);
        let jobs = JobTable::new();
        let stage = JobTable::new();
        let ja = JobAlloc::new();
        let mode = JobMode::JobReplace;

        // nothing exists
        let ret = job_trans_fallback(&stage, &db, &unit_test1, JobKind::JobStart);
        assert_eq!(ret.len(), 0);

        // something exists
        let conf = JobConf::new(Rc::clone(&unit_test1), JobKind::JobStart);
        let ret = job_trans_expand(&stage, &ja, &db, &conf, mode);
        assert!(ret.is_ok());
        let ret = jobs.commit(&stage, mode);
        assert!(ret.is_ok());
        assert_eq!(jobs.len(), 2);
        let ret = job_trans_fallback(&jobs, &db, &unit_test2, JobKind::JobStart);
        assert_eq!(ret.len(), 1);
    }

    #[test]
    fn jt_api_fallback_stop() {
        let relation = UnitRelations::UnitRequires;
        let (db, unit_test1, unit_test2) = prepare_unit_multi(relation);
        let jobs = JobTable::new();
        let stage = JobTable::new();
        let ja = JobAlloc::new();
        let mode = JobMode::JobReplace;

        // nothing exists
        let ret = job_trans_fallback(&stage, &db, &unit_test1, JobKind::JobStop);
        assert_eq!(ret.len(), 0);

        // something exists
        let conf = JobConf::new(Rc::clone(&unit_test2), JobKind::JobStop);
        let ret = job_trans_expand(&stage, &ja, &db, &conf, mode);
        assert!(ret.is_ok());
        let ret = jobs.commit(&stage, mode);
        assert!(ret.is_ok());
        assert_eq!(jobs.len(), 2);
        let ret = job_trans_fallback(&jobs, &db, &unit_test1, JobKind::JobStop);
        assert_eq!(ret.len(), 0);
    }

    fn prepare_unit_multi(relation: UnitRelations) -> (Rc<UnitDb>, Rc<UnitX>, Rc<UnitX>) {
        let db = Rc::new(UnitDb::new());
        let name_test1 = String::from("test1.service");
        let unit_test1 = create_unit(&name_test1);
        let name_test2 = String::from("test2.service");
        let unit_test2 = create_unit(&name_test2);
        db.units_insert(name_test1.clone(), Rc::clone(&unit_test1));
        db.units_insert(name_test2.clone(), Rc::clone(&unit_test2));
        let u1 = Rc::clone(&unit_test1);
        let u2 = Rc::clone(&unit_test2);
        db.dep_insert(u1, relation, u2, true, 0).unwrap();
        (db, unit_test1, unit_test2)
    }

    fn prepare_unit_single() -> (Rc<UnitDb>, Rc<UnitX>) {
        let db = Rc::new(UnitDb::new());
        let name_test1 = String::from("test1.service");
        let unit_test1 = create_unit(&name_test1);
        db.units_insert(name_test1.clone(), Rc::clone(&unit_test1));
        (db, unit_test1)
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
