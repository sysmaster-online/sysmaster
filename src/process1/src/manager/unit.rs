use super::manager;
use super::service;
use std::io;
use std::fs;
use std::collections::{HashSet, HashMap};
use std::cell::RefCell;
use std::sync::Arc;
use std::fs::File;
use std::time::SystemTime;
use std::os::unix::fs::FileTypeExt;

use utils:: {time_util, path_lookup, unit_load};

enum UnitType {
    UnitService = 0,
    UnitMount,
    UnitSwap,
    UnitSocket,
    UnitTarget,
    UnitDevice,
    UnitAutomount,
    UnitTimer,
    UnitPath,
    UnitSlice,
    UnitScope,
    UnitTypeMax,
    UnitTypeInvalid,
    UnitTypeErrnoMax,
}

#[derive(PartialEq)]
pub enum UnitLoadState {
    UNIT_STUB = 0,
    UNIT_LOADED, 
    UNIT_NOT_FOUND,
    UNIT_ERROR,
    UNIT_MERGED, 
    UNIT_MASKED,
    _UNIT_LOAD_STATE_MAX,
    _UNIT_LOAD_STATE_INVALID = -1,
}

enum UnitDependency {
    UNIT_REQUIRES,
    UNIT_REQUIRES_OVERRIDABLE,
    UNIT_REQUISITE,
    UNIT_REQUISITE_OVERRIDABLE,
}

enum UnitNameFlags {
    UNIT_NAME_PLAIN =1,
    UNIT_NAME_INSTANCE = 2,
    UNIT_NAME_TEMPLATE = 4,
    UNIT_NAME_ANY = 1|2|4,
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
    unit_type: UnitType,
    load_state: UnitLoadState,
    unit_file_state: UnitFileState,
    id: String,
    instance: Option<String>,
    name: String,
    depencies: Vec<Unit>,
    desc: String,
    documnetation: String,
    fragment_path: String,
    source_path: String,
    fragment_mtine: u128,
    source_mtime: u128,
    dropin_mtime: u64,
    
    units_by_type: Vec<Unit>,
    has_requires_mounts_for: Vec<Unit>,
    load_queue: Vec<Unit>,
    dbus_queue: Vec<Unit>,
    cleanup_queue: Vec<Unit>,
    gc_queue: Vec<Unit>,
    cgroup_queue: Vec<Unit>,
    pids: HashSet<u64>,
    sigchldgen: u64,
    gc_marker: u64,
    deseialize_job: i32,
    load_error: i32,
    stop_when_unneeded: bool,
    refuse_manual_start: bool,
    allow_isolate: bool,
    ignore_on_isolate: bool,
    ignore_on_snapshot: bool,
    condition_result: bool,
    assert_result: bool,
    transient: bool,
    in_load_queue: bool,
    in_dubs_queue: bool,
    in_cleanup_queue: bool,
    in_gc_queue: bool,
    default_dependencies: bool,
    perpetual: bool,
    conf: Option<unit_load::Conf>,
    manager: Option<Arc<UnitManager>>,
}

