#![warn(unused_imports)]
use super::job_entry::{JobConf, JobKind};
use crate::manager::unit::unit_base::{JobMode, UnitRelationAtom};
use crate::manager::unit::unit_datastore::UnitDb;
use crate::manager::unit::unit_entry::UnitX;
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
        JobKind::JobStart => notify_event_start(db, config, mode_option),
        JobKind::JobStop => notify_event_stop(db, config, mode_option),
        JobKind::JobReload => notify_event_reload(db, config, mode_option),
        _ => unreachable!("kind of notify is not supported."),
    }
}

fn notify_result_start(db: &UnitDb, unit: Rc<UnitX>, atom: UnitRelationAtom) -> Vec<JobConf> {
    assert!(
        atom == UnitRelationAtom::UnitAtomOnSuccess || atom == UnitRelationAtom::UnitAtomOnFailure
    );
    let mut configs = Vec::new();
    for other in db.dep_gets_atom(&unit, atom).iter() {
        configs.push(JobConf::new(Rc::clone(other), JobKind::JobStart));
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

    let mode = mode_option.unwrap_or(JobMode::JobReplace);
    for other in db
        .dep_gets_atom(unit, UnitRelationAtom::UnitAtomRetroActiveStartReplace)
        .iter()
    {
        if !db.dep_is_dep_atom_with(unit, UnitRelationAtom::UnitAtomAfter, other) {
            targets.push((JobConf::new(Rc::clone(other), JobKind::JobStart), mode));
        }
    }

    let mode = mode_option.unwrap_or(JobMode::JobFail);
    for other in db
        .dep_gets_atom(unit, UnitRelationAtom::UnitAtomRetroActiveStartFail)
        .iter()
    {
        if !db.dep_is_dep_atom_with(unit, UnitRelationAtom::UnitAtomAfter, other) {
            targets.push((JobConf::new(Rc::clone(other), JobKind::JobStart), mode));
        }
    }

    let mode = mode_option.unwrap_or(JobMode::JobReplace);
    for other in db
        .dep_gets_atom(unit, UnitRelationAtom::UnitAtomRetroActiveStopOnStart)
        .iter()
    {
        targets.push((JobConf::new(Rc::clone(other), JobKind::JobStop), mode));
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

    let mode = mode_option.unwrap_or(JobMode::JobReplace);
    for other in db
        .dep_gets_atom(unit, UnitRelationAtom::UnitAtomRetroActiveStopOnStop)
        .iter()
    {
        targets.push((JobConf::new(Rc::clone(other), JobKind::JobStop), mode));
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

    let mode = mode_option.unwrap_or(JobMode::JobFail);
    for other in db
        .dep_gets_atom(unit, UnitRelationAtom::UnitAtomPropagatesReloadTo)
        .iter()
    {
        targets.push((JobConf::new(Rc::clone(other), JobKind::JobTryReload), mode));
    }

    targets
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manager::data::DataManager;
    use crate::manager::unit::uload_util::UnitFile;
    use crate::manager::unit::unit_base::UnitType;
    use crate::plugin::Plugin;
    use utils::logger;

    #[test]
    fn jn_api() {
        let db = Rc::new(UnitDb::new());
        let name_test1 = String::from("test1.service");
        let unit_test1 = create_unit(&name_test1);
        db.units_insert(name_test1.clone(), Rc::clone(&unit_test1));

        // result
        let atom = UnitRelationAtom::UnitAtomOnSuccess;
        let mode = JobMode::JobReplace;
        let (confs, m) = job_notify_result(&db, Rc::clone(&unit_test1), atom, mode);
        assert_eq!(mode, m);
        assert_eq!(confs.len(), 0);

        // event: start
        let conf = JobConf::new(Rc::clone(&unit_test1), JobKind::JobStart);
        let ret = job_notify_event(&db, &conf, None);
        assert_eq!(ret.len(), 0);

        // event: stop
        let conf = JobConf::new(Rc::clone(&unit_test1), JobKind::JobStop);
        let ret = job_notify_event(&db, &conf, None);
        assert_eq!(ret.len(), 0);

        // event: reload
        let conf = JobConf::new(Rc::clone(&unit_test1), JobKind::JobReload);
        let ret = job_notify_event(&db, &conf, None);
        assert_eq!(ret.len(), 0);
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
