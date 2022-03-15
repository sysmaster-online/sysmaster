extern crate siphasher;

use std::any::Any;
use std::error::Error;

use std::fs::File;
use std::rc::Rc;
use std::os::unix::fs::FileTypeExt;
use std::hash::Hasher;

use std::hash::Hash;
use utils:: {time_util, unit_config_parser};
use super::unit_manager::*;
use super::unit_interface::UnitAction;

use std::collections::{HashMap, HashSet};
use std::cell::RefCell;



use std::ops::Deref;
use nix::sys::signal::Signal;
use nix::unistd::Pid;
use log;
use super::UnitType;

#[derive(Hash, PartialEq, Eq, Debug, Copy, Clone)]
pub enum UnitRelations {
    UnitRequires,
    UnitWants,
    UnitBindsTo,

    UnitRequiresBy,
    UnitWantsBy,
    UnitBoundBy,

    UnitBefore,
    UnitAfter,

    UnitTriggers,
    UnitTriggeredBy,
}

impl Default for UnitRelations {
    fn default()-> Self {UnitRelations::UnitRequires}
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

pub enum UnitState {
    UnitActive = 0,
    UnitReloading = 1,
    UnitInActive = 2,
    UnitFailed = 3,
    UnitActiving = 4,
    UnitDeactiving = 5,
    UnitStateMax = 6,
    UnitStateInvalid = -1,
}


impl Default for UnitLoadState {
    fn default() -> Self { UnitLoadState::UnitStub}
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

#[derive(Eq, PartialEq, Debug)]
pub enum UnitActiveState {
    UnitActive,
    UnitReloading,
    UnitInactive,
    UnitFailed,
    UnitActivating,
    UnitDeactiviting,
    UnitMaintenance,
}

pub enum KillOperation {
    KillTerminate,
    KillTerminateAndLog,
    KillRestart,
    KillKill,
    KillWatchdog,
    KillInvalid,
}

impl KillOperation {
    pub fn to_signal(&self) -> Signal {
        match *self {
            KillOperation::KillTerminate | KillOperation::KillTerminateAndLog |
                KillOperation::KillRestart => Signal::SIGTERM,
            KillOperation::KillKill => Signal::SIGKILL,
            KillOperation::KillWatchdog => Signal::SIGABRT,
            _ => Signal::SIGTERM,
        }
    }
}

#[derive(Default, Debug)]
pub struct Unit {
    pub unit_type: UnitType,
    pub load_state: UnitLoadState,
    pub id: String,
    pub name: String,
    dependencies: HashMap<UnitRelations, RefCell<HashSet<UnitObjWrapper>>>,
    desc: String,
    documnetation: String,
    config_file_path: String,
    config_file_mtime: u128,
    pids: HashSet<u64>,
    sigchldgen: u64,
    in_load_queue: bool,
    default_dependencies: bool,
    pub conf: Option<Rc<unit_config_parser::Conf>>,
    unit: Option<Rc<Box<dyn UnitAction>>>,
}

impl PartialEq for Unit {
     fn eq(&self, other: &Self) -> bool {
         self.unit_type == other.unit_type && self.id == other.id
     }
}

pub trait UnitObj: std::fmt::Debug {
    fn init(&self){}
    fn done(&self){}
    fn load(&mut self, _m: &mut UnitManager) -> Result<(), Box<dyn Error>> {Ok(())}
    fn coldplug(&self){}
    fn dump(&self){}
    fn start(&mut self, _m: &mut UnitManager){}
    fn stop(&mut self, _m: &mut UnitManager){}
    fn reload(&mut self, _m: &mut UnitManager){}
    
    fn kill(&self){}
    fn check_gc(&self)->bool;
    fn release_resources(&self){}
    fn check_snapshot(&self){}
    fn sigchld_events(&mut self,_m: &mut UnitManager, _pid:Pid,_code:i32, _status:Signal) {}
    fn reset_failed(&self){}
    fn trigger(&mut self, _other: Rc<RefCell<Box<dyn UnitObj>>>) {}
    fn in_load_queue(&self) -> bool;