pub trait UnitObj {
    fn init(&self){}
    fn done(&self){}
    fn load(&mut self) -> bool {false}
    fn coldplug(&self){}
    fn dump(&self){}
    fn start(&self){}
    fn stop(&self){}
    fn reload(&self){}
    fn kill(&self){}
    fn check_gc(&self)->bool;
    fn release_resources(&self){}
    fn check_snapshot(&self){}
    fn sigchld_events(&self, pid:u64,code:i32, status:i32){}
    fn reset_failed(&self){}
}
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
    pub fn new() -> Self {
        Unit{
            unit_type: UnitType::UnitTypeInvalid,
            load_state: UnitLoadState::UNIT_STUB,
            id: String::from(""),
            instance: Some(String::from("")),
            name: String::from(""),
            depencies: Vec::<Unit>::new(),
            desc: String::from(""),
            documnetation: null_str!(""),
            fragment_path: null_str!(""),
            source_path: null_str!(""),
            fragment_mtine: 0,
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
            default_dependencies: true,
            perpetual: false,
            unit_file_state: UnitFileState::UnitFileStateInvalid,
            manager: None,
            conf: None,
        }
    }
    pub fn set_manager(&mut self,manager: Option<Arc<UnitManager>>) {
        self.manager = manager;
    }
    
    pub fn set_load_state(&mut self, load_state: UnitLoadState){
        self.load_state = load_state;
    }

    pub fn unit_load_fragment_and_dropin(&mut self, frament_required: bool) -> bool {
        if !self.unit_load_fragment() {
            return false;
        }

        if self.load_state == UnitLoadState::UNIT_STUB {
                if frament_required {
                    return false;
                }
                self.load_state = UnitLoadState::UNIT_LOADED;
        }

        if !self.unit_load_dropin() {
            return false;
        }

        if !self.source_path.is_empty() {
            match fs::metadata(&self.source_path) {
                Ok(metadata) => match metadata.modified() {
                    Ok(time) => {
                        self.source_mtime = time_util::timespec_load(time);
                    },
                    _ => {
                        self.source_mtime = 0;
                    },
                }

                _ => {
                    self.source_mtime = 0;
                }
            }
        }

        return true;

    }

    fn unit_load_dropin(&mut self) -> bool { 
        todo!();
        true
    }

    fn unit_load_fragment(&mut self) -> bool {
        let r: isize = 0;
        let fragment: String = String::from("/usr/lib/systemd/system/dbus.service");

        if self.transient {
            self.load_state = UnitLoadState::UNIT_LOADED;
            return true;
        }

        if self.fragment_path != fragment {
            self.fragment_path = fragment;
        }

        if !self.fragment_path.is_empty() {
            let file = File::open(&self.fragment_path);
            let time: SystemTime;

            match file {
                Err(_e) => {println!("open file failed**********************");return false;},
                Ok(f) => 
                    match f.metadata(){
                        Err(e) => return false,
                        Ok(m) => 

			    if ((m.is_file() && m.len() <=0) || m.file_type().is_char_device()) {
                                self.load_state = UnitLoadState::UNIT_MASKED;
                                if self.perpetual {
                                    self.load_state = UnitLoadState::UNIT_LOADED;
                                }
                                self.fragment_mtine = 0;
                            } else {
                                self.load_state = UnitLoadState::UNIT_LOADED;
                                // self.fragment_mtine = time_util::timespec_load(time);
                                match unit_load::unit_file_load(self.fragment_path.to_string()) {
                                    Ok(conf) => self.conf = Some(conf),
                                    Err(e) => {
					return false;},
                                }
                            }
                    },
            }

          
            println!("fragmeng_mtime is: {}", self.fragment_mtine);

        }

        return true;

    }


    fn unit_file_build_name_map(&mut self) { 
        todo!()
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
            fragment_path: null_str!(""),
            source_path: null_str!(""),
            fragment_mtine: 0,
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
}

impl  UnitObj for MountUnit{
    fn init(&self) { todo!() }
    fn done(&self) { todo!() }
    fn load(&mut self) -> bool { todo!() }
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
}

fn unit_new(manager: Arc<UnitManager>, unit_type: UnitType) -> Box<dyn UnitObj> {
    let mut unit = Unit::new();
    unit.set_manager(Some(manager));
    match unit_type {
        UnitType::UnitService => {
            return Box::new(service::ServiceUnit::new(unit))
        },
        UnitType::UnitSocket => {
            return Box::new(service::ServiceUnit::new(unit))
        }
        UnitType::UnitTarget => Box::new(service::ServiceUnit::new(unit)),
        UnitType::UnitDevice => Box::new(service::ServiceUnit::new(unit)),
        UnitType::UnitDevice => Box::new(service::ServiceUnit::new(unit)),
        UnitType::UnitTimer => Box::new(service::ServiceUnit::new(unit)),
        (_) => Box::new(service::ServiceUnit::new(unit)),
        //TODO

        /*
        UnitPath,
        UnitSlice,
        UnitScope,
        UnitTypeMax,
        UnitTypeInvalid,
        UnitTypeErrnoMax,
        */
    }
}

pub struct UnitManager {
    units: RefCell<HashMap<String, RefCell<Box<dyn UnitObj>>>>,
    unit_id_map: HashMap<String, String>,
    unit_name_map: HashMap<String, String>,
    lookup_path: RefCell<path_lookup::LookupPaths>,
    unit_cache_timestamp_hash: u64,

}

impl UnitManager{
    pub fn new() -> Self{
        UnitManager {
            units: RefCell::new(HashMap::new()),
            unit_id_map: HashMap::new(),
            unit_name_map: HashMap::new(),
            unit_cache_timestamp_hash:0,
            lookup_path: RefCell::new(path_lookup::LookupPaths::new()),
        }
    }

    pub fn insert_unit(&self, name: String, unit: Box<dyn UnitObj>) {
	let mut units = self.units.borrow_mut();
	units.insert(name, RefCell::new(unit));
    }
}

impl  manager::Mangerobj for UnitManager  {
    fn init(&self){
    }
    
    fn load(&self){
        let mut units_vec = self.units.borrow_mut();
        
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


#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn  test_mangerplugin(){
        let unit_manger = UnitManager::new();
        let mut mp = manager::MangerLoader::new();
        mp.load_plugins(Box::new(unit_manger));
        assert_eq!(mp.run(),0);
    }

    #[test]
    fn  test_unit_load(){
        let unit_manager = UnitManager::new();
        let mut unit = unit_new(Arc::new(unit_manager), UnitType::UnitService);
        // unit_manager.insert_unit(String::from("systemd"), unit);
        assert_eq!(unit.load(), true);
    }
}
