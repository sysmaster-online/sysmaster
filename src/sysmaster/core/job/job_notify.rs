use super::job_entry::JobConf;
use super::job_rentry::JobKind;
use crate::core::unit::UnitRelationAtom;
use crate::core::unit::UnitDb;
use crate::core::unit::UnitX;
use crate::core::unit::JobMode;
use std::rc::Rc;

pub(super) fn job_notify_result(
    db: &UnitDb,
    unit: Rc<UnitX>,
    atom: UnitRelationAtom,
    mode: JobMode,
) -> (Vec<JobConf>, JobMode) {
    let configs = match atom {
        UnitRelationAtom::UnitAtomOnSuccess | UnitRelationAtom::UnitAtomOnFailure => {
            notify_result_start(db, unit, atom)
        }
        _ => unreachable!("kind of notify is not supported."),
    };

    (configs, mode)
}

pub(super) fn job_notify_event(
    db: &UnitDb,
    config: &JobConf,
    mode_option: Option<JobMode>,
) -> Vec<(JobConf, JobMode)> {
    match config.get_kind() {
        JobKind::Start => notify_event_start(db, config, mode_option),
        JobKind::Stop => notify_event_stop(db, config, mode_option),
        JobKind::Reload => notify_event_reload(db, config, mode_option),
        _ => unreachable!("kind of notify is not supported."),
    }
}

fn notify_result_start(db: &UnitDb, unit: Rc<UnitX>, atom: UnitRelationAtom) -> Vec<JobConf> {
    assert!(
        atom == UnitRelationAtom::UnitAtomOnSuccess || atom == UnitRelationAtom::UnitAtomOnFailure
    );
    let mut configs = Vec::new();
    for other in db.dep_gets_atom(&unit, atom).iter() {
        configs.push(JobConf::new(other, JobKind::Start));
    }
    configs
}

fn notify_event_start(
    db: &UnitDb,
    config: &JobConf,
    mode_option: Option<JobMode>,
) -> Vec<(JobConf, JobMode)> {
    let unit = config.get_unit();
    let mut targets = Vec::new();

    let mode = mode_option.unwrap_or(JobMode::Replace);
    for other in db
        .dep_gets_atom(unit, UnitRelationAtom::UnitAtomRetroActiveStartReplace)
        .iter()
    {
        if !db.dep_is_dep_atom_with(unit, UnitRelationAtom::UnitAtomAfter, other) {
            targets.push((JobConf::new(other, JobKind::Start), mode));
        }
    }

    let mode = mode_option.unwrap_or(JobMode::Fail);
    for other in db
        .dep_gets_atom(unit, UnitRelationAtom::UnitAtomRetroActiveStartFail)
        .iter()
    {
        if !db.dep_is_dep_atom_with(unit, UnitRelationAtom::UnitAtomAfter, other) {
            targets.push((JobConf::new(other, JobKind::Start), mode));
        }
    }

    let mode = mode_option.unwrap_or(JobMode::Replace);
    for other in db
        .dep_gets_atom(unit, UnitRelationAtom::UnitAtomRetroActiveStopOnStart)
        .iter()
    {
        targets.push((JobConf::new(other, JobKind::Stop), mode));
    }

    targets
}

fn notify_event_stop(
    db: &UnitDb,
    config: &JobConf,
    mode_option: Option<JobMode>,
) -> Vec<(JobConf, JobMode)> {
    let unit = config.get_unit();
    let mut targets = Vec::new();

    let mode = mode_option.unwrap_or(JobMode::Replace);
    for other in db
        .dep_gets_atom(unit, UnitRelationAtom::UnitAtomRetroActiveStopOnStop)
        .iter()
    {
        targets.push((JobConf::new(other, JobKind::Stop), mode));
    }

    targets
}

fn notify_event_reload(
    db: &UnitDb,
    config: &JobConf,
    mode_option: Option<JobMode>,
) -> Vec<(JobConf, JobMode)> {
    let unit = config.get_unit();
    let mut targets = Vec::new();

    let mode = mode_option.unwrap_or(JobMode::Fail);
    for other in db
        .dep_gets_atom(unit, UnitRelationAtom::UnitAtomPropagatesReloadTo)
        .iter()
    {
        targets.push((JobConf::new(other, JobKind::TryReload), mode));
    }

    targets
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::manager::rentry::RELI_HISTORY_MAX_DBS;
    use crate::core::unit::DataManager;
    use crate::core::unit::test_utils;

    use crate::core::unit::UnitDb;
    use crate::core::unit::UnitX;
    use crate::core::unit::UnitRe;

    use crate::core::reliability::Reliability;
    use libutils::logger;

    #[test]
    fn jn_api() {
        let (_, db, unit_test1) = prepare_unit_single();

        // result
        let atom = UnitRelationAtom::UnitAtomOnSuccess;
        let mode = JobMode::Replace;
        let (confs, m) = job_notify_result(&db, Rc::clone(&unit_test1), atom, mode);
        assert_eq!(mode, m);
        assert_eq!(confs.len(), 0);

        // event: start
        let conf = JobConf::new(&unit_test1, JobKind::Start);
        let ret = job_notify_event(&db, &conf, None);
        assert_eq!(ret.len(), 0);

        // event: stop
        let conf = JobConf::new(&unit_test1, JobKind::Stop);
        let ret = job_notify_event(&db, &conf, None);
        assert_eq!(ret.len(), 0);

        // event: reload
        let conf = JobConf::new(&unit_test1, JobKind::Reload);
        let ret = job_notify_event(&db, &conf, None);
        assert_eq!(ret.len(), 0);
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
        let unitx = test_utils::create_unit_for_test_pub(dmr, relir, rentryr, name);
        unitx
    }
}
