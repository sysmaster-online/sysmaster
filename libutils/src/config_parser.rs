use crate::u_config::{Conf, ConfValue, Section};
use std::error::Error;
use std::io::{Error as IOError, ErrorKind};
use toml::Value;

use super::u_config::Confs;
pub trait ConfigParse {
    fn unit_file_parse(&self, file_content: &str) -> Result<Confs, IOError>;
}
pub struct ConfigParser(String);

impl ConfigParser {
    pub fn new(unit_type: String) -> Self {
        Self(unit_type)
    }
}
impl ConfigParse for ConfigParser {
    fn unit_file_parse(&self, file_content: &str) -> Result<Confs, IOError> {
        let conf: Value = match toml::from_str(file_content) {
            Ok(conf) => conf,
            Err(why) => {
                return Err(IOError::new(
                    ErrorKind::Other,
                    format!("translate string to struct failed{}", why),
                ));
            }
        };
        let error_info = Value::String("config file format is error".to_string());
        if let Some(v_table) = conf.as_table() {
            let mut confs = Confs::new(self.0.to_string());
            // must be a table not support for other format
            for key in v_table.keys() {
                let mut section: Section<Conf> = Section::new(key.to_string());
                if let Some(v_t_v_table) =
                    v_table.get(key).unwrap_or_else(|| &error_info).as_table()
                {
                    //must be a table not support for other format
                    for t_key in v_t_v_table.keys() {
                        if let Some(tmp) = v_t_v_table.get(t_key) {
                            if let Some(v_str) = tmp.as_str() {
                                section.add_conf(Conf::new(
                                    t_key.to_string(),
                                    ConfValue::String(v_str.to_string()),
                                ));
                            } else {
                                return Err(IOError::new(ErrorKind::Other, error_info.to_string()));
                            }
                        }
                    }
                } else {
                    return Err(IOError::new(ErrorKind::Other, error_info.to_string()));
                }
                confs.add_section(section);
            }
            return Ok(confs);
        } else {
            return Err(IOError::new(
                ErrorKind::Other,
                "config file format is error",
            ));
        }
    }
}

mod tests {

    use super::{ConfigParse, ConfigParser};
    use crate::u_config::ConfValue;
    use std::fs::File;
    use std::io::{Error, ErrorKind, Read};

    #[test]
    fn test_config_unit_file_load() -> Result<(), Error> {
        let file_path = "config.service";
        let mut file = File::open(file_path).unwrap();
        let mut buf = String::new();
        match file.read_to_string(&mut buf) {
            Ok(s) => s,
            Err(_e) => {
                return Err(Error::new(ErrorKind::Other, "Error: Open file failed"));
            }
        };
        let a = ConfigParser("service".to_string());
        let conf = a.unit_file_parse(&buf);
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

/*struct ServiceConfigParser;

impl<'a> ConfigParser<'a, Conf> for ServiceConfigParser
{
    fn unit_file_load(file_path: String) -> Result<Conf,Error>{
        let mut file = match File::open(file_path) {
            Ok(f) => f,
            Err(_e) => {
                return Err(Error::new(ErrorKind::Other, "Error: Open file failed"));
            }
        };

        let mut buf = String::new();
        match file.read_to_string(&mut buf) {
            Ok(s) => s,
            Err(_e) => {
                return Err(Error::new(ErrorKind::Other, "read file content failed"));
            }
        };

        let conf: Conf = match toml::from_str(&buf) {
            Ok(conf) => conf,
            Err(_e) => {
                return Err(Error::new(
                    ErrorKind::Other,
                    "translate string to struct failed",
                ));
            }
        };

        return Ok(conf);
    }
}
*/
