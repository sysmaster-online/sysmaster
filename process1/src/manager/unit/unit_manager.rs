use std::cell::RefCell;
use std::rc::Rc;
use std::collections::{HashSet, HashMap, VecDeque};
use utils::path_lookup::LookupPaths;
use super::{UnitType, Unit,UnitObj};
use super::{unit_new};
use std::fs;
use utils:: {time_util, path_lookup, unit_config_parser};
use crate::manager::service;
use siphasher::sip::SipHasher24;
use walkdir::{DirEntry,WalkDir};
use std::hash::Hasher;

#[macro_use]
use crate::unit_name_to_type;
//unitManger composition of units with hash map
pub struct UnitManager {
    units: Rc<RefCell<HashMap<String, Rc<RefCell<Box<dyn UnitObj>>>>>>,
    unit_id_map: HashMap<String, String>,
    unit_name_map: HashMap<String, String>,
    lookup_path: LookupPaths,
    last_updated_timestamp_hash: u64,
    load_queue: VecDeque<Rc<RefCell<Box<dyn UnitObj>>>>,
}


impl UnitManager{
    pub fn new() -> Self{
        UnitManager {
            units: Rc::new(RefCell::new(HashMap::new())),
            unit_id_map: HashMap::new(),
            unit_name_map: HashMap::new(),
            last_updated_timestamp_hash: 0,
            lookup_path: path_lookup::LookupPaths::new(),
            load_queue: VecDeque::new(),
        }
    }

    pub fn insert_unit(&self, name: String, unit: Rc<RefCell<Box<dyn UnitObj>>>) {
	    let mut units = self.units.borrow_mut();
	    units.insert(name, unit);
    }

    pub fn get_unit_on_name(&self, name: &str) -> Option<Rc<RefCell<Box<dyn UnitObj>>>> {
        self.units.borrow_mut().get(name).and_then(|u| Some(u.clone()))
    }

    pub fn build_name_map(&mut self) -> bool {
	    let mut timestamp_hash_new: u64 = 0;
        if !self.lookup_paths_updated(&mut timestamp_hash_new) {
            return false;
        }

        for dir in &self.lookup_path.search_path {
            if !std::path::Path::new(&dir).exists() {
                log::warn!("dir {} is not exist", dir);
                continue;
	        }
            for entry in WalkDir::new(&dir)
                .min_depth(1)
	            .max_depth(1)
                .into_iter() {
                let entry = entry.unwrap();
                let filename = entry.file_name().to_str().unwrap().to_string();
                let file_path = entry.path().to_str().unwrap().to_string();
                if self.unit_id_map.contains_key(&filename) {
                    continue;
                }
                self.unit_id_map.insert(filename, file_path);
            }
        }
	    self.last_updated_timestamp_hash = timestamp_hash_new;
        return true;
    }

    pub fn get_unit_file_path(&self, unit_name: &str) -> Option<&String> {
        match self.unit_id_map.get(unit_name) {
            None => { return None; },
            Some(v) => {
                return Some(v);
            },
        }
    }

    fn lookup_paths_updated(&mut self, timestamp_new: &mut u64) -> bool {
        let updated: u64;
        let mut siphash24 = SipHasher24::new_with_keys(0, 0);
        for dir in &self.lookup_path.search_path {
	    match fs::metadata(&dir) {
                Ok(metadata) => match metadata.modified() {
                    Ok(time) => {
                        siphash24.write_u128(time_util::timespec_load(time));
                    },
                    _ => {
                        log::error!("failed to get mtime {}", dir);
                    },
                }
                _ => {
                    log::error!("failed to get metadata of {}", dir);
                }
            }
        }

        updated = siphash24.finish();
        *timestamp_new = updated;
        return updated != self.last_updated_timestamp_hash;
    }

    pub fn init_lookup_path(&mut self) {
        self.lookup_path.init_lookup_paths();
    }


    pub fn dispatch_load_queue(&mut self) {
        log::debug!("dispatch load queue");

        loop {
            match self.load_queue.pop_front() {
                None => {break},
                Some(unit) => {
                    match unit.borrow_mut().load(self) {
                        Ok(()) => {continue},
                        Err(e) => {
                            log::error!("load unit config failed: {}", e.to_string())
                        }
                    }
                }
            }
        }
    }

    pub fn push_load_queue(&mut self, unit: Rc<RefCell<Box<dyn UnitObj>>>) {
        self.load_queue.push_back(unit);
    }

    
    
    fn prepare_unit(&mut self, name: &str) -> Option<Rc<RefCell<Box<dyn UnitObj>>>> { 
        let unit_type = unit_name_to_type!(name);
        if unit_type == UnitType::UnitTypeInvalid {
            return None;
        }

        let unit = unit_new(unit_type, name);
        let u = Rc::new(RefCell::new(unit));
        self.insert_unit(name.to_string(), u.clone());
        Some(u.clone())
    }

    pub fn load_unit(&mut self, name: &str) -> Option<Rc<RefCell<Box<dyn UnitObj>>>> {
        if let Some(unit) = self.get_unit_on_name(name) {
             return Some(unit);
        }; 

        let unit = self.prepare_unit(name);
        let u = if let Some(u) = unit {
            u
        } else {
            return None;
        };

        self.push_load_queue(u.clone());
        self.dispatch_load_queue();
        Some(u.clone())
    }
}



#[cfg(test)]
mod tests {
    use crate::manager::service::ServiceUnit;

    use super::*;


    #[test]
    fn  test_unit_load(){
        let mut unit_manager = UnitManager::new();
        unit_manager.init_lookup_path();

        let unit_name = String::from("config.service");
        unit_manager.load_unit(&unit_name);

        assert_ne!(unit_manager.units.borrow().len(), 0);

        let unit_obj = unit_manager.get_unit_on_name(&unit_name).unwrap();
        let unit = unit_obj.borrow();
        let service_unit = unit.as_any().downcast_ref::<ServiceUnit>().unwrap();
        assert_eq!(&service_unit.get_unit_name(), &unit_name);
    }
}
