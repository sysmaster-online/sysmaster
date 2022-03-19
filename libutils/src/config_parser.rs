extern crate toml;
use std::fs::File;
use std::collections::HashMap;
use std::io::prelude::*;
use std::io::{Error, ErrorKind};
use std::marker::PhantomData;
use serde::de;
use super::u_config::Confs;
pub trait ConfigParse<'a,T:de::Deserialize<'a>> {
    fn unit_file_load(&self,file_content: &'a str) -> Result<T, Error>;
}


pub trait ConvertTo<T>{
    fn convertToT(&self) -> T;
}
struct ConfigParser<T>(String,PhantomData<T>);


impl <'a, T> ConfigParse<'a,T> for ConfigParser<T>
where 
    T:de::Deserialize<'a> + ConvertTo<Confs>,
    {
        fn unit_file_load(&self,file_content: &'a str) -> Result<T,Error>{        
            let conf: T = match toml::from_str(file_content) {
                Ok(conf) => conf,
                Err(_e) => {
                    println!("{}",_e);
                    return Err(Error::new(
                        ErrorKind::Other,
                        "translate string to struct failed{}",
                    ));
                }
            };   
            return Ok(conf);
        }
    }

struct ConfigParsers<T>{
    parsers_map:HashMap<String, T>
}




mod tests {
    
    use crate::unit_config_parser::Conf;
    use super::{ConfigParse,ConfigParser ,ConvertTo};
    use std::fs::File;
    use std::io::{Error, Read, ErrorKind};
    use std::marker::PhantomData;
    use crate::u_config::{Confs, ConfValue};
    impl ConvertTo<Confs> for Conf{
        fn convertToT(&self) -> Confs{
            let mut confs = Confs::new("service".to_string());
            match &self.unit{
                Some(u) =>{
                    let mut section = crate::u_config::Section::new("unit".to_string());
                    let conf = crate::u_config::Conf::new("description".to_string(),ConfValue::String((u.description.as_ref().unwrap()).to_string()));
                    section.addConf(conf);
                    confs.addSection(section);
                }
                None => {

                }
            }
            match &self.service{
                Some(u) =>{
                    let mut section = crate::u_config::Section::new("Service".to_string());
                    let mut vcs = Vec::new();
                    for item in u.exec_start.as_ref().unwrap().iter() {
                        vcs.push(ConfValue::String(item.to_string()));
                    }
                    let conf = crate::u_config::Conf::new("ExecStart".to_string(),ConfValue::Array(vcs));
                    section.addConf(conf);
                    confs.addSection(section);
                }
                None => {

                }
            }
            confs
        }
    }
    #[test]
    fn test_config_unit_file_load()->Result<(), Error> {
        let file_path = "config.service";
        let mut file = File::open(file_path).unwrap();
        let mut buf = String::new();
        match file.read_to_string(&mut buf) {
            Ok(s) => s,
            Err(_e) => {
                return Err(Error::new(ErrorKind::Other, "Error: Open file failed"));
            }
        };
        println!("int test {}",&buf);
        let a = ConfigParser::<Conf>("service".to_string(),PhantomData::default());
        let conf = a.unit_file_load(&buf);
        match conf {
            Ok(conf) =>{
                match &conf.install {
                    Some(c) => assert_eq!(c.wanted_by, Some("dbus".to_string())),
                    None => {
                        return Err(Error::new(ErrorKind::Other, "no install field"));
                    }
                };
                let confs = conf.convertToT();
                let v = confs.getSections();
                for item in v.iter() {
                    if item.getSectionName() == "service"{
                        let confs = item.getConfs();
                        for item_c in confs.iter() {
                            if item_c.get_key() =="ExecStart"{
                                match &item_c.get_values()[0]{
                                    ConfValue::String(str) => {
                                        println!("{}",str.to_string());
                                        assert_eq!("/usr/bin/reboot".to_string(),str.to_string())
                                    },
                                    ConfValue::Interger(_) => todo!(),
                                    ConfValue::Float(_) => todo!(),
                                    ConfValue::Boolean(_) => todo!(),
                                    ConfValue::Array(_) => todo!(),
                                }
                            }
                        }
                    }
                }
            }Err(e) => {
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