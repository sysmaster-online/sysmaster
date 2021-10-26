use super::manager;
use std::collections::HashSet;
use std::cell::RefCell;
use std::sync::Arc;

enum UnitType {
    UNIT_SERVICE = 0,
    UNIT_SOCKET,
    UNIT_BUSNAME,
    UNIT_TARGET, 
    UNIT_SNAPSHOT,
    UNIT_DEVICE,
}

enum UnitLoadState {
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

struct Unit {
    unit_type: UnitType,
    load_state: UnitLoadState,
    id: String,
    instance: Option<String>,
    name: String,
    depencies: Vec<Unit>,
    desc: String,
    documnetation: String,
    fragment_path: String,
    source_path: String,
    fragment_mtine: u64,
    source_mtime: u64,
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
    manager: Option<Arc<UnitManager>>,
}

pub trait UnitObj {
    fn init(&self){}
    fn done(&self){}
    fn load(&self){}
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
            unit_type: UnitType::UNIT_SERVICE,
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
            manager:None,
        }
    }
    pub fn set_manager(&mut self,manger: Option<Arc<UnitManager>>){
        self.manager = manger;
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
fn load(&self) { todo!() }
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
 

pub struct UnitManager {
    units: RefCell<Vec<RefCell <Box<dyn UnitObj>>>>,
}

impl UnitManager{
    pub fn new() -> Self{
        UnitManager {units: RefCell::new(Vec::new())}
    }
}

impl  manager::Mangerobj for UnitManager  {
    fn init(&self){
    }
    
    fn load(&self){
        let mut units_vec = self.units.borrow_mut();
        
        let unit: Unit = Unit::new();

        units_vec.push(RefCell::new(Box::new(unit)));
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
}