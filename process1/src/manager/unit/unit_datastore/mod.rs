use crate::manager::data::UnitRelations;
use crate::manager::unit::unit_entry::UnitX;
use crate::manager::unit::unit_relation_atom::UnitRelationAtom;
use crate::manager::unit::UnitErrno;
use nix::unistd::Pid;
use std::error::Error;
use std::rc::Rc;
use unit_child::UnitChild;
use unit_dep::UnitDep;
use unit_sets::UnitSets;

//#[derive(Debug)]
pub struct UnitDb {
    units: Rc<UnitSets>,
    dep: Rc<UnitDep>,
    child: Rc<UnitChild>,
}

impl UnitDb {
    pub(super) fn new() -> UnitDb {
        let _units = Rc::new(UnitSets::new());
        UnitDb {
            units: Rc::clone(&_units),
            dep: Rc::new(UnitDep::new()),
            child: Rc::new(UnitChild::new(Rc::clone(&_units))),
        }
    }

    /*pub fn get_instance() -> Rc<RefCell<UnitStorage<K,V>>> {
        static mut PLUGIN: Option<Rc<RefCell<UnitStorage<K,V>>>> = None;
        unsafe {
            PLUGIN
                .get_or_insert_with(|| {
                    let mut unitStorage: UnitStorage<K, V> = Self::new();
                    Rc::new(RefCell::new(unitStorage))
                })
                .clone()
        }
    }*/

    pub(super) fn units_insert(&self, name: String, unit: Rc<UnitX>) -> Option<Rc<UnitX>> {
        self.units.insert(name, unit)
    }

    pub(super) fn units_get(&self, name: &str) -> Option<Rc<UnitX>> {
        self.units.get(name)
    }

    pub(super) fn units_get_all(&self) -> Vec<Rc<UnitX>> {
        self.units.get_all()
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
}

mod unit_child;
mod unit_dep;
mod unit_sets;
