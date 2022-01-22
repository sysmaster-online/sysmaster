extern crate siphasher;

use super::{manager, service};
use std::any::Any;
use std::error::Error;
use std::fs;
use std::collections::{HashSet, HashMap, VecDeque};
use std::cell::RefCell;
use std::fs::File;
use std::rc::Rc;
use std::os::unix::fs::FileTypeExt;
use siphasher::sip::SipHasher24;
use std::hash::Hasher;
use walkdir::WalkDir;
use std::hash::Hash;

use utils:: {time_util, path_lookup, unit_config_parser};

#[derive(Debug, PartialEq, Eq)]
pub enum UnitType {
    UnitService = 0,
    UnitTarget,
    UnitTypeMax,
    UnitTypeInvalid,
    UnitTypeErrnoMax,
}
#[derive(Hash, PartialEq, Eq, Debug, Copy, Clone)]
enum UnitRelations {
    UnitRequires,
    UnitWants,
    UnitBindsTo,

    UnitRequiresBy,
    UnitWantsBy,
    UnitBoundBy,

    UnitBefore,
    UnitAfter,
}

#[derive(PartialEq, Debug, Eq)]
pub enum UnitLoadState {
    UnitStub = 0,
    UnitLoaded,
    UnitNotFound,
    UnitError,
    UnitMerged,
    UnitMasked,
    UnitLoadStateMax,
    UnitLoadStateInvalid = -1,
}

enum UnitNameFlags {
    UnitNamePlain =1,
    UnitNameInstance = 2,
    UnitNameTemplate = 4,
    UnitNameAny = 1|2|4,
}

enum UnitFileState {
    UnitFileEnabled,
    UnitFileEnabledRuntime,
    UnitFileLinked,
    UnitFileLinkedRuntime,
    UnitFileAlias,
    UnitFileMasked,
    UnitFileMaskedRuntime,
    UnitFileStatic,
    UnitFileDisabled,
    UnitFileIndirect,
    UnitFileGenerated,
    UnitFileTransient,
    UnitFileBad,
    UnitFileStateMax,
    UnitFileStateInvalid,
}

pub struct Unit {
    pub unit_type: UnitType,
    pub load_state: UnitLoadState,
    pub id: String,
    instance: Option<String>,
    pub name: String,
    dependencies: HashMap<UnitRelations, RefCell<HashSet<UnitObjWrapper>>>,
    desc: String,
    documnetation: String,
    config_file_path: String,
    config_file_mtime: u128,

    pids: HashSet<u64>,
    sigchldgen: u64,
    deseialize_job: i32,
    load_error: i32,
    stop_when_unneeded: bool,
    transient: bool,
    in_load_queue: bool,
    default_dependencies: bool,
    pub conf: Option<Rc<unit_config_parser::Conf>>,
}

impl PartialEq for Unit {
     fn eq(&self, other: &Unit) -> bool {
         self.unit_type == other.unit_type && self.id == other.id
     }
}

pub trait UnitObj {
    fn init(&self){}
    fn done(&self){}
    fn load(&mut self, _manager: &mut UnitManager) -> Result<(), Box<dyn Error>> {Ok(())}
    fn coldplug(&self){}
    fn dump(&self){}
    fn start(&self){}
    fn stop(&self){}
    fn reload(&self){}
    fn kill(&self){}
    fn check_gc(&self)->bool;
    fn release_resources(&self){}
    fn check_snapshot(&self){}
    fn sigchld_events(&self, _pid:u64,_code:i32, _status:i32){}
    fn reset_failed(&self){}

    fn eq(&self, other: &dyn UnitObj) -> bool;
    fn hash(&self) -> u64;
    fn as_any(&self) -> &dyn Any;
}

impl PartialEq for Box<dyn UnitObj> {
    fn eq(&self, other: &Self) -> bool {
        UnitObj::eq(self.as_ref(), other.as_ref())
    }
}

impl Eq for Box<dyn UnitObj> {}

impl Hash for Box<dyn UnitObj> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let key_hash = UnitObj::hash(self.as_ref());
        state.write_u64(key_hash);
    }
}

