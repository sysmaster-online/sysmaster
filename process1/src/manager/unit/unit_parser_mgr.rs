use std::default::Default;
use std::error::Error;
use std::io::{Error as IoError, ErrorKind};
use std::{cell::RefCell, collections::HashMap, fs::File, io::Read};
use utils::config_parser::ConfigParser;
use utils::unit_conf::ConfFactory;
use utils::{config_parser::ConfigParse, unit_conf::Confs};
pub struct UnitParserMgr<T> {
    config_parsers: RefCell<HashMap<String, T>>,
}

impl<T> UnitParserMgr<T>
where
    T: ConfigParse,
{
    pub(super) fn new() -> Self {
        Self {
            config_parsers: RefCell::new(HashMap::new()),
        }
    }
    /*
     *Different unit have different conf format,so need default parser
     */
    pub(super) fn register_parser(&self, unit_type: String, config_parse: T) {
        self.config_parsers
            .borrow_mut()
            .insert(unit_type, config_parse);
    }

    pub(super) fn unit_file_parser(
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
        u_t_parser.unit_file_parse(&buf)
    }
}

impl<T> Default for UnitParserMgr<ConfigParser<T>>
where
    T: ConfFactory,
{
    fn default() -> Self {
        let _self = Self::new();
        _self
    }
}

mod tests {

    use super::UnitParserMgr;
    use std::fs::File;
    use std::io::{Error, ErrorKind, Read};
    use std::path::PathBuf;
    use utils::config_parser::ConfigParser;
    use utils::unit_conf::{ConfFactory, ConfValue, Confs, Section};
    struct ServiceFactory;
    impl ConfFactory for ServiceFactory {
        fn product_confs(&self) -> Confs {
            let mut confs = Confs::new("service".to_string());
            let unit_section = Section::new("unit".to_string());
            let service_section = Section::new("service".to_string());
            let install_section = Section::new("install".to_string());
            confs.add_section(unit_section);
            confs.add_section(service_section);
            confs.add_section(install_section);
            confs
        }
    }
    #[test]
    fn test_unit_parser_mgr_unit_file_load() -> Result<(), Error> {
        let file_path = "examples/config.service";
        let mut config_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        config_path.push("../libutils");
        config_path.push(file_path);
        println!("{:?}", config_path);
        //let mut file = File::open(file_path).unwrap();
        //let mut buf = String::new();
        let service_factory = ServiceFactory;
        let ump = UnitParserMgr::default();
        let config_parse = ConfigParser::new("service".to_string(), service_factory);
        ump.register_parser("service".to_string(), config_parse);
        let conf = ump.unit_file_parser("service", config_path.to_str().unwrap());
        match conf {
            Ok(conf) => {
                let v = conf.get_sections();
                for item in v.iter() {
                    if item.get_section_name() == "service" {
                        let confs = item.get_confs();
                        for item_c in confs.iter() {
                            if item_c.get_key() == "ExecStart" {
                                match &item_c.get_values()[0] {
                                    ConfValue::String(str) => {
                                        println!("{}", str.to_string());
                                        assert_eq!("/usr/bin/reboot".to_string(), str.to_string())
                                    }
                                    ConfValue::Interger(_) => todo!(),
                                    ConfValue::Float(_) => todo!(),
                                    ConfValue::Boolean(_) => todo!(),
                                    ConfValue::Array(_) => todo!(),
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
        Ok(())
    }
}
