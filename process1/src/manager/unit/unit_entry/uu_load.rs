use super::uu_config::{
    InstallConfOption, UeConfig, UeConfigInstall, UeConfigUnit, UnitConfOption, UnitConfigItem,
};
use crate::manager::data::{DataManager, UnitDepConf, UnitRelations};
use crate::manager::unit::uload_util::UnitFile;
use crate::manager::unit::unit_base::{self, UnitLoadState, UnitType};
use crate::null_str;
use std::cell::RefCell;
use std::error::Error;
use std::rc::Rc;
use utils::config_parser::{unit_file_reader, ConfigParse};
//#[derive(Debug)]
pub(super) struct UeLoad {
    // associated objects
    dm: Rc<DataManager>,
    file: Rc<UnitFile>,
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
}

impl UeLoad {
    pub(super) fn new(
        dmr: &Rc<DataManager>,
        filer: &Rc<UnitFile>,
        config: &Rc<UeConfig>,
        id: String,
    ) -> UeLoad {
        UeLoad {
            dm: Rc::clone(dmr),
            config: Rc::clone(config),
            file: Rc::clone(filer),
            id,
            load_state: RefCell::new(UnitLoadState::UnitStub),
            config_file_path: RefCell::new(null_str!("")),
            config_file_mtime: RefCell::new(0),
            in_load_queue: RefCell::new(false),
            default_dependencies: true,
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

    pub(super) fn load_unit_confs(&self) -> Result<String, Box<dyn Error>> {
        self.build_name_map();
        if let Some(p) = self.get_unit_file_path() {
            self.set_config_file_path(&p);
            let _unit_type = unit_base::unit_name_to_type(&self.id); //best use of owner not name,need reconstruct
            match unit_file_reader(&p) {
                Ok(_vs) => {
                    let unit_parser = UeConfigUnit::builder_parser();
                    let config_unit = unit_parser.conf_file_parse(&_vs);
                    let install_parser = UeConfigInstall::builder_parser();
                    let install = install_parser.conf_file_parse(&_vs);
                    let ret1 = install.map(|_conf| {
                        self.config.set_installconf(_conf);
                    });
                    if ret1.is_err() {
                        return Err(format!(
                            "parse unit install config for unit [{}] err{:?}",
                            self.id,
                            ret1.err()
                        )
                        .into());
                    }
                    let ret2 = config_unit.map(|_unit| {
                        self.config.set_unitconf(_unit);
                    });
                    if ret2.is_err() {
                        return Err(format!(
                            "parse unit config for unit [{}] from file err{:?}",
                            self.id,
                            ret2.err()
                        )
                        .into());
                    }
                    let ret = self.parse();
                    if ret.is_err() {
                        return Err(format!(
                            "parse unit deps error [{}]  err{:?}",
                            self.id,
                            ret.err()
                        )
                        .into());
                    }

                    return Ok(_vs);
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
        confvalue: &String,
        relation: UnitRelations,
        ud_conf: &mut UnitDepConf,
    ) -> Result<(), Box<dyn Error>> {
        let deps = confvalue.split_whitespace();
        for dep in deps {
            // zan shi zhe me chuli yinggai jiang unit quan bu jiexi chulai
            let result = self.parse_unit_relation(dep, relation, ud_conf);
            if let Err(r) = result {
                return Err(r);
            }
        }
        Ok(())
    }

    fn parse(&self) -> Result<(), Box<dyn Error>> {
        let mut ud_conf = UnitDepConf::new(); // need get config from config database,and update depends hereW

        let mut parse_deps = |relation: UnitRelations, confvalue: &String| {
            self.parse_unit_relations(confvalue, relation, &mut ud_conf)
        };

        let wantedby = self
            .config
            .get_install_conf_value(InstallConfOption::WantedBy);
        if let UnitConfigItem::UcItemWantedBy(wb) = wantedby {
            let result = parse_deps(UnitRelations::UnitWantsBy, &wb);
            if let Err(r) = result {
                return Err(r);
            }
        }
        let requiredby = self
            .config
            .get_install_conf_value(InstallConfOption::RequiredBy);
        if let UnitConfigItem::UcItemRequiredBy(rb) = requiredby {
            let result = parse_deps(UnitRelations::UnitRequiresBy, &rb);
            if let Err(r) = result {
                return Err(r);
            }
        }

        let mut unit_deps = |relation: UnitRelations| {
            let unit_wants = self
                .config
                .get_unit_conf_value(UnitConfOption::Relation(relation));
            if let UnitConfigItem::UcItemRelation(relation, ws) = unit_wants {
                parse_deps(relation, &ws)
            } else {
                Ok(())
            }
        };

        if let Err(r) = unit_deps(UnitRelations::UnitWants) {
            return Err(r);
        } else if let Err(r) = unit_deps(UnitRelations::UnitBefore) {
            return Err(r);
        } else if let Err(r) = unit_deps(UnitRelations::UnitAfter) {
            return Err(r);
        } else if let Err(r) = unit_deps(UnitRelations::UnitRequires) {
            return Err(r);
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
