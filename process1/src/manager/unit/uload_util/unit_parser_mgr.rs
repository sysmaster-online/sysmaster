use std::default::Default;
use std::io::{Error as IoError, ErrorKind};
use std::{cell::RefCell, collections::HashMap, fs::File, io::Read};
use utils::config_parser::{ConfigParser};
use utils::unit_conf::{ConfFactory, Section, SectionType};
use utils::{config_parser::ConfigParse, unit_conf::Confs};

pub(in crate::manager::unit) const SECTION_UNIT: &str = "Unit";
pub(in crate::manager::unit) const SECTION_INSTALL: &str = "Install";
pub(in crate::manager::unit) struct UnitParserMgr<T> {
    config_parsers: RefCell<HashMap<String, T>>,
}

// the declaration "pub(self)" is for identification only.
impl<T> UnitParserMgr<T>
where
    T: ConfigParse,
{
    pub(self) fn new() -> Self {
        Self {
            config_parsers: RefCell::new(HashMap::new()),
        }
    }
    /*
     *Different unit have different conf format,so need default parser
     */
    pub(self) fn register_parser(&self, unit_type: String, config_parse: T) {
        if self.config_parsers.borrow().get(&unit_type).is_some() {
            return;
        }
        self.config_parsers
            .borrow_mut()
            .insert(unit_type, config_parse);
    }

    pub(in crate::manager::unit) fn unit_file_parser(
        &self,
        unit_type: &str,
        file_path: &str,
    ) -> Result<Confs, IoError> {
        
        let u_parser_map = self.config_parsers.borrow();
        let u_parser = u_parser_map.get(unit_type);
        let u_t_parser = match u_parser {
            Some(u_t_parser) => u_t_parser,
            None => {
                return Err(IoError::new(
                    ErrorKind::Other,
                    r#"unit parse not found,unit_type is error"#,
                ))
            }
        };
        let mut file = match File::open(file_path) {
            Err(why) => {
                return Err(IoError::new(
                    ErrorKind::Other,
                    format!("Error: Open file failed detail {}{}!", why, file_path),
                ))
            }
            Ok(file) => file,
        };
        let mut buf = String::new();
        match file.read_to_string(&mut buf) {
            Ok(s) => s,
            Err(why) => {
                return Err(IoError::new(
                    ErrorKind::Other,
                    format!("Error: read file buf error reason is {}", why),
                ));
            }
        };
        u_t_parser.toml_file_parse(&buf)
    }
}

pub(in crate::manager::unit) struct UnitConfigParser(ConfigParser<DefalutFactory>);

impl ConfigParse for UnitConfigParser {
    type Item = u32;
    fn toml_file_parse(&self, file_content: &str) -> Result<Confs, IoError> {
        self.0.toml_file_parse(file_content)
    }
}
impl UnitConfigParser {
    fn new(unit_type: String, section_name: String) -> Self {
        let default_factory = DefalutFactory(section_name);
        let config_parse = ConfigParser::new(unit_type.to_string(), default_factory);
        Self(config_parse)
    }
}

impl Default for UnitParserMgr<UnitConfigParser> {
    fn default() -> Self {
        let _self = Self::new();
        _self
    }
}

impl UnitParserMgr<UnitConfigParser> {
    pub(in crate::manager::unit) fn register_parser_by_private_section_name(
        &self,
        unit_type: String,
        section_name: String,
    ) {
        let config_parse = UnitConfigParser::new(unit_type.to_string(), section_name);
        self.register_parser(unit_type, config_parse);
    }
}

struct DefalutFactory(String);

impl ConfFactory for DefalutFactory {
    fn product_confs(&self) -> Confs {
        let private_section = Section::new((&self.0).to_string(), SectionType::PRIVATE);
        let unit_section = Section::new(SECTION_UNIT.to_string(), SectionType::PUB);
        let install_section = Section::new(SECTION_INSTALL.to_string(), SectionType::PUB);
        let mut confs = Confs::new("service".to_string());
        confs.add_section(unit_section);
        confs.add_section(private_section);
        confs.add_section(install_section);
        confs
    }
}

#[cfg(test)]
mod tests {
    use super::UnitParserMgr;
    use crate::manager::UnitType;

    use std::io::{Error, ErrorKind};
    use std::path::PathBuf;
    use utils::logger;
    use utils::unit_conf::ConfValue;

    #[test]
    fn test_unit_parser_mgr_unit_file_load() -> Result<(), Error> {
        logger::init_log_with_console("test", 4);
        let file_path = "examples/config.service";
        let mut config_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        config_path.push("../libutils");
        config_path.push(file_path);
        log::debug!("{:?}", config_path);
        //let mut file = File::open(file_path).unwrap();
        //let mut buf = String::new();
        //let service_factory = DefalutFactory("Service".to_string());
        let ump = UnitParserMgr::default();
        ump.register_parser_by_private_section_name(
            UnitType::UnitService.to_string(),
            "Service".to_string(),
        );
        let conf = ump.unit_file_parser("Service", config_path.to_str().unwrap());
        match conf {
            Ok(conf) => {
                let v = conf.get_sections();
                for item in v.iter() {
                    log::debug!("iter for section [{}]", item.get_section_name());
                    if item.get_section_name() == "Service" {
                        let confs = item.get_confs();
                        for item_c in confs.iter() {
                            if item_c.get_key() == "ExecStart" {
                                match &item_c.get_values()[0] {
                                    ConfValue::String(str) => {
                                        log::debug!(
                                            "key is [{}] confvalue [{}]",
                                            item_c.get_key(),
                                            str.to_string()
                                        );
                                        assert_eq!(
                                            "/usr/bin/echo 'test'".to_string(),
                                            str.to_string()
                                        );
                                        return Ok(());
                                    }
                                    ConfValue::Interger(_) => todo!(),
                                    ConfValue::Float(_) => todo!(),
                                    ConfValue::Boolean(_) => todo!(),
                                    ConfValue::Array(_) => todo!(),
                                    ConfValue::Error(_) => todo!(),
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                return Err(Error::new(ErrorKind::Other, e.to_string()));
            }
        };
        log::debug!("config file parse fiailed");
        assert_eq!("unit config file parser error".to_string(), "nil");
        Ok(())
    }
}
