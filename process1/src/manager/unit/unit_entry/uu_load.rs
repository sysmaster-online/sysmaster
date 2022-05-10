use super::uu_config::{UeConfig, UnitConfigItem};
use crate::manager::data::{DataManager, UnitDepConf, UnitRelations};
use crate::manager::unit::uload_util::{
    UnitConfigParser, UnitFile, UnitParserMgr, SECTION_INSTALL, SECTION_UNIT,
};
use crate::manager::unit::unit_base::{self, JobMode, UnitLoadState, UnitType};
use crate::null_str;
use conf_option::{InstallConfOption, UnitConfOption};
use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;
use utils::unit_conf::{ConfValue, Confs};

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

    pub(super) fn get_unit_confs(&self) -> Result<Confs, Box<dyn Error>> {
        self.build_name_map();
        if let Some(p) = self.get_unit_file_path() {
            self.set_config_file_path(&p);
            let unit_type = unit_base::unit_name_to_type(&self.id); //best use of owner not name,need reconstruct
            match self
                .unit_conf_mgr
                .unit_file_parser(&unit_type.to_string(), &p)
            {
                Ok(confs) => return Ok(confs),
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

    fn parse(&self, confs: &Confs) -> Result<(), Box<dyn Error>> {
        let mut ud_conf = UnitDepConf::new(); // need get config from config database,and update depends here
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
                _ if key == InstallConfOption::Alias.to_string() => {
                    let result = set_base_config(conf_values);

                    if let ConfValue::String(_s) = result {
                        self.config.set(UnitConfigItem::UcItemInsAlias(_s));
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
                        self.config.set(UnitConfigItem::UcItemInsAlso(_s));
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

                _ if key == UnitConfOption::Desc.to_string() => {
                    let confvalue = set_base_config(conf.get_values());
                    if let ConfValue::String(str) = confvalue {
                        self.config.set(UnitConfigItem::UcItemDesc(str));
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
                        self.config.set(UnitConfigItem::UcItemDoc(str.to_string()));
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
                        self.config.set(UnitConfigItem::UcItemAllowIsolate(v));
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
                        self.config.set(UnitConfigItem::UcItemIgnoreOnIsolate(v));
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
                        self.config
                            .set(UnitConfigItem::UcItemOnSucJobMode(JobMode::JobReplace));
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
                        self.config
                            .set(UnitConfigItem::UcItemOnFailJobMode(JobMode::JobReplace));
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

    use crate::manager::data::UnitRelations;
    use core::fmt::{Display, Formatter, Result};

    pub(super) enum UnitConfOption {
        Desc,
        Documentation,
        Relation(UnitRelations),
        AllowIsolate,
        IgnoreOnIolate,
        OnSucessJobMode,
        OnFailureJobMode,
    }

    impl Display for UnitConfOption {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result {
            match self {
                UnitConfOption::Desc => write!(f, "Description"),
                UnitConfOption::Documentation => write!(f, "Documentation"),
                UnitConfOption::Relation(relation) => write!(f, "{}", relation),
                UnitConfOption::AllowIsolate => write!(f, "AllowIsolate"),
                UnitConfOption::IgnoreOnIolate => write!(f, "IgnoreOnIolate"),
                UnitConfOption::OnSucessJobMode => write!(f, "OnSucessJobMode"),
                UnitConfOption::OnFailureJobMode => write!(f, "OnFailureJobMode"),
            }
        }
    }

    impl From<UnitConfOption> for String {
        fn from(unit_conf_opt: UnitConfOption) -> Self {
            match unit_conf_opt {
                UnitConfOption::Desc => "Desc".into(),
                UnitConfOption::Documentation => "Documentation".into(),
                UnitConfOption::Relation(relation) => relation.into(),
                UnitConfOption::AllowIsolate => "AllowIsolate".into(),
                UnitConfOption::IgnoreOnIolate => "IgnoreOnIolate".into(),
                UnitConfOption::OnSucessJobMode => "OnSucessJobMode".into(),
                UnitConfOption::OnFailureJobMode => "OnFailureJobMode".into(),
            }
        }
    }

    pub(super) enum InstallConfOption {
        Alias,
        WantedBy,
        RequiredBy,
        Also,
        DefaultInstance,
    }

    impl Display for InstallConfOption {
        fn fmt(&self, fmt: &mut Formatter<'_>) -> Result {
            match self {
                InstallConfOption::Alias => write!(fmt, "Alias"),
                InstallConfOption::WantedBy => write!(fmt, "WantedBy"),
                InstallConfOption::RequiredBy => write!(fmt, "RequiredBy"),
                InstallConfOption::Also => write!(fmt, "Also"),
                InstallConfOption::DefaultInstance => write!(fmt, "DefaultInstance"),
            }
        }
    }

    impl From<InstallConfOption> for String {
        fn from(install_conf_opt: InstallConfOption) -> Self {
            match install_conf_opt {
                InstallConfOption::Alias => "Alias".into(),
                InstallConfOption::WantedBy => "WantedBy".into(),
                InstallConfOption::RequiredBy => "RequiredBy".into(),
                InstallConfOption::Also => "Also".into(),
                InstallConfOption::DefaultInstance => "DefaultInstance".into(),
            }
        }
    }
}
