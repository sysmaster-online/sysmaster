use std::io::{Error as IOError, ErrorKind};

use std::{fs::File, io::Read};
pub trait ConfigParse {
    type Item;
    fn conf_file_parse(&self, _file_content: &str) -> Result<Self::Item, IOError> {
        let ret: Result<Self::Item, IOError> = Err(IOError::new(
            ErrorKind::Other,
            format!("config file not Contain Section{}", "Service"),
        ));
        return ret;
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