    fn eq(&self, other: &dyn UnitObj) -> bool;
    fn hash(&self) -> u64;
    fn as_any(&self) -> &dyn Any;
    fn getDependencies(&self) -> Vec<(UnitRelations,Rc<RefCell<Box<dyn UnitObj>>>)>  { let v: Vec<(UnitRelations,Rc<RefCell<Box<dyn UnitObj>>>)> = Vec::new(); v}
}



#[macro_export]
macro_rules! declure_unitobj_plugin {
    ($unit_type:ty, $constructor:path) => {
        #[no_mangle]
        pub fn __unit_obj_create() -> *mut dyn $crate::manager::unit::UnitObj {
            let construcotr: fn() ->$unit_type = $constructor;

            let obj = construcotr();
            let boxed: Box<dyn $crate::manager::unit::UnitObj>  = Box::new(obj);
            Box::into_raw(boxed)
        }
    };
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

#[derive(Eq, PartialEq, Debug)]
struct UnitObjWrapper(Rc<RefCell<Box<dyn UnitObj>>>);

impl Deref for UnitObjWrapper {
    type Target = Rc<RefCell<Box<dyn UnitObj>>>;
    
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Hash for UnitObjWrapper {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.borrow().hash(state);
    }
}


#[macro_export]
macro_rules! null_str{
    ($name:expr) => {
        String::from($name)
    }
}

// #[macro_export]
// macro_rules! unit_name_to_type{
//     ($name:expr) => {
//         match $name{
//             "*.service" => UnitType::UnitService,
//             "*.target" => UnitType::UnitTarget,
//             _ => UnitType::UnitTypeInvalid,
//         }
//     };
// }

pub fn unit_name_to_type(unit_name: &str) -> UnitType {
    let words: Vec<&str> = unit_name.split(".").collect();
    match words[words.len()-1] {
        "service" => UnitType::UnitService,
        "target" => UnitType::UnitTarget,
        _ => UnitType::UnitTypeInvalid,
    }
}

impl Unit {
    pub fn new(name: &str) -> Self {
        Unit{
            unit_type: UnitType::UnitTypeInvalid,
            load_state: UnitLoadState::UnitStub,
            id: String::from(name),
            name: String::from(""),
            dependencies: HashMap::new(),
            desc: String::from(""),
            documnetation: null_str!(""),
            config_file_path: null_str!(""),
            config_file_mtime: 0,
            pids: HashSet::<u64>::new(),
            sigchldgen: 0,
            in_load_queue: false,
            default_dependencies: true,
            conf: None,
            unit: None,
        }
    }

    pub fn set_load_state(&mut self, load_state: UnitLoadState){
        self.load_state = load_state;
    }

    pub fn setId(&mut self, id:&str){
        self.id.clear();
        self.id.push_str(id);
    }

    pub fn setDesc(&mut self, desc:&str){
        self.desc.clear();
        self.desc.push_str(desc);
    }

    pub fn set_doc(&mut self, doc:&str){
        self.documnetation.clear();
        self.documnetation.push_str(doc);
    }

    pub fn set_In_load_queue(&mut self, t:bool){
        self.in_load_queue = t;
    }


    pub fn addPids(&mut self, pid:u64) -> bool{
        self.pids.insert(pid)
    }

    pub fn removePids(&mut self, pid:u64) -> bool{
        self.pids.remove(&pid)
    }

    pub fn setConfig_file_path(&mut self, configFilePath: &str){
        self.config_file_path.clear();
        self.config_file_path.push_str(configFilePath);
    }

    pub fn updateDependencies(&mut self,relation: UnitRelations,unit: Rc<RefCell<Box<dyn UnitObj>>>) -> Result<(), Box<dyn Error>>{
        let _relation = relation.clone();
        if !self.dependencies.contains_key(&_relation){
            self.dependencies.insert(_relation,RefCell::new(HashSet::new()));
        }
        let mut depends = self.dependencies.get(&relation).unwrap().borrow_mut();
        depends.insert(UnitObjWrapper(unit.clone()));//todo!() is need clone ?
        Ok(())
    }

    fn getDependencies(&self) -> Vec<(UnitRelations,Rc<RefCell<Box<dyn UnitObj>>>)> {
        let mut v_dependencies: Vec<(UnitRelations,Rc<RefCell<Box<dyn UnitObj>>>)>  = Vec::new();
        for (k_r,v_set) in self.dependencies.iter(){
            for v_u in v_set.borrow().iter() {
                v_dependencies.push((*k_r,Rc::clone(&v_u.0)));
            }
        }
        v_dependencies
    }

    fn unit_config_load(&mut self) -> Result<(), Box<dyn Error>> {
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
            };
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
        for unit in units{
            self.parse_unit_relation(manager, unit, relation)?;
        }
        Ok(())
    }

    fn parse_unit_relation(&mut self, m: &mut UnitManager, unit_name: &str, relation: UnitRelations) -> Result<(), Box<dyn Error>> {
        log::debug!("parse relation unit relation name is {}, relation is {:?}", unit_name, relation);

        let unit_type = unit_name_to_type(unit_name);
        if unit_type == UnitType::UnitTypeInvalid {
            return Err(format!("invalid unit type of unit {}", unit_name).into());
        }
        let other = if let Some(_unit) = m.get_unit_on_name(unit_name) {
             return Ok(());
        } else {
            let unit = match super::unit_new(unit_type, unit_name) {
                Ok(u) => u,
                Err(e) => return Err(e),
            };
            let u = Rc::new(RefCell::new(unit));
            m.push_load_queue(u.clone());
            u
        };

        m.insert_unit(unit_name.to_string(), other.clone());
        self.updateDependencies(relation, other.clone());
        Ok(())
    }

    fn unit_load(&mut self, m: &mut UnitManager) -> Result<(), Box<dyn Error>> {
        self.in_load_queue = false;
        self.build_name_map(m);

        if let Some(p) = self.get_unit_file_path(m) {
            self.setConfig_file_path(&p);
        }
        
        if self.config_file_path.is_empty(){
            return Err(format!("config file path is empty").into());
        }

        match self.unit_config_load(){
            Ok(conf) =>{
                self.parse(m);
            } 
            Err(e) =>{
                return Err(e);
            }
        }
        return Ok(());
    }

    pub fn in_load_queue(&self) -> bool {
        self.in_load_queue == true
    }

    pub fn notify(&mut self, manager: &mut UnitManager, original_state: UnitActiveState, new_state: UnitActiveState) {
        if original_state != new_state {
            log::debug!("unit active state change from: {:?} to {:?}", original_state, new_state);
        }

        match self.dependencies.get(&UnitRelations::UnitTriggeredBy) {
            Some(d) => {
                d.borrow_mut().iter().for_each(
                    |u| u.borrow_mut().trigger(manager.get_unit_on_name(&self.id).unwrap().clone())
                );
            },
            None => {},
        }
    }


 }

impl UnitObj for Unit{
    fn init(&self){

    }
    fn load(&mut self, m: &mut UnitManager) -> Result<(), Box<dyn Error>>{
        self.unit_load(m);
        Ok(())
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

    fn in_load_queue(&self) -> bool {
        self.in_load_queue()
    }

    fn getDependencies(&self) -> Vec<(UnitRelations,Rc<RefCell<Box<dyn UnitObj>>>)>  {
        self.getDependencies()
    }
}



pub trait ConfigParser {

