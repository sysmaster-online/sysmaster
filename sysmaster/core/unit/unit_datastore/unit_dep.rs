use super::unit_sets::UnitSets;
use super::table::{TableOp, TableSubscribe};
use super::super::unit_base;
use super::super::{UnitRelationAtom};
use super::UnitX;
use super::UnitRe;
use super::super::UnitErrno;
use super::UnitRelations;
use super::ReStation;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub(super) struct UnitDep {
    sub_name: String, // key for table-subscriber: UnitSets
    sub: Rc<UnitDepSub>,
}

impl ReStation for UnitDep {
    // no input, no compensate

    // data
    fn db_map(&self) {
        self.sub.data.borrow_mut().db_map();
    }

    // reload
    fn entry_clear(&self) {
        self.sub.data.borrow_mut().clear();
    }
}

impl UnitDep {
    pub(super) fn new(rentryr: &Rc<UnitRe>, unitsr: &Rc<UnitSets>) -> UnitDep {
        let ud = UnitDep {
            sub_name: String::from("UnitDep"),
            sub: Rc::new(UnitDepSub::new(rentryr, unitsr)),
        };
        ud.register(unitsr);
        ud
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
        self.sub
            .data
            .borrow_mut()
            .insert(source, relation, dest, reference, source_mask);
        Ok(())
    }

    #[allow(dead_code)]
    pub(super) fn remove(&self, source: &UnitX, relation: UnitRelations, dest: &UnitX) {
        self.sub.data.borrow_mut().remove(source, relation, dest)
    }

    #[allow(dead_code)]
    pub(super) fn remove_unit(&self, source: &UnitX) {
        self.sub.data.borrow_mut().remove_unit(source)
    }

    pub(super) fn gets(&self, source: &UnitX, relation: UnitRelations) -> Vec<Rc<UnitX>> {
        self.sub.data.borrow().gets(source, relation)
    }

    pub(super) fn gets_atom(&self, source: &UnitX, atom: UnitRelationAtom) -> Vec<Rc<UnitX>> {
        let mut dests = Vec::new();
        for relation in unit_base::unit_relation_from_unique_atom(atom).iter() {
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
        self.sub.data.borrow().is_dep_with(source, relation, dest)
    }

    pub(super) fn is_dep_atom_with(
        &self,
        source: &UnitX,
        atom: UnitRelationAtom,
        dest: &UnitX,
    ) -> bool {
        for relation in unit_base::unit_relation_from_unique_atom(atom).iter() {
            if self.is_dep_with(source, *relation, dest) {
                // something hits
                return true;
            }
        }
        false
    }

    fn register(&self, unitsr: &UnitSets) {
        let subscriber = Rc::clone(&self.sub);
        unitsr.register(&self.sub_name, subscriber);
    }
}

struct UnitDepSub {
    data: RefCell<UnitDepData>,
}

impl TableSubscribe<String, Rc<UnitX>> for UnitDepSub {
    fn notify(&self, op: &TableOp<String, Rc<UnitX>>) {
        match op {
            TableOp::TableInsert(_, _) => {} // do nothing
            TableOp::TableRemove(_, unit) => self.remove_unit(unit),
        }
    }
}

// the declaration "pub(self)" is for identification only.
impl UnitDepSub {
    pub(self) fn new(rentryr: &Rc<UnitRe>, unitsr: &Rc<UnitSets>) -> UnitDepSub {
        UnitDepSub {
            data: RefCell::new(UnitDepData::new(rentryr, unitsr)),
        }
    }

