use super::uf_interface::UnitX;
use crate::manager::data::*;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::rc::Rc;

#[derive(Eq, PartialEq, Debug)]
struct UnitObjWrapper(Rc<RefCell<Rc<UnitX>>>);

impl Deref for UnitObjWrapper {
    type Target = Rc<RefCell<Rc<UnitX>>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Hash for UnitObjWrapper {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.borrow().hash(state);
    }
}

#[derive(Debug)]
pub struct UeDep {
    deps: HashMap<UnitRelations, RefCell<HashSet<UnitObjWrapper>>>,
}

impl UeDep {
    pub fn new() -> UeDep {
        UeDep {
            deps: HashMap::new(),
        }
    }

    pub fn updateDependencies(
        &mut self,
        relation: UnitRelations,
        unit: Rc<RefCell<Rc<UnitX>>>,
    ) -> Result<(), Box<dyn Error>> {
        let _relation = relation.clone();
        if !self.deps.contains_key(&_relation) {
            self.deps.insert(_relation, RefCell::new(HashSet::new()));
        }
        self.deps
            .get(&relation)
            .unwrap()
            .borrow_mut()
            .insert(UnitObjWrapper(unit.clone())); //todo!() is need clone ?
        Ok(())
    }

    pub fn get(&self, relation: UnitRelations) -> Vec<Rc<RefCell<Rc<UnitX>>>> {
        let mut unitxs = Vec::new();
        for (r, u) in self.getDependencies() {
            if r == relation {
                unitxs.push(Rc::clone(&u));
            }
        }
        unitxs
    }

    fn getDependencies(&self) -> Vec<(UnitRelations, Rc<RefCell<Rc<UnitX>>>)> {
        let mut v_dependencies: Vec<(UnitRelations, Rc<RefCell<Rc<UnitX>>>)> = Vec::new();
        for (k_r, v_set) in self.deps.iter() {
            for v_u in v_set.borrow().iter() {
                v_dependencies.push((*k_r, Rc::clone(&v_u.0)));
            }
        }
        v_dependencies
    }
}