#[derive(Eq, PartialEq)]
struct UnitObjWrapper(Rc<RefCell<Box<dyn UnitObj>>>);

impl Hash for UnitObjWrapper {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.borrow().hash(state);
    }
}

// #[derive(Hash, PartialEq, Eq)]
struct MountUnit {
    mount_unit:Unit,
}

#[macro_export]
macro_rules! null_str{
    ($name:expr) => {
        String::from($name)
    }
}

impl Unit {
    pub fn new(name: String) -> Self {
        Unit{
            unit_type: UnitType::UnitTypeInvalid,
            load_state: UnitLoadState::UnitStub,
            id: name,
            instance: Some(String::from("")),
            name: String::from(""),
            dependencies: HashMap::new(),
            desc: String::from(""),
            documnetation: null_str!(""),
            config_file_path: null_str!(""),
            config_file_mtime: 0,
            deseialize_job:0,
            pids: HashSet::<u64>::new(),
            sigchldgen: 0,
            load_error: 0,
            stop_when_unneeded: false,
            transient: false,
            in_load_queue: false,
            default_dependencies: true,
            conf: None,
        }
    }

    pub fn set_load_state(&mut self, load_state: UnitLoadState){
        self.load_state = load_state;
    }

    pub fn unit_load(&mut self, manager: &mut UnitManager, frament_required: bool) -> Result<(), Box<dyn Error>> {
        self.unit_config_load(manager)?;

        if self.load_state == UnitLoadState::UnitStub {
                if frament_required {
                    return Ok(());
                }
                self.load_state = UnitLoadState::UnitLoaded;
        }

        self.unit_load_dropin()?;
        return Ok(());

    }

