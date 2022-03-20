#![warn(unused_imports)]
use super::job_entry::{JobConf, JobKind};
use crate::manager::data::JobMode;
use crate::manager::unit::unit_dep::UnitDep;
use crate::manager::unit::unit_entry::UnitX;
use crate::manager::unit::unit_relation_atom::UnitRelationAtom;
use std::rc::Rc;

pub(super) fn job_notify_result(
    dep: &UnitDep,
    unit: Rc<UnitX>,
    atom: UnitRelationAtom,
    mode: JobMode,
) -> (Vec<JobConf>, JobMode) {
    let configs = match atom {
        UnitRelationAtom::UnitAtomOnSuccess | UnitRelationAtom::UnitAtomOnFailure => {
            notify_result_start(dep, unit, atom)
        }
        _ => unreachable!("kind of notify is not supported."),
    };

    (configs, mode)
}

pub(super) fn job_notify_event(
    dep: &UnitDep,
    config: &JobConf,
    mode_option: Option<JobMode>,
) -> Vec<(JobConf, JobMode)> {
    match config.get_kind() {
        JobKind::JobStart => notify_event_start(dep, config, mode_option),
        JobKind::JobStop => notify_event_stop(dep, config, mode_option),
        JobKind::JobReload => notify_event_reload(dep, config, mode_option),
        _ => unreachable!("kind of notify is not supported."),
    }
}

fn notify_result_start(dep: &UnitDep, unit: Rc<UnitX>, atom: UnitRelationAtom) -> Vec<JobConf> {
    assert!(
        atom == UnitRelationAtom::UnitAtomOnSuccess || atom == UnitRelationAtom::UnitAtomOnFailure
    );
    let mut configs = Vec::new();
    for other in dep.gets_atom(&unit, atom).iter() {
        configs.push(JobConf::new(Rc::clone(other), JobKind::JobStart));
    }
    configs
}

fn notify_event_start(
    dep: &UnitDep,
    config: &JobConf,
    mode_option: Option<JobMode>,
) -> Vec<(JobConf, JobMode)> {
    let unit = config.get_unit();
    let mut targets = Vec::new();

    let mode = mode_option.unwrap_or(JobMode::JobReplace);
    for other in dep
        .gets_atom(unit, UnitRelationAtom::UnitAtomRetroActiveStartReplace)
        .iter()
    {
        if !dep.is_dep_atom_with(unit, UnitRelationAtom::UnitAtomAfter, other) {
            targets.push((JobConf::new(Rc::clone(other), JobKind::JobStart), mode));
        }
    }

    let mode = mode_option.unwrap_or(JobMode::JobFail);
    for other in dep
        .gets_atom(unit, UnitRelationAtom::UnitAtomRetroActiveStartFail)
        .iter()
    {
        if !dep.is_dep_atom_with(unit, UnitRelationAtom::UnitAtomAfter, other) {
            targets.push((JobConf::new(Rc::clone(other), JobKind::JobStart), mode));
        }
    }

    let mode = mode_option.unwrap_or(JobMode::JobReplace);
    for other in dep
        .gets_atom(unit, UnitRelationAtom::UnitAtomRetroActiveStopOnStart)
        .iter()
    {
        targets.push((JobConf::new(Rc::clone(other), JobKind::JobStop), mode));
    }

    targets
}

fn notify_event_stop(
    dep: &UnitDep,
    config: &JobConf,
    mode_option: Option<JobMode>,
) -> Vec<(JobConf, JobMode)> {
    let unit = config.get_unit();
    let mut targets = Vec::new();

    let mode = mode_option.unwrap_or(JobMode::JobReplace);
    for other in dep
        .gets_atom(unit, UnitRelationAtom::UnitAtomRetroActiveStopOnStop)
        .iter()
    {
        targets.push((JobConf::new(Rc::clone(other), JobKind::JobStop), mode));
    }

    targets
}

fn notify_event_reload(
    dep: &UnitDep,
    config: &JobConf,
    mode_option: Option<JobMode>,
) -> Vec<(JobConf, JobMode)> {
    let unit = config.get_unit();
    let mut targets = Vec::new();

    let mode = mode_option.unwrap_or(JobMode::JobFail);
    for other in dep
        .gets_atom(unit, UnitRelationAtom::UnitAtomPropagatesReloadTo)
        .iter()
    {
        targets.push((JobConf::new(Rc::clone(other), JobKind::JobTryReload), mode));
    }

    targets
}
