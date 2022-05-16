use siphasher::sip::SipHasher24;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::hash::Hasher;
use utils::path_lookup::LookupPaths;
use utils::{path_lookup, time_util};
use walkdir::WalkDir;

pub(in crate::manager::unit) struct UnitFile {
    data: RefCell<UnitFileData>,
}

impl UnitFile {
    pub(in crate::manager::unit) fn new() -> UnitFile {
        UnitFile {
            data: RefCell::new(UnitFileData::new()),
        }
    }

    pub(in crate::manager::unit) fn build_name_map(&self) -> bool {
        self.data.borrow_mut().build_name_map()
    }

    pub(in crate::manager::unit) fn get_unit_file_path(&self, unit_name: &str) -> Option<String> {
        self.data.borrow().get_unit_file_path(unit_name)
    }

    pub(in crate::manager::unit) fn init_lookup_path(&self) {
        self.data.borrow_mut().init_lookup_path()
    }
}

#[derive(Debug)]
struct UnitFileData {
    unit_id_map: HashMap<String, String>,
    unit_name_map: HashMap<String, String>,
    lookup_path: LookupPaths,
    last_updated_timestamp_hash: u64,
}

// the declaration "pub(self)" is for identification only.
impl UnitFileData {
    pub(self) fn new() -> UnitFileData {
        UnitFileData {
            unit_id_map: HashMap::new(),
            unit_name_map: HashMap::new(),
            last_updated_timestamp_hash: 0,
            lookup_path: path_lookup::LookupPaths::new(),
        }
    }

    pub(self) fn build_name_map(&mut self) -> bool {
        let mut timestamp_hash_new: u64 = 0;
        if !self.lookup_paths_updated(&mut timestamp_hash_new) {
            return false;
        }

        for dir in &self.lookup_path.search_path {
            if !std::path::Path::new(&dir).exists() {
                log::warn!("dir {} is not exist", dir);
                continue;
            }
            let mut tmp_dir = dir.to_string();
            if tmp_dir.ends_with("libutils") {
                tmp_dir.push_str("/examples/");
            }
            for entry in WalkDir::new(&tmp_dir.as_str())
                .min_depth(1)
                .max_depth(1)
                .into_iter()
            {
                let entry = entry.unwrap();
                let filename = entry.file_name().to_str().unwrap().to_string();
                let file_path = entry.path().to_str().unwrap().to_string();
                self.unit_id_map.insert(filename, file_path);
            }
        }
        self.last_updated_timestamp_hash = timestamp_hash_new;
        return true;
    }

    pub(self) fn get_unit_file_path(&self, unit_name: &str) -> Option<String> {
        match self.unit_id_map.get(unit_name) {
            None => {
                return None;
            }
            Some(v) => {
                return Some(v.to_string());
            }
        }
    }

    pub(self) fn init_lookup_path(&mut self) {
        self.lookup_path.init_lookup_paths();
    }

    fn lookup_paths_updated(&mut self, timestamp_new: &mut u64) -> bool {
        let updated: u64;
        let mut siphash24 = SipHasher24::new_with_keys(0, 0);
        for dir in &self.lookup_path.search_path {
            match fs::metadata(&dir) {
                Ok(metadata) => match metadata.modified() {
                    Ok(time) => {
                        siphash24.write_u128(time_util::timespec_load(time));
                    }
                    _ => {
                        log::error!("failed to get mtime {}", dir);
                    }
                },
                Err(e) => {
                    log::error!("failed to get metadata of {}, err: {}", dir, e);
                }
            }
        }

        updated = siphash24.finish();
        *timestamp_new = updated;
        return updated != self.last_updated_timestamp_hash;
    }
}