    fn unit_load_dropin(&mut self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    fn unit_config_load(&mut self, manager: &mut UnitManager) -> Result<(), Box<dyn Error>> {
        let unit_path: String;

        if self.transient {
            self.load_state = UnitLoadState::UnitLoaded;
            return Ok(());
        }

        self.build_name_map(manager);

        match self.get_unit_file_path(manager) {
            Some(v) => {unit_path = v.to_string()},
            None => {
                log::error!("not found unit file {}", self.id);
                return Err(format!("not found unit file {}", self.id).into());
            },
        }

        log::debug!("build unit map success, unit path {}", unit_path);
        if self.config_file_path!= unit_path {
            self.config_file_path = unit_path;
        }

        if self.config_file_path.is_empty() {
            return Err(format!("config file path is empty").into());
        }

        let file = File::open(&self.config_file_path)?;
        let meta = file.metadata()?;

        if (meta.is_file() && meta.len() <=0) || meta.file_type().is_char_device() {
            self.load_state = UnitLoadState::UnitLoaded;
            self.config_file_mtime = 0;
        } else {
            let mtime = meta.modified()?;
            self.config_file_mtime = time_util::timespec_load(mtime);
            self.load_state = UnitLoadState::UnitLoaded;
            match unit_config_parser::unit_file_load(self.config_file_path.to_string()) {
                Ok(conf) => self.conf = Some(Rc::new(conf)),
                Err(e) => {
                    return Err(format!("file load err {:?}", e).into());
                },
            }

            log::debug!("config file mtime is: {}", self.config_file_mtime);
        }

        return Ok(());
    }

    fn build_name_map(&self, manager: &mut UnitManager) {
        manager.build_name_map();
    }

    fn get_unit_file_path(&self, manager: &mut UnitManager) -> Option<String> {
        match manager.get_unit_file_path(&self.id) {
            Some(v) => { return  Some(v.to_string())},
            None => {
                log::error!("not find unit file {}", self.id);
                None
            },
        }
    }

    fn parse_unit_relations(&mut self, manager: &mut UnitManager, units: &str, relation: UnitRelations) -> Result<(), Box<dyn Error>> {
        let units = units.split_whitespace();
        for unit in units {
            self.parse_unit_relation(manager, unit, relation)?;
        }
        Ok(())
    }

    fn parse_unit_relation(&mut self, manager: &mut UnitManager, unit_name: &str, relation: UnitRelations) -> Result<(), Box<dyn Error>> {
        log::debug!("parse relation unit relation name is {}, relation is {:?}", unit_name, relation);

        let unit_type = manager.unit_name_to_type(unit_name);
        if unit_type == UnitType::UnitTypeInvalid {
            return Err(format!("invalid unit type of unit {}", unit_name).into());
        }

        let other = if let Some(_unit) = manager.get_unit_on_name(&unit_name) {
             return Ok(());
        } else {
            let unit = unit_new(unit_type, unit_name.to_string());
            let u = Rc::new(RefCell::new(unit));
            manager.push_load_queue(u.clone());
            u
        };

        manager.insert_unit(unit_name.to_string(), other.clone());
        self.unit_update_dependency(relation, other.clone());
        Ok(())
    }

    fn unit_update_dependency(&mut self, relation: UnitRelations, other: Rc<RefCell<Box<dyn UnitObj>>>) {

        if !self.dependencies.contains_key(&relation.clone()) {
            self.dependencies.insert(relation.clone(), RefCell::new(HashSet::new()));
        }

        let mut dependencies = self.dependencies.get(&relation).unwrap().borrow_mut();
        dependencies.insert(UnitObjWrapper(other.clone()));
    }
 }

/*
impl <'l> Default for Unit<'l> {
    fn default() -> Self {
        Self{
            unitType: UnitType::UNIT_SERVICE,
            load_state: UnitLoadState::UNIT_STUB,
            id: String::from(""),
            instance: String::from(""),
            name: String::from(""),
            depencies: Vec::<Unit>::new(),
            desc: String::from(""),
            documnetation: null_str!(""),
            config_file_path: null_str!(""),
            source_path: null_str!(""),
            config_file_mtime: 0,
            source_mtime: 0,
            dropin_mtime: 0,
            units_by_type: Vec::<Unit>::new(),
            has_requires_mounts_for: Vec::<Unit>::new(),
            load_queue: Vec::<Unit>::new(),
            dbus_queue: Vec::<Unit>::new(),
            cleanup_queue: Vec::<Unit>::new(),
            gc_queue: Vec::<Unit>::new(),
            cgroup_queue: Vec::<Unit>::new(),
            pids: HashSet::<u64>::new(),
            sigchldgen: 0,
            gc_marker: 0,
            deseialize_job: 0,
            load_error: 0,
            stop_when_unneeded: false,
            refuse_manual_start: false,
            allow_isolate: false,
            ignore_on_isolate: false,
            ignore_on_snapshot: false,
            condition_result: false,
            assert_result: false,
            transient: false,
            in_load_queue: false,
            in_dubs_queue: false,
            in_cleanup_queue: false,
            in_gc_queue: false,
            manager: None,
        }
    }
}
*/


impl UnitObj for Unit{
    fn init(&self){

    }
    fn done(&self){

    }
    fn check_gc(&self) -> bool { todo!() }

    fn eq(&self, _other: &dyn UnitObj) -> bool {
        todo!()
    }

    fn hash(&self) -> u64 {
        todo!()
    }

    fn as_any(&self) -> &dyn Any {
        todo!()
    }
}

impl  UnitObj for MountUnit{
    fn init(&self) { todo!() }
    fn done(&self) { todo!() }
    fn load(&mut self, _manager: &mut UnitManager) -> Result<(), Box<dyn Error>> { todo!() }
    fn coldplug(&self) { todo!() }
    fn start(&self) { todo!() }
    fn dump(&self) { todo!() }
    fn stop(&self) { todo!() }
    fn reload(&self) { todo!() }
    fn kill(&self) { todo!() }
    fn check_gc(&self) -> bool { todo!() }
    fn release_resources(&self) { todo!() }
    fn check_snapshot(&self) { todo!() }
    fn sigchld_events(&self, _: u64, _: i32, _: i32) { todo!() }
    fn reset_failed(&self) { todo!() }

    fn eq(&self, _other: &dyn UnitObj) -> bool {
        todo!()
    }

