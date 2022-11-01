use super::unit_base::UnitRelationAtom;
use crate::manager::table::TableSubscribe;
use crate::manager::unit::unit_entry::UnitX;
use crate::manager::unit::unit_rentry::{UnitRe, UnitRelations, UnitType};
use crate::manager::unit::UnitErrno;
use crate::reliability::ReStation;
use libcgroup;
use libutils::Result;
use nix::unistd::Pid;
use nix::NixPath;
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

impl ReStation for UnitDb {
    // no input, no compensate

    // data: special map

    // reload: entry-only
    fn entry_clear(&self) {
        self.child.entry_clear();
        self.dep.entry_clear();
        self.units.entry_clear();
    }
}

impl Drop for UnitDb {
    fn drop(&mut self) {
        log::debug!("UnitDb drop, clear.");
        // repeating protection
        self.clear();
    }
}

impl UnitDb {
    pub(super) fn new(rentryr: &Rc<UnitRe>) -> UnitDb {
        let _units = Rc::new(UnitSets::new());
        UnitDb {
            units: Rc::clone(&_units),
            dep: UnitDep::new(rentryr, &_units),
            child: UnitChild::new(rentryr, &_units),
        }
    }

    pub(super) fn db_map_excl_units(&self) {
        // dep
        self.dep.db_map();

        // child
        self.child.db_map();
    }

    pub(super) fn units_insert(&self, name: String, unit: Rc<UnitX>) -> Option<Rc<UnitX>> {
        self.units.insert(name, unit)
    }

    pub(super) fn unit_remove(&self, name: &str) {
        self.units.remove(name);
    }

    pub(super) fn units_get(&self, name: &str) -> Option<Rc<UnitX>> {
        self.units.get(name)
    }

    pub(super) fn units_get_all(&self, unit_type: Option<UnitType>) -> Vec<Rc<UnitX>> {
        let mut units = self.units.get_all();
        units.retain(|ur| {
            if let Some(ut) = unit_type {
                ur.unit_type() == ut
            } else {
                true
            }
        });
        units
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

    pub(super) fn dep_gets(&self, name: &str, relation: UnitRelations) -> Vec<Rc<UnitX>> {
        let unitx = self.units_get(name);

        if unitx.is_none() {
            return Vec::new();
        }

        self.dep.gets(&unitx.unwrap(), relation)
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

    pub(super) fn child_add_watch_pid(&self, id: &str, pid: Pid) {
        self.child.add_watch_pid(id, pid)
    }

    pub(super) fn child_unwatch_pid(&self, id: &str, pid: Pid) {
        self.child.unwatch_pid(id, pid)
    }

    pub(super) fn child_watch_all_pids(&self, id: &str) {
        let u = self.units_get(id).unwrap();
        let cg_path = u.cg_path();
        if cg_path.is_empty() {
            return;
        }

        let pids = libcgroup::cg_get_pids(&cg_path);
        for pid in pids {
            log::debug!("watch all cgroup pids: {}", pid);
            self.child.add_watch_pid(id, pid)
        }
    }

    pub(super) fn get_unit_by_pid(&self, pid: Pid) -> Option<Rc<UnitX>> {
        self.child.get_unit_by_pid(pid)
    }

    // repeating protection
    pub(super) fn clear(&self) {
        self.child.entry_clear();
        self.dep.entry_clear();
        self.units.clear();
    }
}

// dependency: unit_sets -> {unit_dep | unit_child}
mod unit_child;
mod unit_dep;
mod unit_sets;