    ///parse toml conf to unitObj who impl the trait
    /// conf is not need mutablility
    fn parse(&mut self, m: &mut UnitManager) -> Result<(), Box<dyn Error>> { Ok(())}
}

impl  ConfigParser  for Unit  {
    fn parse(&mut self,  m: &mut UnitManager) -> Result<(), Box<dyn Error>> {

        // impl ugly
        if self.conf.is_none() {
            return Err(format!("load config file failed").into());
        }
        let p_conf = self.conf.as_ref().unwrap().clone();

        if p_conf.unit.is_none() {
            return Err(format!("config unit section is not configured").into());
        }
        let unit = p_conf.unit.as_ref().unwrap();

        match &unit.wants {
            None => {},
            Some(w) => {
                self.parse_unit_relations(m, w, UnitRelations::UnitWants)?;
            }
        }

        match &unit.before {
            None => {},
            Some(w) => {
                self.parse_unit_relations(m, w, UnitRelations::UnitBefore)?;
            }
        }

        match &unit.after {
            None => {},
            Some(w) => {
                self.parse_unit_relations(m, w, UnitRelations::UnitAfter)?;
            }
        }

        match &unit.requires {
            None => {},
            Some(w) => {
                self.parse_unit_relations(m, w, UnitRelations::UnitRequires)?;
            }
        }

        match &unit.description {
            None => {},
            Some(des) => {
                self.setDesc(des);
            }
        }

        match &unit.documentation {
            None => {},
            Some(doc) => {
                self.set_doc(doc);
            }
        }
        Ok(())
    }
}


