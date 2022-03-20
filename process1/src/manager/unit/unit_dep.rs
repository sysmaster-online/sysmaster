use super::unit_entry::UnitX;
use super::unit_relation::{self};
use super::unit_relation_atom::{self, UnitRelationAtom};
use super::UnitErrno;
use crate::manager::data::UnitRelations;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug)]
pub(super) struct UnitDep {
    data: RefCell<UnitDepData>,
}

impl UnitDep {
    pub(super) fn new() -> UnitDep {
        UnitDep {
            data: RefCell::new(UnitDepData::new()),
        }
    }

    pub(super) fn insert(
        &self,
        source: Rc<UnitX>,
        relation: UnitRelations,
        dest: Rc<UnitX>,
        reference: bool,
        source_mask: u16,
    ) -> Result<(), UnitErrno> {
        source.dep_check(relation, &dest)?;
        self.data
            .borrow_mut()
            .insert(source, relation, dest, reference, source_mask);
        Ok(())
    }

    pub(super) fn remove(&self, source: &UnitX, relation: UnitRelations, dest: &UnitX) {
        self.data.borrow_mut().remove(source, relation, dest)
    }

    pub(super) fn remove_unit(&self, source: &UnitX) {
        self.data.borrow_mut().remove_unit(source)
    }

    pub(super) fn gets(&self, source: &UnitX, relation: UnitRelations) -> Vec<Rc<UnitX>> {
        self.data.borrow().gets(source, relation)
    }

    pub(super) fn gets_atom(&self, source: &UnitX, atom: UnitRelationAtom) -> Vec<Rc<UnitX>> {
        let mut dests = Vec::new();
        for relation in unit_relation_atom::unit_relation_from_unique_atom(atom).iter() {
            dests.append(&mut self.gets(source, *relation));
        }
        dests
    }

    pub(super) fn is_dep_with(
        &self,
        source: &UnitX,
        relation: UnitRelations,
        dest: &UnitX,
    ) -> bool {
        self.data.borrow().is_dep_with(source, relation, dest)
    }

    pub(super) fn is_dep_atom_with(
        &self,
        source: &UnitX,
        atom: UnitRelationAtom,
        dest: &UnitX,
    ) -> bool {
        for relation in unit_relation_atom::unit_relation_from_unique_atom(atom).iter() {
            if self.is_dep_with(source, *relation, dest) {
                // something hits
                return true;
            }
        }
        false
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct UnitDepMask {
    source: u16,
    dest: u16,
}

#[derive(Debug)]
struct UnitDepData {
    // key: unit-source + UnitRelations, value: (unit-destination : mask)-list
    t: HashMap<Rc<UnitX>, HashMap<UnitRelations, HashMap<Rc<UnitX>, UnitDepMask>>>,
}

// the declaration "pub(self)" is for identification only.
impl UnitDepData {
    pub(self) fn new() -> UnitDepData {
        UnitDepData { t: HashMap::new() }
    }

    pub(self) fn insert(
        &mut self,
        source: Rc<UnitX>,
        relation: UnitRelations,
        dest: Rc<UnitX>,
        reference: bool,
        source_mask: u16,
    ) {
        // check input
        if source.as_ref() == dest.as_ref() {
            // ptr_eq?
            // Err(UnitErrno::UnitErrInternal)
            return;
        }

        let mask = UnitDepMask {
            source: source_mask,
            dest: 0,
        };
        let mask_inverse = UnitDepMask {
            source: 0,
            dest: source_mask,
        };
        let relation_inverse = unit_relation::unit_relation_to_inverse(relation);

        // insert in two-directions way
        self.insert_one_way(Rc::clone(&source), relation, Rc::clone(&dest), mask);
        self.insert_one_way(
            Rc::clone(&dest),
            relation_inverse,
            Rc::clone(&source),
            mask_inverse,
        );

        // process reference in two-directions way
        if reference {
            let ref_relation = UnitRelations::UnitReferences;
            let ref_relation_inverse = unit_relation::unit_relation_to_inverse(ref_relation);
            self.insert_one_way(Rc::clone(&source), ref_relation, Rc::clone(&dest), mask);
            self.insert_one_way(
                Rc::clone(&dest),
                ref_relation_inverse,
                Rc::clone(&source),
                mask_inverse,
            );
        }
    }

    pub(self) fn remove(&mut self, source: &UnitX, relation: UnitRelations, dest: &UnitX) {
        // remove in two-directions way
        let relation_inverse = unit_relation::unit_relation_to_inverse(relation);
        self.remove_one_way(source, relation, dest);
        self.remove_one_way(dest, relation_inverse, source);
    }

    pub(self) fn remove_unit(&mut self, source: &UnitX) {
        if let Some(sv) = self.t.get(source) {
            let mut removes = Vec::new();
            for (relation, dv) in sv.iter() {
                for (dest, _) in dv.iter() {
                    removes.push((*relation, Rc::clone(dest)));
                }
            }

            for (relation, dest) in removes.iter() {
                self.remove(source, *relation, dest);
            }
        }
    }

    pub(self) fn gets(&self, source: &UnitX, relation: UnitRelations) -> Vec<Rc<UnitX>> {
        let mut dests = Vec::new();

        if let Some(sv) = self.t.get(source) {
            if let Some(dv) = sv.get(&relation) {
                dests.append(
                    &mut dv
                        .iter()
                        .map(|(destr, _)| Rc::clone(destr))
                        .collect::<Vec<_>>(),
                );
            }
        }

        dests
    }

    pub(self) fn is_dep_with(&self, source: &UnitX, relation: UnitRelations, dest: &UnitX) -> bool {
        if let Some(sv) = self.t.get(source) {
            if let Some(dv) = sv.get(&relation) {
                return dv.contains_key(dest);
            }
        }

        false
    }

    fn insert_one_way(
        &mut self,
        source: Rc<UnitX>,
        relation: UnitRelations,
        dest: Rc<UnitX>,
        mask: UnitDepMask,
    ) {
        self.get_mut_dv_pad(source, relation).insert(dest, mask);
    }

    fn remove_one_way(&mut self, source: &UnitX, relation: UnitRelations, dest: &UnitX) {
        if let Some(sv) = self.t.get_mut(source) {
            let mut dv_dummy = HashMap::new(); // nothing inside
            sv.get_mut(&relation).unwrap_or(&mut dv_dummy).remove(dest); // remove dest
            if !sv.is_empty() {
                self.t.remove(source); // remove unit-entry to release the key 'Rc<Unit>'
            }
        }
    }

    fn get_mut_sv_pad(
        &mut self,
        source: Rc<UnitX>,
    ) -> &mut HashMap<UnitRelations, HashMap<Rc<UnitX>, UnitDepMask>> {
        // verify existance
        if let None = self.t.get(&source) {
            // nothing exists, pad it.
            self.t.insert(Rc::clone(&source), HashMap::new());
        }

        // return the one that must exist
        self.t
            .get_mut(&source)
            .expect("something inserted is not found.")
    }

    fn get_mut_dv_pad(
        &mut self,
        source: Rc<UnitX>,
        relation: UnitRelations,
    ) -> &mut HashMap<Rc<UnitX>, UnitDepMask> {
        // verify existance
        let sv = self.get_mut_sv_pad(source);
        if let None = sv.get(&relation) {
            // nothing exists, pad it.
            sv.insert(relation, HashMap::new());
        }

        // return the one that must exist
        sv.get_mut(&relation)
            .expect("something inserted is not found.")
    }
}
