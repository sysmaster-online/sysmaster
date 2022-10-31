use libutils::path_lookup::LookupPaths;
use libutils::time_util;
use siphasher::sip::SipHasher24;
use std::cell::RefCell;
use std::collections::HashMap;
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

    pub fn build_name_map(&self, name: String, has_loaded: bool) -> bool {
        self.data.borrow_mut().build_id_map(name, has_loaded)
    }

    pub fn get_unit_id_fragment_pathbuf(&self, name: &String) -> Vec<PathBuf> {
        self.data.borrow().get_unit_id_fragment_pathbuf(name)
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
    last_updated_timestamp_hash: u64,
    lookup_path: Rc<LookupPaths>,
}

// the declaration "pub(self)" is for identification only.
impl UnitFileData {
    pub(self) fn new(lookup_path: &Rc<LookupPaths>) -> UnitFileData {
        UnitFileData {
            unit_id_fragment: HashMap::new(),
            unit_id_dropin_wants: HashMap::new(),
            unit_id_dropin_requires: HashMap::new(),
            unit_name_map: HashMap::new(),
            lookup_path: lookup_path.clone(),
            last_updated_timestamp_hash: 0,
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

    pub(self) fn build_id_map(&mut self, name: String, has_loaded: bool) -> bool {
        if !has_loaded || self.lookup_paths_updated() {
            self.build_id_fragment(&name);
            self.build_id_dropin(&name, "wants".to_string());
            self.build_id_dropin(&name, "requires".to_string());
        }
        false
    }

    fn build_id_fragment(&mut self, name: &String) {
        let mut pathbuf_fragment = Vec::new();
        for v in &self.lookup_path.search_path {
            if let Err(_e) = fs::metadata(v) {
                continue;
            }
            let pathd = format!("{}/{}.d", v, name);
            let dir = Path::new(&pathd);
            if dir.is_dir() {
                for entry in dir.read_dir().unwrap() {
                    let fragment = entry.unwrap().path();
                    if fragment.is_file() {
                        let file_name =
                            String::from(fragment.file_name().unwrap().to_str().unwrap());
                        if file_name.starts_with('.') || file_name.ends_with(".toml") {
                            continue;
                        }
                        let path = format!("{}.toml", fragment.to_string_lossy());

                        if let Err(e) = std::fs::copy(fragment, &path) {
                            log::warn!("copy file content to toml file error: {}", e);
                        }
                        pathbuf_fragment.push(Path::new(&path).to_path_buf());
                    }
                }
            }
            let path = if v.ends_with('/') {
                format!("{}{}", v, name)
            } else {
                format!("{}/{}", v, name)
            };
            let tmp = Path::new(&path);
            if tmp.exists() && !tmp.is_symlink() {
                let path = format!("{}.toml", tmp.to_string_lossy());
                if let Err(e) = std::fs::copy(tmp, &path) {
                    log::warn!("copy file content to toml file error: {}", e);
                }
                let to = Path::new(&path);
                pathbuf_fragment.push(to.to_path_buf());
            }
        }

        self.unit_id_fragment
            .insert(name.to_string(), pathbuf_fragment);
    }

    fn build_id_dropin(&mut self, name: &String, suffix: String) {
        let mut pathbuf_dropin = Vec::new();
        for v in &self.lookup_path.search_path {
            let path = format!("{}/{}.{}", v, name, suffix);
            let dir = Path::new(&path);
            if dir.is_dir() {
                for entry in dir.read_dir().unwrap() {
                    let dropin = entry.unwrap().path();
                    if dropin.is_symlink() {
                        if let Ok(abs_path) = dropin.canonicalize() {
                            let mut file_name = PathBuf::new();
                            file_name.push(abs_path.file_name().unwrap());
                            pathbuf_dropin.push(file_name);
                        }
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
    }

    pub(self) fn lookup_paths_updated(&mut self) -> bool {
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
                Err(_e) => {
                    log::debug!("unit file config lookup path {}  not found", dir);
                    continue;
                }
            }
        }

        let updated: u64 = siphash24.finish();

        let path_updated = updated != self.last_updated_timestamp_hash;
        self.last_updated_timestamp_hash = updated;
        path_updated
    }
}
