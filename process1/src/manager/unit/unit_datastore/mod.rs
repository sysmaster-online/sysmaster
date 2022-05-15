use super::unit_base::UnitRelationAtom;
use crate::manager::data::UnitRelations;
use crate::manager::table::TableSubscribe;
use crate::manager::unit::unit_entry::UnitX;
use crate::manager::unit::UnitErrno;
use cgroup;
use nix::unistd::Pid;
use std::error::Error;
use std::rc::Rc;
use unit_child::UnitChild;
use unit_dep::UnitDep;
use unit_sets::UnitSets;

//#[derive(Debug)]
pub(super) struct UnitDb {
    units: Rc<UnitSets>,
    dep: UnitDep,
    child: UnitChild,
}

impl UnitDb {
    pub(super) fn new() -> UnitDb {
        let _units = Rc::new(UnitSets::new());
        UnitDb {
            units: Rc::clone(&_units),
            dep: UnitDep::new(&_units),
            child: UnitChild::new(&_units),
        }
    }

    pub(super) fn units_insert(&self, name: String, unit: Rc<UnitX>) -> Option<Rc<UnitX>> {
        self.units.insert(name, unit)
    }

    pub(super) fn units_get(&self, name: &str) -> Option<Rc<UnitX>> {
        self.units.get(name)
    }

    pub(super) fn units_get_all(&self) -> Vec<Rc<UnitX>> {
        self.units.get_all()
    }

    pub(super) fn units_register(
        &self,
        sub_name: &str,
        subscriber: Rc<dyn TableSubscribe<String, Rc<UnitX>>>,
    ) -> Option<Rc<dyn TableSubscribe<String, Rc<UnitX>>>> {
        self.units.register(sub_name, subscriber)
    }

    pub(super) fn dep_insert(
        &self,
        source: Rc<UnitX>,
        relation: UnitRelations,
        dest: Rc<UnitX>,
        reference: bool,
        source_mask: u16,
    ) -> Result<(), UnitErrno> {
        self.dep
            .insert(source, relation, dest, reference, source_mask)
    }

    pub(super) fn dep_gets_atom(&self, source: &UnitX, atom: UnitRelationAtom) -> Vec<Rc<UnitX>> {
        self.dep.gets_atom(source, atom)
    }

    pub(super) fn dep_is_dep_atom_with(
        &self,
        source: &UnitX,
        atom: UnitRelationAtom,
        dest: &UnitX,
    ) -> bool {
        self.dep.is_dep_atom_with(source, atom, dest)
    }

    pub(super) fn child_dispatch_sigchld(&self) -> Result<(), Box<dyn Error>> {
        self.child.dispatch_sigchld()
    }

    pub(super) fn child_add_watch_pid(&self, pid: Pid, id: &str) {
        self.child.add_watch_pid(pid, id)
    }

    pub(super) fn child_unwatch_pid(&self, pid: Pid) {
        self.child.unwatch_pid(pid)
    }

    pub(super) fn child_watch_all_pids(&self, id: &str) {
        let u = self.units_get(id).unwrap();
        let cg_path = u.cg_path();
        let pids = cgroup::cg_get_pids(&cg_path);

        for pid in pids {
            log::debug!("watch all cgroup pids: {}", pid);
            self.child.add_watch_pid(pid, id)
        }
    }
}

// dependency: unit_sets -> {unit_dep | unit_child}
mod unit_child;
mod unit_dep;
mod unit_sets;
