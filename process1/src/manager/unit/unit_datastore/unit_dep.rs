use crate::manager::data::UnitRelations;
use crate::manager::unit::unit_entry::UnitX;
use crate::manager::unit::unit_relation::{self};
use crate::manager::unit::unit_relation_atom::{self, UnitRelationAtom};
use crate::manager::unit::UnitErrno;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manager::data::{DataManager, UnitType};
    use crate::manager::unit::unit_file::UnitFile;
    use crate::manager::unit::unit_parser_mgr::UnitParserMgr;
    use crate::plugin::Plugin;
    use std::path::PathBuf;
    use utils::logger;

    #[test]
    fn dep_insert() {
        let dep = UnitDep::new();
        let name_test1 = String::from("test1.service");
        let unit_test1 = create_unit(&name_test1);
        let name_test2 = String::from("test2.service");
        let unit_test2 = create_unit(&name_test2);
        let name_test3 = String::from("test3.service");
        let unit_test3 = create_unit(&name_test3);
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
        let dep = UnitDep::new();
        let name_test1 = String::from("test1.service");
        let unit_test1 = create_unit(&name_test1);
        let name_test2 = String::from("test2.service");
        let unit_test2 = create_unit(&name_test2);
        let name_test3 = String::from("test3.service");
        let unit_test3 = create_unit(&name_test3);
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
        let dep = UnitDep::new();
        let name_test1 = String::from("test1.service");
        let unit_test1 = create_unit(&name_test1);
        let name_test2 = String::from("test2.service");
        let unit_test2 = create_unit(&name_test2);
        let name_test3 = String::from("test3.service");
        let unit_test3 = create_unit(&name_test3);
        let relation2 = UnitRelations::UnitRequires;
        let relation3 = UnitRelations::UnitWants;
        let atom2 = UnitRelationAtom::UnitAtomPullInStart; // + require, - want
        let atom3 = UnitRelationAtom::UnitAtomPullInStartIgnored; // - require, + want
        let atom = UnitRelationAtom::UnitAtomAddStopWhenUnneededQueue; // + require, + want

        let value = dep.is_dep_atom_with(&unit_test1, atom2, &unit_test2);
        assert_eq!(value, false);
        let value = dep.is_dep_atom_with(&unit_test1, atom3, &unit_test2);
        assert_eq!(value, false);
        let value = dep.is_dep_atom_with(&unit_test1, atom, &unit_test2);
        assert_eq!(value, false);
        let value = dep.is_dep_atom_with(&unit_test1, atom2, &unit_test3);
        assert_eq!(value, false);
        let value = dep.is_dep_atom_with(&unit_test1, atom3, &unit_test3);
        assert_eq!(value, false);
        let value = dep.is_dep_atom_with(&unit_test1, atom, &unit_test3);
        assert_eq!(value, false);

        dep.insert(
            Rc::clone(&unit_test1),
            relation2,
            Rc::clone(&unit_test2),
            true,
            0,
        )
        .unwrap();
        let value = dep.is_dep_atom_with(&unit_test1, atom2, &unit_test2);
        assert_eq!(value, true);
        let value = dep.is_dep_atom_with(&unit_test1, atom3, &unit_test2);
        assert_eq!(value, false);
        let value = dep.is_dep_atom_with(&unit_test1, atom, &unit_test2);
        assert_eq!(value, true);
        let value = dep.is_dep_atom_with(&unit_test1, atom2, &unit_test3);
        assert_eq!(value, false);
        let value = dep.is_dep_atom_with(&unit_test1, atom3, &unit_test3);
        assert_eq!(value, false);
        let value = dep.is_dep_atom_with(&unit_test1, atom, &unit_test3);
        assert_eq!(value, false);

        dep.insert(
            Rc::clone(&unit_test1),
            relation3,
            Rc::clone(&unit_test3),
            true,
            0,
        )
        .unwrap();
        let value = dep.is_dep_atom_with(&unit_test1, atom2, &unit_test2);
        assert_eq!(value, true);
        let value = dep.is_dep_atom_with(&unit_test1, atom3, &unit_test2);
        assert_eq!(value, false);
        let value = dep.is_dep_atom_with(&unit_test1, atom, &unit_test2);
        assert_eq!(value, true);
        let value = dep.is_dep_atom_with(&unit_test1, atom2, &unit_test3);
        assert_eq!(value, false);
        let value = dep.is_dep_atom_with(&unit_test1, atom3, &unit_test3);
        assert_eq!(value, true);
        let value = dep.is_dep_atom_with(&unit_test1, atom, &unit_test3);
        assert_eq!(value, true);
    }

    fn create_unit(name: &str) -> Rc<UnitX> {
        logger::init_log_with_console("test_unit_load", 4);
        log::info!("test");
        let dm = Rc::new(DataManager::new());
        let file = Rc::new(UnitFile::new());
        let unit_conf_parser_mgr = Rc::new(UnitParserMgr::default());
        let unit_type = UnitType::UnitService;
        let plugins = Rc::clone(&Plugin::get_instance());
        let mut config_path1 = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        config_path1.push("../target/debug");
        plugins
            .borrow_mut()
            .set_library_dir(&config_path1.to_str().unwrap());
        plugins.borrow_mut().load_lib();
        let mut config_path2 = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        config_path2.push("../target/release");
        plugins
            .borrow_mut()
            .set_library_dir(&config_path2.to_str().unwrap());
        plugins.borrow_mut().load_lib();
        let subclass = plugins.borrow().create_unit_obj(unit_type).unwrap();
        Rc::new(UnitX::new(
            dm,
            file,
            unit_conf_parser_mgr,
            unit_type,
            name,
            subclass.into_unitobj(),
        ))
    }

    fn contain_unit(units: &Vec<Rc<UnitX>>, unit: &Rc<UnitX>) -> bool {
        for u in units.iter() {
            if Rc::ptr_eq(u, unit) {
                return true;
            }
        }

        false
    }
}
