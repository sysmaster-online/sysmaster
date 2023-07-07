// Copyright (c) 2022 Huawei Technologies Co.,Ltd. All rights reserved.
//
// sysMaster is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan
// PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY
// KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

use super::entry::UnitX;
use super::rentry::UnitRe;
use crate::utils::table;
use child::UnitChild;
use core::error::*;
use core::rel::ReStation;
use core::unit::{UnitRelationAtom, UnitRelations, UnitType};
use deps::UnitDep;
use nix::unistd::Pid;
use nix::NixPath;
use sets::UnitSets;
use std::rc::Rc;
use table::TableSubscribe;

//#[derive(Debug)]
pub(crate) struct UnitDb {
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
    pub fn new(rentryr: &Rc<UnitRe>) -> UnitDb {
        let _units = Rc::new(UnitSets::new());
        UnitDb {
            units: Rc::clone(&_units),
            dep: UnitDep::new(rentryr, &_units),
            child: UnitChild::new(rentryr, &_units),
        }
    }

    pub(super) fn db_map_excl_units(&self, reload: bool) {
        // dep
        self.dep.db_map(reload);

        // child
        self.child.db_map(reload);
    }

    pub(super) fn db_insert_excl_units(&self) {
        // dep
        self.dep.db_insert();

        // child
        self.child.db_insert();
    }

    pub fn units_insert(&self, name: String, unit: Rc<UnitX>) -> Option<Rc<UnitX>> {
        self.units.insert(name, unit)
    }

    #[allow(dead_code)]
    pub fn unit_remove(&self, name: &str) {
        self.units.remove(name);
    }

    pub fn units_get(&self, name: &str) -> Option<Rc<UnitX>> {
        self.units.get(name)
    }

    pub fn units_get_all(&self, unit_type: Option<UnitType>) -> Vec<Rc<UnitX>> {
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

    pub fn units_register(
        &self,
        sub_name: &str,
        subscriber: Rc<dyn TableSubscribe<String, Rc<UnitX>>>,
    ) -> Option<Rc<dyn TableSubscribe<String, Rc<UnitX>>>> {
        self.units.register(sub_name, subscriber)
    }

    pub fn dep_insert(
        &self,
        source: Rc<UnitX>,
        relation: UnitRelations,
        dest: Rc<UnitX>,
        reference: bool,
        source_mask: u16,
    ) -> Result<()> {
        self.dep
            .insert(source, relation, dest, reference, source_mask)
    }

    pub fn dep_gets(&self, name: &str, relation: UnitRelations) -> Vec<Rc<UnitX>> {
        let unitx = self.units_get(name);

        if unitx.is_none() {
            return Vec::new();
        }

        self.dep.gets(&unitx.unwrap(), relation)
    }

    pub fn dep_gets_atom(&self, source: &UnitX, atom: UnitRelationAtom) -> Vec<Rc<UnitX>> {
        self.dep.gets_atom(source, atom)
    }

    pub fn dep_is_dep_atom_with(
        &self,
        source: &UnitX,
        atom: UnitRelationAtom,
        dest: &UnitX,
    ) -> bool {
        self.dep.is_dep_atom_with(source, atom, dest)
    }

    pub fn child_add_watch_pid(&self, id: &str, pid: Pid) {
        self.child.add_watch_pid(id, pid)
    }

    pub fn child_unwatch_pid(&self, id: &str, pid: Pid) {
        self.child.unwatch_pid(id, pid)
    }

    pub fn child_watch_all_pids(&self, id: &str) {
        let u = self.units_get(id).unwrap();
        let cg_path = u.cg_path();
        if cg_path.is_empty() {
            return;
        }

        let pids = cgroup::cg_get_pids(&cg_path);
        for pid in pids {
            log::debug!("watch all cgroup pids: {}", pid);
            self.child.add_watch_pid(id, pid)
        }
    }

    pub fn child_unwatch_all_pids(&self, id: &str) {
        self.child.unwatch_all_pids(id);
    }

    pub fn get_unit_by_pid(&self, pid: Pid) -> Option<Rc<UnitX>> {
        self.child.get_unit_by_pid(pid)
    }

    // repeating protection
    pub fn clear(&self) {
        self.child.entry_clear();
        self.dep.entry_clear();
        self.units.clear();
    }
}

// dependency: unit_sets -> {unit_dep | unit_child}
mod child;
mod deps;
mod sets;
