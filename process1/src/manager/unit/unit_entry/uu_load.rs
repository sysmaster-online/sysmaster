use super::uu_config::{InstallConfOption, UnitConfOption};
use crate::manager::data::{DataManager, JobMode, UnitConfig, UnitRelations, UnitType};
use crate::manager::unit::unit_base::{self, UnitLoadState};
use crate::manager::unit::unit_parser_mgr::{SECTION_INSTALL, SECTION_UNIT};
use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;
use utils::unit_conf::{ConfValue, Confs};

use crate::null_str;

//#[derive(Debug)]
pub(super) struct UeLoad {
    // associated objects
    dm: Rc<DataManager>,
    // key
    id: String,

    // data
    load_state: RefCell<UnitLoadState>,
    config_file_path: RefCell<String>,
    config_file_mtime: RefCell<u128>,
    in_load_queue: RefCell<bool>,
    default_dependencies: bool,
    conf: Rc<RefCell<Option<Confs>>>,
}

impl UeLoad {
    pub(super) fn new(dm: Rc<DataManager>, id: String) -> UeLoad {
        UeLoad {
            dm,
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

    pub(super) fn set_in_load_queue(&self, t: bool) {
        *self.in_load_queue.borrow_mut() = t;
    }

    pub(super) fn set_config_file_path(&self, config_filepath: &str) {
        self.config_file_path.borrow_mut().clear();
        self.config_file_path.borrow_mut().push_str(config_filepath);
    }

    pub(super) fn in_load_queue(&self) -> bool {
        *self.in_load_queue.borrow() == true
    }

    fn parse_unit_relation(
        &self,
        unit_name: &str,
        relation: UnitRelations,
        u_config: &mut UnitConfig,
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
        u_config.add_deps((relation, String::from(unit_name)));
        Ok(())
    }

    fn parse_unit_relations(
        &self,
        confvalue: Vec<ConfValue>,
        relation: UnitRelations,
        u_config: &mut UnitConfig,
    ) -> Result<(), Box<dyn Error>> {
        for value in confvalue.iter() {
            if let ConfValue::String(val) = value {
                // zan shi zhe me chuli yinggai jiang unit quan bu jiexi chulai
                let result = self.parse_unit_relation(val, relation, u_config);
                if let Err(r) = result {
                    return Err(r);
                }
            }
        }
        Ok(())
    }
    pub(super) fn parse(&self, confs: &Confs) -> Result<(), Box<dyn Error>> {
        let mut u_config = UnitConfig::new(); // need get config from config database,and update depends here
        let unit_section = confs.get_section_by_name(SECTION_UNIT);
        if unit_section.is_none() {
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
        //let parse_unit_relations = |relation| {};
        let set_base_config = |mut conf_values: Vec<ConfValue>| {
            let conf_value = conf_values.pop();
            let err_str = "Config file format is error";
            let result = conf_value.map_or_else(
                || ConfValue::Error(err_str.to_string()),
                |v| {
                    if let ConfValue::String(str) = v {
                        ConfValue::String(str)
                    } else if let ConfValue::Boolean(v) = v {
                        ConfValue::Boolean(v)
                    } else {
                        ConfValue::Error(err_str.to_string())
                    }
                },
            );
            return result;
        };

        let confs = unit_install.unwrap().get_confs();
        for conf in confs.iter() {
            let key = conf.get_key();
            let conf_values = conf.get_values();
            match key {
                _ if key == InstallConfOption::WantedBy.to_string() => {
                    let result = self.parse_unit_relations(
                        conf_values,
                        UnitRelations::UnitWantsBy,
                        &mut u_config,
                    );
                    if let Err(r) = result {
                        return Err(r);
                    }
                }
                _ if key == InstallConfOption::RequiredBy.to_string() => {
                    let result = self.parse_unit_relations(
                        conf_values,
                        UnitRelations::UnitRequiresBy,
                        &mut u_config,
                    );
                    if let Err(r) = result {
                        return Err(r);
                    }
                }
                _ if key == InstallConfOption::Alias.to_string() => {
                    let result = set_base_config(conf_values);

                    if let ConfValue::String(_s) = result {
                        u_config.set_install_alias(_s);
                    } else {
                        if let ConfValue::Error(_s) = result {
                            return Err(format!(
                                "{},Section [{}] Conf[{}] value is not supported",
                                _s,
                                SECTION_INSTALL,
                                InstallConfOption::Alias
                            )
                            .into());
                        }
                    }
                }
                _ if key == InstallConfOption::Also.to_string() => {
                    let result = set_base_config(conf_values);
                    if let ConfValue::String(_s) = result {
                        u_config.set_install_also(_s);
                    } else {
                        if let ConfValue::Error(_s) = result {
                            return Err(format!(
                                "{},Section [{}] Conf[{}] value is not supported",
                                _s,
                                SECTION_INSTALL,
                                InstallConfOption::Alias
                            )
                            .into());
                        }
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
                        &mut u_config,
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
                        &mut u_config,
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
                        &mut u_config,
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
                        &mut u_config,
                    );
                    if let Err(r) = result {
                        return Err(r);
                    }
                }

                _ if key == UnitConfOption::Desc.to_string() => {
                    let confvalue = set_base_config(conf.get_values());
                    if let ConfValue::String(str) = confvalue {
                        u_config.set_desc(str);
                    } else {
                        if let ConfValue::Error(_s) = confvalue {
                            return Err(format!(
                                "{},Section [{}] Conf[{}] value is not supported",
                                _s,
                                SECTION_INSTALL,
                                UnitConfOption::Desc
                            )
                            .into());
                        }
                    }
                }
                _ if key == UnitConfOption::Documentation.to_string() => {
                    let confvalue = set_base_config(conf.get_values());
                    if let ConfValue::String(str) = confvalue {
                        u_config.set_documentation(str.to_string())
                    } else {
                        if let ConfValue::Error(_s) = confvalue {
                            return Err(format!(
                                "{},Section [{}] Conf[{}] value is not supported",
                                _s,
                                SECTION_UNIT,
                                UnitConfOption::Documentation
                            )
                            .into());
                        }
                    }
                }
                _ if key == UnitConfOption::AllowIsolate.to_string() => {
                    let confvalue = set_base_config(conf.get_values());
                    if let ConfValue::Boolean(v) = confvalue {
                        u_config.set_allow_isolate(v);
                    } else {
                        if let ConfValue::Error(_s) = confvalue {
                            return Err(format!(
                                "{},Section [{}] Conf[{}] value is not supported",
                                _s,
                                SECTION_UNIT,
                                UnitConfOption::AllowIsolate
                            )
                            .into());
                        }
                    }
                }
                _ if key == UnitConfOption::IgnoreOnIolate.to_string() => {
                    let confvalue = set_base_config(conf.get_values());
                    if let ConfValue::Boolean(v) = confvalue {
                        u_config.set_ignore_on_isolate(v);
                    } else {
                        if let ConfValue::Error(_s) = confvalue {
                            return Err(format!(
                                "{},Section [{}] Conf[{}] value is not supported",
                                _s,
                                SECTION_UNIT,
                                UnitConfOption::IgnoreOnIolate
                            )
                            .into());
                        }
                    }
                }
                _ if key == UnitConfOption::OnSucessJobMode.to_string() => {
                    let confvalue = set_base_config(conf.get_values());
                    if let ConfValue::String(_str) = confvalue {
                        u_config.set_on_success_job_mode(JobMode::JobReplace);
                    } else {
                        if let ConfValue::Error(_s) = confvalue {
                            return Err(format!(
                                "{},Section [{}] Conf[{}] value is not supported",
                                _s,
                                SECTION_UNIT,
                                UnitConfOption::OnSucessJobMode
                            )
                            .into());
                        }
                    }
                }
                _ if key == UnitConfOption::OnFailureJobMode.to_string() => {
                    let confvalue = set_base_config(conf.get_values());
                    if let ConfValue::Boolean(_str) = confvalue {
                        u_config.set_on_failure_job_mode(JobMode::JobReplace);
                    } else {
                        if let ConfValue::Error(_s) = confvalue {
                            return Err(format!(
                                "{},Section [{}] Conf[{}] value is not supported",
                                _s,
                                SECTION_UNIT,
                                UnitConfOption::OnFailureJobMode
                            )
                            .into());
                        }
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

        self.dm
            .insert_unit_config(self.id.clone(), Rc::new(u_config));
        Ok(())
    }

    pub(super) fn unit_load(&self, confs: &Confs) -> Result<(), Box<dyn Error>> {
        *self.in_load_queue.borrow_mut() = false;
        match self.parse(confs) {
            Ok(_conf) => {
                return Ok(());
            }
            Err(e) => {
                return Err(format!("{}", e.to_string()).into());
            }
        }
    }
}
