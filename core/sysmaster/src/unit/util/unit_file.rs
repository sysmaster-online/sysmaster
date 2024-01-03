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

use basic::fs::is_symlink;
use basic::fs::LookupPaths;
use core::unit::unit_name_is_valid;
use core::unit::UnitNameFlags;
use siphasher::sip::SipHasher24;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::hash::Hasher;
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub struct UnitFile {
    data: RefCell<UnitFileData>,
}

impl UnitFile {
    pub fn new(lookup_path: &Rc<LookupPaths>) -> UnitFile {
        UnitFile {
            data: RefCell::new(UnitFileData::new(lookup_path)),
        }
    }

    pub fn build_name_map(&self, name: String, has_loaded: bool) {
        self.data.borrow_mut().build_id_map(name, has_loaded);
    }

    pub fn get_unit_id_fragment_pathbuf(&self, name: &str) -> Vec<PathBuf> {
        self.data.borrow().get_unit_id_fragment_pathbuf(name)
    }

    pub fn get_real_name(&self) -> String {
        self.data.borrow().get_real_name()
    }

    pub fn get_all_names(&self) -> Vec<String> {
        self.data.borrow().get_all_names()
    }

    pub fn get_unit_wants_symlink_units(&self, name: &str) -> Vec<PathBuf> {
        self.data.borrow().get_unit_wants_symlink_units(name)
    }

    pub fn get_unit_requires_symlink_units(&self, name: &str) -> Vec<PathBuf> {
        self.data.borrow().get_unit_requires_symlink_units(name)
    }
}

#[derive(Debug)]
struct UnitFileData {
    pub unit_id_fragment: HashMap<String, Vec<PathBuf>>,
    pub real_name: String,
    pub all_names: HashSet<String>,
    pub unit_wants_symlink_units: HashMap<String, Vec<PathBuf>>,
    pub unit_requires_symlink_units: HashMap<String, Vec<PathBuf>>,
    _unit_name_map: HashMap<String, String>,
    last_updated_timestamp_hash: u64,
    lookup_path: Rc<LookupPaths>,
}

// the declaration "pub(self)" is for identification only.
impl UnitFileData {
    pub(self) fn new(lookup_path: &Rc<LookupPaths>) -> UnitFileData {
        UnitFileData {
            unit_id_fragment: HashMap::new(),
            real_name: String::new(),
            all_names: HashSet::new(),
            unit_wants_symlink_units: HashMap::new(),
            unit_requires_symlink_units: HashMap::new(),
            _unit_name_map: HashMap::new(),
            lookup_path: lookup_path.clone(),
            last_updated_timestamp_hash: 0,
        }
    }

    pub(self) fn get_unit_id_fragment_pathbuf(&self, name: &str) -> Vec<PathBuf> {
        match self.unit_id_fragment.get(name) {
            Some(v) => v.to_vec(),
            None => Vec::new(),
        }
    }

    pub(self) fn get_unit_wants_symlink_units(&self, name: &str) -> Vec<PathBuf> {
        match self.unit_wants_symlink_units.get(name) {
            Some(v) => v.to_vec(),
            None => Vec::<PathBuf>::new(),
        }
    }

    pub(self) fn get_unit_requires_symlink_units(&self, name: &str) -> Vec<PathBuf> {
        match self.unit_requires_symlink_units.get(name) {
            Some(v) => v.to_vec(),
            None => Vec::<PathBuf>::new(),
        }
    }

    pub(self) fn get_real_name(&self) -> String {
        self.real_name.clone()
    }

    pub(self) fn get_all_names(&self) -> Vec<String> {
        let mut res: Vec<String> = Vec::new();
        for v in &self.all_names {
            res.push(String::from(v));
        }
        res
    }

    pub(self) fn build_id_map(&mut self, name: String, has_loaded: bool) {
        if !has_loaded || self.lookup_paths_updated() {
            /* Forget the old thing, because some config files may have been deleted. */
            self.unit_id_fragment.remove(&name);
            self.unit_wants_symlink_units.remove(&name);
            self.unit_requires_symlink_units.remove(&name);

            self.build_id_fragment(&name);
            self.build_id_dropin(&name, "wants".to_string());
            self.build_id_dropin(&name, "requires".to_string());
        }
    }

    fn search_dropin_fragment(&mut self, path: &str, name: &str) -> Vec<PathBuf> {
        let mut res: Vec<PathBuf> = Vec::new();
        let pathd_str = format!("{}/{}.d", path, name);
        let dir = Path::new(&pathd_str);
        if !dir.is_dir() {
            return res;
        }
        for entry in dir.read_dir().unwrap() {
            let fragment = entry.unwrap().path();
            if !fragment.is_file() {
                continue;
            }
            let file_name = String::from(fragment.file_name().unwrap().to_str().unwrap());
            if file_name.ends_with(".conf") {
                res.push(fragment);
            }
        }
        res
    }