    fn hash(&self) -> u64 {
        todo!()
    }

    fn as_any(&self) -> &dyn Any {
        todo!()
    }
}

fn unit_new(unit_type: UnitType, name: String) -> Box<dyn UnitObj> {
    let unit = Unit::new(name);

    match unit_type {
        UnitType::UnitService => {
            return Box::new(service::ServiceUnit::new(unit))
        },
        UnitType::UnitTarget => {
            return Box::new(service::ServiceUnit::new(unit))
        },
        _ => {
            return Box::new(service::ServiceUnit::new(unit))
        },
    }
}

pub struct UnitManager {
    units: RefCell<HashMap<String, Rc<RefCell<Box<dyn UnitObj>>>>>,
    unit_id_map: HashMap<String, String>,
    unit_name_map: HashMap<String, String>,
    lookup_path: path_lookup::LookupPaths,
    last_updated_timestamp_hash: u64,
    load_queue: VecDeque<Rc<RefCell<Box<dyn UnitObj>>>>,
}

impl UnitManager{
    pub fn new() -> Self{
        UnitManager {
            units: RefCell::new(HashMap::new()),
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
        self.units.borrow().get(name).and_then(|u| Some(u.clone()))
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

    pub fn unit_name_to_type(&self, unit_name: &str) -> UnitType {
        let words: Vec<&str> = unit_name.split(".").collect();
        match words[words.len()-1] {
            "service" => UnitType::UnitService,
            "target" => UnitType::UnitTarget,
            _ => UnitType::UnitTypeInvalid,
        }
    }


    fn prepare_unit(&mut self, name: &str) -> Option<Rc<RefCell<Box<dyn UnitObj>>>> { 
        let unit_type = self.unit_name_to_type(name);
        if unit_type == UnitType::UnitTypeInvalid {
            return None;
        }

        let unit = unit_new(unit_type, name.to_string());
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

impl  manager::Mangerobj for UnitManager  {
    fn init(&self){
    }

    fn load(&self){
        // let mut units_vec = self.units.borrow_mut();
        // let mut unit = unit_new(*self, UnitType::UNIT_SERVICE);

        // units_vec.insert(String::from("systemd"), RefCell::new(unit));
    }

    fn dispatch(&self) -> i32 {
        0
    }

    fn reload(&self) -> Option<i32>{
        None
    }

    fn destroy(&self){

    }

    // reserved for sd event
    fn event_dispatch(&self) -> Option<i32>{
        None
    }
}

pub trait ConfigParser {
    fn parse(&mut self, _manager: &mut UnitManager) -> Result<(), Box<dyn Error>> { Ok(())}
}

impl ConfigParser for Unit {
    fn parse(&mut self, manager: &mut UnitManager)  -> Result<(), Box<dyn Error>> {
        // impl ugly
        if self.conf.is_none() {
            return Err(format!("load config file failed").into());
        }
        let conf = self.conf.as_ref().unwrap().clone();

        if conf.unit.is_none() {
            return Err(format!("config unit section is not configured").into());
        }
        let unit = conf.unit.as_ref().unwrap();

        match &unit.wants {
            None => {},
            Some(w) => {
                self.parse_unit_relations(manager, w, UnitRelations::UnitWants)?;
            }
        }

        match &unit.before {
            None => {},
            Some(w) => {
                self.parse_unit_relations(manager, w, UnitRelations::UnitBefore)?;
            }
        }

        match &unit.after {
            None => {},
            Some(w) => {
                self.parse_unit_relations(manager, w, UnitRelations::UnitAfter)?;
            }
        }

        match &unit.requires {
            None => {},
            Some(w) => {
                self.parse_unit_relations(manager, w, UnitRelations::UnitRequires)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::manager::service::ServiceUnit;

    use super::*;

    #[test]
    fn  test_mangerplugin(){
        let mut unit_manager = UnitManager::new();
        unit_manager.init_lookup_path();

        let mut mp = manager::MangerLoader::new();
        mp.load_plugins(Box::new(unit_manager));
        assert_eq!(mp.run(),0);
    }

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
