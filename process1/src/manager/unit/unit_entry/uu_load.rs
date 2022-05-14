use super::uu_config::{UeConfig, UnitConfigItem, UeConfigUnit, UeConfigInstall};
use crate::manager::data::{DataManager, UnitDepConf, UnitRelations};
use crate::manager::unit::uload_util::{
    UnitConfigParser, UnitFile, UnitParserMgr, SECTION_INSTALL, SECTION_UNIT,
};
use crate::manager::unit::unit_base::{self, JobMode, UnitLoadState, UnitType};
use crate::null_str;
use conf_option::{InstallConfOption, UnitConfOption};
use toml::Value;
use utils::config_parser::ConfigParse;
use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;
use utils::unit_conf::{ConfValue, Confs};
use super::uu_config_parse;
//#[derive(Debug)]
pub(super) struct UeLoad {
    // associated objects
    dm: Rc<DataManager>,
    file: Rc<UnitFile>,
    unit_conf_mgr: Rc<UnitParserMgr<UnitConfigParser>>,
    config: Rc<UeConfig>,

    // owned objects
    /* key */
    id: String,
    /* data */
    load_state: RefCell<UnitLoadState>,
    config_file_path: RefCell<String>,
    config_file_mtime: RefCell<u128>,
    in_load_queue: RefCell<bool>,
    default_dependencies: bool,
    conf: Rc<RefCell<Option<Confs>>>,
}

impl UeLoad {
    pub(super) fn new(
        dmr: &Rc<DataManager>,
        filer: &Rc<UnitFile>,
        unit_conf_mgrr: &Rc<UnitParserMgr<UnitConfigParser>>,
        configr: &Rc<UeConfig>,
        id: String,
    ) -> UeLoad {
        UeLoad {
            dm: Rc::clone(dmr),
            config: Rc::clone(configr),
            file: Rc::clone(filer),
            unit_conf_mgr: Rc::clone(unit_conf_mgrr),

            id,
            load_state: RefCell::new(UnitLoadState::UnitStub),
            config_file_path: RefCell::new(null_str!("")),
            config_file_mtime: RefCell::new(0),
            in_load_queue: RefCell::new(false),
            default_dependencies: true,
            conf: Rc::new(RefCell::new(None)),
        }
    }

    pub(super) fn set_load_state(&self, load_state: UnitLoadState) {
        *self.load_state.borrow_mut() = load_state;
    }
    
    pub(super) fn get_load_state(&self) -> UnitLoadState {
        let state = self.load_state.clone();
        state.into_inner()
    }

    pub(super) fn set_in_load_queue(&self, t: bool) {
        *self.in_load_queue.borrow_mut() = t;
    }

    fn set_config_file_path(&self, config_filepath: &str) {
        self.config_file_path.borrow_mut().clear();
        self.config_file_path.borrow_mut().push_str(config_filepath);
    }

    pub(super) fn in_load_queue(&self) -> bool {
        *self.in_load_queue.borrow() == true
    }


    pub(super) fn load_unit_confs(&self) -> Result<Value, Box<dyn Error>> {
        self.build_name_map();
        if let Some(p) = self.get_unit_file_path() {
            self.set_config_file_path(&p);
            let _unit_type = unit_base::unit_name_to_type(&self.id); //best use of owner not name,need reconstruct
            match uu_config_parse::unit_file_parser(&p)
            {
                Ok(values) => {
                    let str_buf = values.to_string();
                    let unit_parser = UeConfigUnit::builder_paser();
                    let config_unit = unit_parser.conf_file_parser(&str_buf);
                    let install_parser = UeConfigInstall::builder_paser();
                    let install = install_parser.conf_file_parser(&str_buf);
                    let ret1 = install.map(|_conf|{
                        self.config.set_installconf(_conf.unwrap());
                    });
                    if ret1.is_err(){
                        return Err(format!("parse unit install config for unit [{}] err{:?}",self.id,ret1.err()).into());
                    }
                    let ret2 = config_unit.map(|_unit|{
                        self.config.set_unitconf(_unit.unwrap());
                    });
                    if ret2.is_err(){
                        return Err(format!("parse unit config for unit [{}]  err{:?}",self.id,ret2.err()).into());
                    }
                    return Ok(values);
                }
                Err(e) => {
                    return Err(format!("{}", e.to_string()).into());
                }
            }
        } else {
            return Err(format!("Unit[ {}] file Not found", self.id).into());
        }
    }

