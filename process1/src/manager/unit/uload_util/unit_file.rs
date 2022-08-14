use siphasher::sip::SipHasher24;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::hash::Hasher;
use std::path::{Path, PathBuf};
use utils::path_lookup::LookupPaths;
use utils::{path_lookup, time_util};

pub struct UnitFile {
    data: RefCell<UnitFileData>,
}

impl UnitFile {
    pub fn new() -> UnitFile {
        UnitFile {
            data: RefCell::new(UnitFileData::new()),
        }
    }

    pub fn build_name_map(&self, name: String) -> bool {
        self.data.borrow_mut().build_id_map(name)
    }

    pub fn get_unit_id_fragment_pathbuf(&self, name: &String) -> Vec<PathBuf> {
        self.data
            .borrow()
            .get_unit_id_fragment_pathbuf(name)
            .clone()
    }

    pub fn get_unit_id_dropin_wants(&self, name: &String) -> Vec<PathBuf> {
        self.data.borrow().get_unit_id_dropin_wants(name)
    }

    pub fn get_unit_id_dropin_requires(&self, name: &String) -> Vec<PathBuf> {
        self.data.borrow().get_unit_id_dropin_requires(name)
    }
}

#[derive(Debug)]
struct UnitFileData {
    pub unit_id_fragment: HashMap<String, Vec<PathBuf>>,
    pub unit_id_dropin_wants: HashMap<String, Vec<PathBuf>>,
    pub unit_id_dropin_requires: HashMap<String, Vec<PathBuf>>,
    unit_name_map: HashMap<String, String>,
    lookup_path: LookupPaths,
    last_updated_timestamp_hash: u64,
    updated_timestamp_hash: u64,
}

// the declaration "pub(self)" is for identification only.
impl UnitFileData {
    pub(self) fn new() -> UnitFileData {
        let mut lookup_path = path_lookup::LookupPaths::new();
        lookup_path.init_lookup_paths();
        UnitFileData {
            unit_id_fragment: HashMap::new(),
            unit_id_dropin_wants: HashMap::new(),
            unit_id_dropin_requires: HashMap::new(),
            unit_name_map: HashMap::new(),
            lookup_path,
            last_updated_timestamp_hash: 0,
            updated_timestamp_hash: 0,
        }
    }

    pub(self) fn get_unit_id_fragment_pathbuf(&self, name: &String) -> Vec<PathBuf> {
        match self.unit_id_fragment.get(name) {
            Some(v) => v.to_vec(),
            None => Vec::new(),
        }
    }

    pub(self) fn get_unit_id_dropin_wants(&self, name: &String) -> Vec<PathBuf> {
        match self.unit_id_dropin_wants.get(name) {
            Some(v) => v.to_vec(),
            None => Vec::<PathBuf>::new(),
        }
    }

    pub(self) fn get_unit_id_dropin_requires(&self, name: &String) -> Vec<PathBuf> {
        match self.unit_id_dropin_requires.get(name) {
            Some(v) => v.to_vec(),
            None => Vec::<PathBuf>::new(),
        }
    }

    pub(self) fn build_id_map(&mut self, name: String) -> bool {
        if let true = self.lookup_paths_updated() {
            self.build_id_fragment(&name);
            self.build_id_dropin(&name, "wants".to_string());
            self.build_id_dropin(&name, "requires".to_string());

            self.last_updated_timestamp_hash = self.updated_timestamp_hash;
        }

        false
    }

    pub fn build_id_fragment(&mut self, name: &String) {
        let mut pathbuf_fragment = Vec::new();
        for v in &self.lookup_path.search_path {
            let path = format!("{}/{}", v, name);
            let tmp = Path::new(&path);
            if tmp.exists() && !tmp.is_symlink() {
                let path = format!("{}.toml", tmp.to_string_lossy().to_string());
                std::fs::copy(tmp, &path);
                let to = Path::new(&path);
                pathbuf_fragment.push(to.to_path_buf());
            }
            let pathd = format!("{}/{}.d", v, name);
            let dir = Path::new(&pathd);
            if dir.is_dir() {
                for entry in dir.read_dir().unwrap() {
                    let fragment = entry.unwrap().path();
                    if fragment.is_file() {
                        let path = format!("{}.toml", fragment.to_string_lossy().to_string());
                        std::fs::copy(fragment, &path);
                        let to = Path::new(&path);
                        pathbuf_fragment.push(to.to_path_buf());
                    }
                }
            }
        }

        self.unit_id_fragment
            .insert(name.to_string(), pathbuf_fragment);
    }

    pub fn build_id_dropin(&mut self, name: &String, suffix: String) {
        let mut pathbuf_dropin = Vec::new();
        for v in &self.lookup_path.search_path {
            let path = format!("{}/{}.{}", v, name, suffix);
            let dir = Path::new(&path);
            if dir.is_dir() {
                for entry in dir.read_dir().unwrap() {
                    let dropin = entry.unwrap().path();
                    if dropin.is_symlink() {
                        pathbuf_dropin.push(dropin);
                    }
                }
            }
        }

        match suffix.as_str() {
            "wants" => self
                .unit_id_dropin_wants
                .insert(name.to_string(), pathbuf_dropin),
            "requires" => self
                .unit_id_dropin_requires
                .insert(name.to_string(), pathbuf_dropin),
            _ => unimplemented!(),
        };
        ()
    }

    pub(self) fn lookup_paths_updated(&mut self) -> bool {
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
                    log::debug!("lookup path {}   of unit file config err: {}", dir, e);
                }
            }
        }

        updated = siphash24.finish();
        self.updated_timestamp_hash = updated;
        return updated != self.last_updated_timestamp_hash;
    }
}