    fn build_id_fragment_by_name(&mut self, path: &str, name: &str) -> Option<Vec<PathBuf>> {
        let mut res: Vec<PathBuf> = Vec::new();
        if fs::metadata(path).is_err() {
            return None;
        }

        /* {/etc/sysmater/system, /usr/lib/sysmaster/system}/foo.service */
        let config_path = Path::new(path).join(name);
        if !config_path.exists() {
            if res.is_empty() {
                return None;
            } else {
                return Some(res);
            }
        }

        /* dispatch symlinks */
        for de in Path::new(path).read_dir().unwrap() {
            let de = de.unwrap().path();
            if !is_symlink(&de) {
                continue;
            }

            let file_name = de.file_name().unwrap().to_string_lossy().to_string();
            let target_path = match basic::fs::chase_symlink(&de) {
                Err(e) => {
                    log::debug!("Failed to get the symlink of {:?}: {}, ignoring.", de, e);
                    continue;
                }
                Ok(v) => v,
            };
            let target_name = match target_path.file_name() {
                None => {
                    log::error!("Failed to get the filename of {:?}", target_path);
                    return None;
                }
                Some(v) => v.to_string_lossy().to_string(),
            };

            /* Found a symlink points to the real unit. */
            if target_name == name {
                if !unit_name_is_valid(&target_name, UnitNameFlags::ANY) {
                    continue;
                }
                /* Add this symlink to all_names */
                self.all_names.insert(file_name.clone());
            }
            /* We are processing an alias service. */
            if file_name == name {
                if !unit_name_is_valid(&target_name, UnitNameFlags::ANY) {
                    /* So this symlink is pointing an invalid unit, mark the vector as empty and
                     * we will treat it as masked. */
                    return Some(Vec::new());
                }
                self.real_name = target_name;
                self.all_names.insert(file_name);
                res.push(de);
                return Some(res);
            }
        }

        res.push(config_path);
        Some(res)
    }

    fn build_id_fragment(&mut self, name: &str) {
        let mut pathbuf_fragment = Vec::new();
        let search_path_list = self.lookup_path.search_path.clone();
        for search_path in &search_path_list {
            let mut v = match self.build_id_fragment_by_name(search_path, name) {
                None => continue,
                Some(v) => v,
            };
            /* v is empty when we find a symlink, but it points to a invalid target. If
             * pathbuf_fragment is also empty, this means we haven't found a valid path under
             * higher priority search path. */
            if v.is_empty() && pathbuf_fragment.is_empty() {
                /* unit is masked */
                return;
            }
            pathbuf_fragment.append(&mut v);
            /* One is enough. */
            break;
        }

        if !pathbuf_fragment.is_empty() || !name.contains('@') {
            for search_path in &search_path_list {
                let mut v = self.search_dropin_fragment(search_path, name);
                if v.is_empty() {
                    continue;
                }
                pathbuf_fragment.append(&mut v);
                break;
            }

            self.unit_id_fragment
                .insert(name.to_string(), pathbuf_fragment);
            return;
        }

        /* This is a template service and we didn't find its instance configuration file, try to
         * load the template configuration file. */
        let template_name = name.split_once('@').unwrap().0.to_string() + "@.service";
        for search_path in &search_path_list {
            let mut v = match self.build_id_fragment_by_name(search_path, &template_name) {
                None => continue,
                Some(v) => v,
            };
            if v.is_empty() && pathbuf_fragment.is_empty() {
                /* unit is masked */
                return;
            }
            pathbuf_fragment.append(&mut v);
            break;
        }

        for search_path in &search_path_list {
            let mut v = self.search_dropin_fragment(search_path, &template_name);
            if v.is_empty() {
                continue;
            }
            pathbuf_fragment.append(&mut v);
            break;
        }

        self.unit_id_fragment
            .insert(name.to_string(), pathbuf_fragment);
    }

    fn build_id_dropin(&mut self, name: &str, suffix: String) {
        let mut pathbuf_dropin = Vec::new();
        for v in &self.lookup_path.search_path {
            let path = format!("{}/{}.{}", v, name, suffix);
            let dir = Path::new(&path);
            if !dir.is_dir() {
                continue;
            }
            for entry in dir.read_dir().unwrap() {
                let symlink_unit = entry.unwrap().path();
                if !is_symlink(symlink_unit.as_path()) {
                    continue;
                }

                let symlink_name = symlink_unit.file_name().unwrap();
                let mut file_name = PathBuf::new();
                if symlink_name.to_str().unwrap().contains('@') {
                    file_name.push::<PathBuf>(symlink_name.into());
                } else {
                    file_name = match symlink_unit.canonicalize() {
                        Err(_) => continue,
                        Ok(v) => v,
                    }
                    .file_name()
                    .unwrap()
                    .into();
                }
                pathbuf_dropin.push(file_name);
            }
        }

        match suffix.as_str() {
            "wants" => self
                .unit_wants_symlink_units
                .insert(name.to_string(), pathbuf_dropin),
            "requires" => self
                .unit_requires_symlink_units
                .insert(name.to_string(), pathbuf_dropin),
            _ => unimplemented!(),
        };
    }

    pub(self) fn lookup_paths_updated(&mut self) -> bool {
        let mut siphash24 = SipHasher24::new_with_keys(0, 0);
        for dir in &self.lookup_path.search_path {
            let metadata = match fs::metadata(dir) {
                Err(e) => {
                    log::debug!("Couldn't find unit config lookup path {}: {}", dir, e);
                    continue;
                }
                Ok(v) => v,
            };
            let time = match metadata.modified() {
                Err(_) => {
                    log::error!("Failed to get mtime of {}", dir);
                    continue;
                }
                Ok(v) => v,
            };
            siphash24.write_u128(basic::unistd::timespec_load(time));
        }

        let updated: u64 = siphash24.finish();

        let path_updated = updated != self.last_updated_timestamp_hash;
        self.last_updated_timestamp_hash = updated;
        path_updated
    }
}
