use std::fs;
use std::io::{Error as IOError, ErrorKind};

use std::path::PathBuf;
use std::{fs::File, io::Read};
pub trait ConfigParse {
    type Item;
    fn conf_file_parse(&self, _file_content: &str) -> Result<Self::Item, IOError> {
        Err(IOError::new(
            ErrorKind::Other,
            format!("config file not Contain Section{}", "Service"),
        ))
    }
}

pub fn unit_file_reader(file_path: &str) -> Result<String, IOError> {
    let mut file = match File::open(file_path) {
        Err(why) => {
            return Err(IOError::new(
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
            return Err(IOError::new(
                ErrorKind::Other,
                format!("Error: read file buf error reason is {}", why),
            ));
        }
    };
    Ok(buf)
}

pub fn toml_str_parse(file_content: &str) -> Result<toml::Value, IOError> {
    let conf: toml::Value = match toml::from_str(file_content) {
        Ok(conf) => conf,
        Err(why) => {
            return Err(IOError::new(
                ErrorKind::Other,
                format!("translate string to struct failed{}", why),
            ));
        }
    };
    Ok(conf)
}

pub fn merge_toml(paths: &Vec<PathBuf>, to: &mut PathBuf) -> Result<(), IOError> {
    let mut merged: toml::Value = toml::Value::Table(toml::value::Table::new());
    for file in paths {
        let value: toml::value::Table = toml::from_slice(
            &fs::read(&file).unwrap_or_else(|_| panic!("Error reading {:?}", file)),
        )
        .unwrap_or_else(|_| panic!("Error reading {:?}", file));
        merge(&mut merged, &toml::Value::Table(value));
    }
    fs::write(to, merged.to_string())
}

fn merge(merged: &mut toml::Value, value: &toml::Value) {
    match value {
        toml::Value::String(_)
        | toml::Value::Integer(_)
        | toml::Value::Float(_)
        | toml::Value::Boolean(_)
        | toml::Value::Datetime(_) => *merged = value.clone(),
        toml::Value::Array(x) => match merged {
            toml::Value::Array(merged) => {
                for (k, v) in x.iter().enumerate() {
                    match merged.get_mut(k) {
                        Some(x) => merge(x, v),
                        None => {
                            merged.insert(k, v.clone());
                        }
                    }
                }
            }
            _ => *merged = value.clone(),
        },
        toml::Value::Table(x) => match merged {
            toml::Value::Table(merged) => {
                for (k, v) in x.iter() {
                    match merged.get_mut(k) {
                        Some(x) => merge(x, v),
                        None => {
                            let _ = merged.insert(k.clone(), v.clone());
                        }
                    }
                }
            }
            _ => *merged = value.clone(),
        },
    }
}
