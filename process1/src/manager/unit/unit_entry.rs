extern crate siphasher;

use std::any::Any;
use std::error::Error;
use std::fs;

use std::fs::File;
use std::rc::Rc;
use std::os::unix::fs::FileTypeExt;
use std::hash::Hasher;
use walkdir::WalkDir;
use std::hash::Hash;
use utils:: {time_util, path_lookup, unit_config_parser};
use super::unit_manager::*;

use std::collections::{HashMap, HashSet};
use std::cell::RefCell;

#[derive(Debug, PartialEq, Eq)]
pub enum UnitType {
    UnitService = 0,
    UnitTarget,
    UnitTypeMax,
    UnitTypeInvalid,
    UnitTypeErrnoMax,
}

impl Default for UnitType {
    fn default() -> Self { UnitType::UnitService }
}

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

#[derive(Default)]
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
    fn load(&mut self, m: &mut UnitManager) -> Result<(), Box<dyn Error>> {Ok(())}
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

#[derive(Eq, PartialEq)]
struct UnitObjWrapper(Rc<RefCell<Box<dyn UnitObj>>>);


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
#[macro_export]
macro_rules! unit_name_to_type{
    ($name:expr) => {
        match $name{
            "*.service" => UnitType::UnitService,
            "*.target" => UnitType::UnitTarget,
            _ => UnitType::UnitTypeInvalid,
        }
    };
}

impl Unit {
    pub fn new(name: &str) -> Self {
        Unit{
            unit_type: UnitType::UnitTypeInvalid,
            load_state: UnitLoadState::UnitStub,
            id: String::from(name),
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

    pub fn setId(&mut self, id:&str){
        self.id.clear();
        self.id.push_str(id);
    }

    pub fn setDesc(&mut self, desc:&str){
        self.desc.clear();
        self.desc.push_str(desc);
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

        let unit_type = unit_name_to_type!(unit_name);
        if unit_type == UnitType::UnitTypeInvalid {
            return Err(format!("invalid unit type of unit {}", unit_name).into());
        }
        let other = if let Some(_unit) = m.get_unit_on_name(unit_name) {
             return Ok(());
        } else {
            let unit = super::unit_new(unit_type, unit_name);
            let u = Rc::new(RefCell::new(unit));
            m.push_load_queue(u.clone());
            u
        };

        m.insert_unit(unit_name.to_string(), other.clone());
        self.updateDependencies(relation, other.clone());
        Ok(())
    }

    fn unit_load(&mut self, m: &mut UnitManager) -> Result<(), Box<dyn Error>> {
        if self.config_file_path.is_empty(){
            return Err(format!("config file path is empty").into());
        }else{
           match self.unit_config_load(){
               Ok(conf) =>{
                   self.parse(m);
               } 
               Err(e) =>{
                   return Err(e);
               }
           }
        }
        return Ok(());
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
}



pub trait ConfigParser {

    ///parse toml conf to unitObj who impl the trait
    /// conf is not need mutablility
    fn parse(&mut self, m: &mut UnitManager) -> Result<(), Box<dyn Error>> { Ok(())}
}

impl ConfigParser for Unit {
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
        Ok(())
    }
}