    fn parse_unit_relation(
        &self,
        unit_name: &str,
        relation: UnitRelations,
        ud_conf: &mut UnitDepConf,
    ) -> Result<(), Box<dyn Error>> {
        log::debug!(
            "parse relation unit relation name is {}, relation is {:?}",
            unit_name,
            relation.to_string()
        );

        let unit_type = unit_base::unit_name_to_type(unit_name);
        if unit_type == UnitType::UnitTypeInvalid {
            return Err(format!("invalid unit type of unit {}", unit_name).into());
        }
        ud_conf.deps.push((relation, String::from(unit_name)));
        Ok(())
    }

    fn parse_unit_relations(
        &self,
        confvalue: Vec<ConfValue>,
        relation: UnitRelations,
        ud_conf: &mut UnitDepConf,
    ) -> Result<(), Box<dyn Error>> {
        for value in confvalue.iter() {
            if let ConfValue::String(val) = value {
                // zan shi zhe me chuli yinggai jiang unit quan bu jiexi chulai
                let result = self.parse_unit_relation(val, relation, ud_conf);
                if let Err(r) = result {
                    return Err(r);
                }
            }
        }
        Ok(())
    }

    fn parse(&self) -> Result<(), Box<dyn Error>> {
        let mut ud_conf = UnitDepConf::new(); // need get config from config database,and update depends here
        let wants = self.config.get_wants();
        let unit_section = confs.get_section_by_name(SECTION_UNIT);
        if unit_section.is_none(){
            return Err(format!(
                "config file format is error,section [{}]  not found",
                SECTION_UNIT
            )
            .into());
        }
        let unit_install = confs.get_section_by_name(SECTION_INSTALL);
        if unit_install.is_none() {
            return Err(format!(
                "Config file format is error,Section [{}] not found",
                SECTION_INSTALL
            )
            .into());
        }
        let confs = unit_install.unwrap().get_confs();
        for conf in confs.iter() {
            let key = conf.get_key();
            let conf_values = conf.get_values();
            match key {
                _ if key == InstallConfOption::WantedBy.to_string() => {
                    let result = self.parse_unit_relations(
                        conf_values,
                        UnitRelations::UnitWantsBy,
                        &mut ud_conf,
                    );
                    if let Err(r) = result {
                        return Err(r);
                    }
                }
                _ if key == InstallConfOption::RequiredBy.to_string() => {
                    let result = self.parse_unit_relations(
                        conf_values,
                        UnitRelations::UnitRequiresBy,
                        &mut ud_conf,
                    );
                    if let Err(r) = result {
                        return Err(r);
                    }
                }
                _ => {
                    return Err(format!(
                        "config file of {}  section format is error",
                        SECTION_INSTALL
                    )
                    .into());
                }
            }
        }
        let confs = unit_section.unwrap().get_confs();
        for conf in confs.iter() {
            let key = conf.get_key();
            match key {
                _ if key == UnitConfOption::Relation(UnitRelations::UnitWants).to_string() => {
                    let confvalue = conf.get_values();
                    let result = self.parse_unit_relations(
                        confvalue,
                        UnitRelations::UnitWants,
                        &mut ud_conf,
                    );
                    if let Err(r) = result {
                        return Err(r);
                    }
                }
                _ if key == UnitConfOption::Relation(UnitRelations::UnitBefore).to_string() => {
                    let confvalue = conf.get_values();
                    let result = self.parse_unit_relations(
                        confvalue,
                        UnitRelations::UnitBefore,
                        &mut ud_conf,
                    );
                    if let Err(r) = result {
                        return Err(r);
                    }
                }
                _ if key == UnitConfOption::Relation(UnitRelations::UnitAfter).to_string() => {
                    let confvalue = conf.get_values();
                    let result = self.parse_unit_relations(
                        confvalue,
                        UnitRelations::UnitBefore,
                        &mut ud_conf,
                    );
                    if let Err(r) = result {
                        return Err(r);
                    }
                }
                _ if key == UnitConfOption::Relation(UnitRelations::UnitRequires).to_string() => {
                    let confvalue = conf.get_values();
                    let result = self.parse_unit_relations(
                        confvalue,
                        UnitRelations::UnitRequires,
                        &mut ud_conf,
                    );
                    if let Err(r) = result {
                        return Err(r);
                    }
                }
                _ => {
                    return Err(format!(
                        "config file of {}  section format is error",
                        SECTION_UNIT
                    )
                    .into());
                }
            }
        }

        self.dm.insert_ud_config(self.id.clone(), ud_conf);
        Ok(())
    }

    fn build_name_map(&self) {
        self.file.build_name_map();
    }

    fn get_unit_file_path(&self) -> Option<String> {
        match self.file.get_unit_file_path(&self.id) {
            Some(v) => return Some(v.to_string()),
            None => {
                log::error!("not find unit file {}", &self.id);
                None
            }
        }
    }
}

mod conf_option {

   


    
   
}