    fn remove_unit(&self, unit: &UnitX) {
        self.data.borrow_mut().remove_unit(unit)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct UnitDepMask {
    source: u16,
    dest: u16,
}

impl UnitDepMask {
    fn new(s_mask: u16, d_mask: u16) -> UnitDepMask {
        UnitDepMask {
            source: s_mask,
            dest: d_mask,
        }
    }
}

#[allow(clippy::type_complexity)]
struct UnitDepData {
    // associated objects
    units: Rc<UnitSets>,
    rentry: Rc<UnitRe>,

    // owned objects
    // key: unit-source + UnitRelations, value: (unit-destination : mask)-list
    t: HashMap<Rc<UnitX>, HashMap<UnitRelations, HashMap<Rc<UnitX>, UnitDepMask>>>,
}

// the declaration "pub(self)" is for identification only.
impl UnitDepData {
    pub(self) fn new(rentryr: &Rc<UnitRe>, unitsr: &Rc<UnitSets>) -> UnitDepData {
        UnitDepData {
            units: Rc::clone(unitsr),
            rentry: Rc::clone(rentryr),
            t: HashMap::new(),
        }
    }

    pub(self) fn clear(&mut self) {
        self.t.clear();
    }

    pub(self) fn db_map(&mut self) {
        for source in self.rentry.dep_keys().iter() {
            let src = self.units.get(source).unwrap();
            let mask = UnitDepMask::new(0, 0);
            for (relation, dest) in self.rentry.dep_get(source).iter() {
                let r_src = *relation;
                let r_dst = unit_base::unit_relation_to_inverse(r_src);
                let dst = self.units.get(dest).unwrap();
                self.insert_one_way(Rc::clone(&src), r_src, Rc::clone(&dst), mask);
                self.insert_one_way(Rc::clone(&dst), r_dst, Rc::clone(&src), mask);
            }
        }
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

        let mask = UnitDepMask::new(source_mask, 0);
        let mask_inverse = UnitDepMask::new(0, source_mask);
        let relation_inverse = unit_base::unit_relation_to_inverse(relation);

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
            let ref_relation_inverse = unit_base::unit_relation_to_inverse(ref_relation);
            self.insert_one_way(Rc::clone(&source), ref_relation, Rc::clone(&dest), mask);
            self.insert_one_way(
                Rc::clone(&dest),
                ref_relation_inverse,
                Rc::clone(&source),
                mask_inverse,
            );
        }

        // update rentry
        self.db_update(&source);
        self.db_update(&dest);
    }

    pub(self) fn remove(&mut self, source: &UnitX, relation: UnitRelations, dest: &UnitX) {
        // remove in two-directions way
        let relation_inverse = unit_base::unit_relation_to_inverse(relation);
        self.remove_one_way(source, relation, dest);
        self.remove_one_way(dest, relation_inverse, source);

        // update rentry
        self.db_update(source);
        self.db_update(dest);
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
        // verify existence
        if self.t.get(&source).is_none() {
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
        // verify existence
        let sv = self.get_mut_sv_pad(source);
        if sv.get(&relation).is_none() {
            // nothing exists, pad it.
            sv.insert(relation, HashMap::new());
        }

        // return the one that must exist
        sv.get_mut(&relation)
            .expect("something inserted is not found.")
    }

    fn db_update(&self, unit: &UnitX) {
        let mut deps = Vec::new();
        let sv_empty = HashMap::new();
        let sv = self.t.get(unit).unwrap_or(&sv_empty);
        for (relation, dv) in sv.iter() {
            deps.append(
                &mut dv
                    .iter()
                    .map(|(destr, _)| (*relation, destr.id().clone()))
                    .collect::<Vec<_>>(),
            );
        }
        self.rentry.dep_insert(unit.id(), &deps);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::manager::rentry::RELI_HISTORY_MAX_DBS;
    use crate::core::unit::data::DataManager;
    use crate::core::unit::test::test_utils;
    use libsysmaster::reliability::Reliability;
    use libutils::logger;

    #[test]
    fn dep_insert() {
        let dm = Rc::new(DataManager::new());
        let reli = Rc::new(Reliability::new(RELI_HISTORY_MAX_DBS));
        let rentry = Rc::new(UnitRe::new(&reli));
        let sets = UnitSets::new();
        let dep = UnitDep::new(&rentry, &Rc::new(sets));
        let name_test1 = String::from("test1.service");
        let unit_test1 = create_unit(&dm, &reli, &rentry, &name_test1);
        let name_test2 = String::from("test2.service");
        let unit_test2 = create_unit(&dm, &reli, &rentry, &name_test2);
        let name_test3 = String::from("test3.service");
        let unit_test3 = create_unit(&dm, &reli, &rentry, &name_test3);
        let relation = UnitRelations::UnitRequires;

        let old = dep.insert(
            Rc::clone(&unit_test1),
            relation,
            Rc::clone(&unit_test2),
            true,
            0,
        );
        assert!(old.is_ok());

        let old = dep.insert(
            Rc::clone(&unit_test1),
            relation,
            Rc::clone(&unit_test3),
            true,
            0,
        );
        assert!(old.is_ok());

        let old = dep.insert(
            Rc::clone(&unit_test2),
            relation,
            Rc::clone(&unit_test3),
            true,
            0,
        );
        assert!(old.is_ok());
    }

    #[test]
    fn dep_gets_atom() {
        let dm = Rc::new(DataManager::new());
        let reli = Rc::new(Reliability::new(RELI_HISTORY_MAX_DBS));
        let rentry = Rc::new(UnitRe::new(&reli));
        let sets = UnitSets::new();
        let dep = UnitDep::new(&rentry, &Rc::new(sets));
        let name_test1 = String::from("test1.service");
        let unit_test1 = create_unit(&dm, &reli, &rentry, &name_test1);
        let name_test2 = String::from("test2.service");
        let unit_test2 = create_unit(&dm, &reli, &rentry, &name_test2);
        let name_test3 = String::from("test3.service");
        let unit_test3 = create_unit(&dm, &reli, &rentry, &name_test3);
        let relation2 = UnitRelations::UnitRequires;
        let relation3 = UnitRelations::UnitWants;
        let atom2 = UnitRelationAtom::UnitAtomPullInStart; // + require, - want
        let atom3 = UnitRelationAtom::UnitAtomPullInStartIgnored; // - require, + want
        let atom = UnitRelationAtom::UnitAtomAddStopWhenUnneededQueue; // + require, + want

        let units = dep.gets_atom(&unit_test1, atom2);
        assert_eq!(units.len(), 0);

        dep.insert(
            Rc::clone(&unit_test1),
            relation2,
            Rc::clone(&unit_test2),
            true,
            0,
        )
        .unwrap();
        dep.insert(
            Rc::clone(&unit_test1),
            relation3,
            Rc::clone(&unit_test3),
            true,
            0,
        )
        .unwrap();

        let units = dep.gets_atom(&unit_test1, atom2);
        assert_eq!(units.len(), 1);
        assert!(contain_unit(&units, &unit_test2));
        assert!(!contain_unit(&units, &unit_test3));

        let units = dep.gets_atom(&unit_test1, atom3);
        assert_eq!(units.len(), 1);
        assert!(!contain_unit(&units, &unit_test2));
        assert!(contain_unit(&units, &unit_test3));

        let units = dep.gets_atom(&unit_test1, atom);
        assert_eq!(units.len(), 2);
        assert!(contain_unit(&units, &unit_test2));
        assert!(contain_unit(&units, &unit_test3));
    }

    #[test]
    fn dep_is_dep_atom_with() {
        let dm = Rc::new(DataManager::new());
        let reli = Rc::new(Reliability::new(RELI_HISTORY_MAX_DBS));
        let rentry = Rc::new(UnitRe::new(&reli));
        let sets = UnitSets::new();
        let dep = UnitDep::new(&rentry, &Rc::new(sets));
        let name_test1 = String::from("test1.service");
        let unit_test1 = create_unit(&dm, &reli, &rentry, &name_test1);
        let name_test2 = String::from("test2.service");
        let unit_test2 = create_unit(&dm, &reli, &rentry, &name_test2);
        let name_test3 = String::from("test3.service");
        let unit_test3 = create_unit(&dm, &reli, &rentry, &name_test3);
        let relation2 = UnitRelations::UnitRequires;
        let relation3 = UnitRelations::UnitWants;
        let atom2 = UnitRelationAtom::UnitAtomPullInStart; // + require, - want
        let atom3 = UnitRelationAtom::UnitAtomPullInStartIgnored; // - require, + want
        let atom = UnitRelationAtom::UnitAtomAddStopWhenUnneededQueue; // + require, + want

        let value = dep.is_dep_atom_with(&unit_test1, atom2, &unit_test2);
        assert!(!value);
        let value = dep.is_dep_atom_with(&unit_test1, atom3, &unit_test2);
        assert!(!value);
        let value = dep.is_dep_atom_with(&unit_test1, atom, &unit_test2);
        assert!(!value);
        let value = dep.is_dep_atom_with(&unit_test1, atom2, &unit_test3);
        assert!(!value);
        let value = dep.is_dep_atom_with(&unit_test1, atom3, &unit_test3);
        assert!(!value);
        let value = dep.is_dep_atom_with(&unit_test1, atom, &unit_test3);
        assert!(!value);

        dep.insert(
            Rc::clone(&unit_test1),
            relation2,
            Rc::clone(&unit_test2),
            true,
            0,
        )
        .unwrap();
        let value = dep.is_dep_atom_with(&unit_test1, atom2, &unit_test2);
        assert!(value);
        let value = dep.is_dep_atom_with(&unit_test1, atom3, &unit_test2);
        assert!(!value);
        let value = dep.is_dep_atom_with(&unit_test1, atom, &unit_test2);
        assert!(value);
        let value = dep.is_dep_atom_with(&unit_test1, atom2, &unit_test3);
        assert!(!value);
        let value = dep.is_dep_atom_with(&unit_test1, atom3, &unit_test3);
        assert!(!value);
        let value = dep.is_dep_atom_with(&unit_test1, atom, &unit_test3);
        assert!(!value);

        dep.insert(
            Rc::clone(&unit_test1),
            relation3,
            Rc::clone(&unit_test3),
            true,
            0,
        )
        .unwrap();
        let value = dep.is_dep_atom_with(&unit_test1, atom2, &unit_test2);
        assert!(value);
        let value = dep.is_dep_atom_with(&unit_test1, atom3, &unit_test2);
        assert!(!value);
        let value = dep.is_dep_atom_with(&unit_test1, atom, &unit_test2);
        assert!(value);
        let value = dep.is_dep_atom_with(&unit_test1, atom2, &unit_test3);
        assert!(!value);
        let value = dep.is_dep_atom_with(&unit_test1, atom3, &unit_test3);
        assert!(value);
        let value = dep.is_dep_atom_with(&unit_test1, atom, &unit_test3);
        assert!(value);
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

    fn contain_unit(units: &[Rc<UnitX>], unit: &Rc<UnitX>) -> bool {
        for u in units.iter() {
            if Rc::ptr_eq(u, unit) {
                return true;
            }
        }

        false
    }
}
