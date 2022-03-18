extern crate toml;
use super::unit_config_parser::Conf;
use std::fs::File;
use std::collections::HashMap;
use std::io::prelude::*;
use std::io::{Error, ErrorKind};
use std::marker::PhantomData;
use serde::de;

pub trait ConfigParser<'a,T:de::Deserialize<'a>> {
    fn unit_file_load(&self,file_path: String) -> Result<T, Error>;
}


pub trait XXX{
    fn to_confs(&self) -> Confs;
}
struct ConfigParserImpl<T>(String,PhantomData<T>);


impl <'a,T>ConfigParser<'a,T> for ConfigParserImpl
where 
    T:de::Deserialize<'a>,
    {
        fn unit_file_load(&self,file_path: String) -> Result<T,Error>{
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
        
            let conf: T = match toml::from_str(&buf) {
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

struct ConfigParsers<T>{
    parsers_map:HashMap<String, T>
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