use std::error::Error;
use std::rc::Rc;
use std::cell::RefCell;
use std::fs::File;
use std::os::unix::fs::FileTypeExt;
use utils:: {time_util, unit_config_parser};
use crate::manager::data::*;
use crate::manager::unit::unit_base::{self, UnitLoadState};
use crate::manager::unit::unit_manager::{UnitManager};

use crate::null_str;

#[derive(Debug)]
pub struct UeLoad {
    dm: Rc<DataManager>,

    // key
    id: String,

    // data
    load_state: RefCell<UnitLoadState>,
    config_file_path: RefCell<String>,
    config_file_mtime: RefCell<u128>,
    in_load_queue: RefCell<bool>,
    default_dependencies: bool,
    conf: RefCell<Option<Rc<unit_config_parser::Conf>>>,
}

impl UeLoad {
    pub fn new(dm:Rc<DataManager>, id:String) -> UeLoad {
        UeLoad {
            dm,
            id,
            load_state: RefCell::new(UnitLoadState::UnitStub),
            config_file_path: RefCell::new(null_str!("")),
            config_file_mtime: RefCell::new(0),
            in_load_queue: RefCell::new(false),
            default_dependencies: true,
            conf: RefCell::new(None),
        }
    }

    pub fn set_load_state(&mut self, load_state: UnitLoadState){
        *self.load_state.borrow_mut() = load_state;
    }

    pub fn set_In_load_queue(&self, t:bool){
        *self.in_load_queue.borrow_mut() = t;
    }

    pub fn setConfig_file_path(&self, configFilePath: &str){
        self.config_file_path.borrow_mut().clear();
        self.config_file_path.borrow_mut().push_str(configFilePath);
    }

    pub fn get_conf(&self) -> Option<Rc<unit_config_parser::Conf>> {
        self.conf.borrow().as_ref().cloned()
    }

    fn unit_config_load(&self) -> Result<(), Box<dyn Error>> {
        if self.config_file_path.borrow().is_empty() {
            return Err(format!("config file path is empty").into());
        }

        let file = File::open(self.config_file_path.clone().into_inner())?;
        let meta = file.metadata()?;

        if (meta.is_file() && meta.len() <=0) || meta.file_type().is_char_device() {
            *self.load_state.borrow_mut() = UnitLoadState::UnitLoaded;
            *self.config_file_mtime.borrow_mut() = 0;
        } else {
            let mtime = meta.modified()?;
            *self.config_file_mtime.borrow_mut() = time_util::timespec_load(mtime);
            *self.load_state.borrow_mut() = UnitLoadState::UnitLoaded;
            match unit_config_parser::unit_file_load(self.config_file_path.borrow().to_string()) {
                Ok(conf) => *self.conf.borrow_mut() = Some(Rc::new(conf)),
                Err(e) => {
                    return Err(format!("file load err {:?}", e).into());
                },
            };
            log::debug!("config file mtime is: {}", self.config_file_mtime.borrow());
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
                log::error!("not find unit file {}", &self.id);
                None
            },
        }
    }

    pub fn unit_load(&self, m: &mut UnitManager) -> Result<(), Box<dyn Error>> {
        *self.in_load_queue.borrow_mut() = false;
        self.build_name_map(m);

        if let Some(p) = self.get_unit_file_path(m) {
            self.setConfig_file_path(&p);
        }
        
        if self.config_file_path.borrow().is_empty(){
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
        *self.in_load_queue.borrow_mut() == true
    }

    fn parse_unit_relations(&self, manager: &mut UnitManager, units: &str, relation: UnitRelations, u_config: &mut UnitConfig) -> Result<(), Box<dyn Error>> {
        let units = units.split_whitespace();
        for unit in units{
            self.parse_unit_relation(manager, unit, relation, u_config)?;
        }
        Ok(())
    }

    fn parse_unit_relation(&self, m: &mut UnitManager, unit_name: &str, relation: UnitRelations, u_config: &mut UnitConfig) -> Result<(), Box<dyn Error>> {
        log::debug!("parse relation unit relation name is {}, relation is {:?}", unit_name, relation);

        let unit_type = unit_base::unit_name_to_type(unit_name);
        if unit_type == UnitType::UnitTypeInvalid {
            return Err(format!("invalid unit type of unit {}", unit_name).into());
        }
        let other = if let Some(_unit) = m.units.get_unit_on_name(unit_name) {
             return Ok(());
        } else {
            let unit = match crate::manager::unit::unit_new(Rc::clone(&self.dm), unit_type, unit_name) {
                Ok(u) => u,
                Err(e) => return Err(e),
            };
            let u = Rc::new(RefCell::new(unit));
            m.push_load_queue(u.clone());
            u
        };

        m.units.insert_unit(unit_name.to_string(), other.clone());
        u_config.deps.push((relation, String::from(unit_name)));
        Ok(())
    }

    pub fn parse(&self,  m: &mut UnitManager) -> Result<(), Box<dyn Error>> {
        let mut u_config = UnitConfig::new();

        // impl ugly
        if self.conf.borrow().is_none() {
            return Err(format!("load config file failed").into());
        }
        let p_conf = self.conf.borrow().as_ref().unwrap().clone();

        if p_conf.unit.is_none() {
            return Err(format!("config unit section is not configured").into());
        }
        let unit = p_conf.unit.as_ref().unwrap();

        match &unit.wants {
            None => {},
            Some(w) => {
                self.parse_unit_relations(m, w, UnitRelations::UnitWants, &mut u_config)?;
            }
        }

        match &unit.before {
            None => {},
            Some(w) => {
                self.parse_unit_relations(m, w, UnitRelations::UnitBefore, &mut u_config)?;
            }
        }

        match &unit.after {
            None => {},
            Some(w) => {
                self.parse_unit_relations(m, w, UnitRelations::UnitAfter, &mut u_config)?;
            }
        }

        match &unit.requires {
            None => {},
            Some(w) => {
                self.parse_unit_relations(m, w, UnitRelations::UnitRequires, &mut u_config)?;
            }
        }

        match &unit.description {
            None => {},
            Some(des) => {
                u_config.desc = String::from(des);
            }
        }

        match &unit.documentation {
            None => {},
            Some(doc) => {
                u_config.documnetation = String::from(doc);
            }
        }

        self.dm.insert_unit_config(self.id.clone(), u_config);
        Ok(())
    }
}



