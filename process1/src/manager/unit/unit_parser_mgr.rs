use std::error::Error;
use std::io::{Error as IoError, ErrorKind};
use std::{cell::RefCell, collections::HashMap, fs::File, io::Read};
use utils::config_parser::ConfigParser;
use utils::{config_parser::ConfigParse, u_config::Confs};
struct UnitParserMgr {
    config_parsers: RefCell<HashMap<String, ConfigParser>>,
}

impl UnitParserMgr {
    pub(super) fn new() -> Self {
        Self {
            config_parsers: RefCell::new(HashMap::new()),
        }
    }
    pub(super) fn register_parser(&self, unit_type: String) {
        let config_par = ConfigParser::new((&unit_type).to_string());
        self.config_parsers
            .borrow_mut()
            .insert(unit_type, config_par);
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

impl Default for UnitParserMgr {
    fn default() -> Self {
        Self::new()
    }
}

mod tests {

    use super::UnitParserMgr;
    use std::fs::File;
    use std::io::{Error, ErrorKind, Read};
    use utils::u_config::ConfValue;

    #[test]
    fn test_unit_parser_mgr_unit_file_load() -> Result<(), Error> {
        let file_path = "config.service";
        //let mut file = File::open(file_path).unwrap();
        //let mut buf = String::new();
        let ump = UnitParserMgr::new();
        ump.register_parser("service".to_string());
        let conf = ump.unit_file_parser("service", file_path);
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
