use std::io::{Error as IoError,ErrorKind};
use toml::Value;
use std::{fs::File, io::Read};

pub(in crate::manager::unit) fn unit_file_parser (
    file_path: &str,
) -> Result<Value, IoError> {
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
    let v: Value = toml::from_str(&buf).unwrap();
    return Ok(v);
}