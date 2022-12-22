//!
use std::fs::File;
use std::io::{BufReader, Error as IOError, ErrorKind, Read};
use std::path::Path;

use nix::unistd::Pid;

use crate::conf_parser;
use crate::Error;

fn cmdline_content() -> Result<String, IOError> {
    let mut file = match File::open("/proc/cmdline") {
        Err(why) => {
            return Err(IOError::new(
                ErrorKind::Other,
                format!("Error: Open file failed detail {}!", why),
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

/// read the content from /proc/cmdline and return the value depend the key
pub fn cmdline_get_value(key: &str) -> Result<Option<String>, Error> {
    let buf = cmdline_content()?;

    let cmdline: Vec<&str> = buf.split_whitespace().collect();

    for cmd in cmdline.iter() {
        let k_val: Vec<&str> = cmd.split('=').map(|s| s.trim()).collect();
        if k_val.len() == 2 && k_val[0] == key {
            return Ok(Some(k_val[1].to_string()));
        }
    }

    Ok(None)
}

/// read the content from /proc/cmdline and return the bool value depend the key
pub fn proc_cmdline_get_bool(key: &str) -> Result<bool, Error> {
    let val = cmdline_get_value(key)?;

    if val.is_none() {
        return Ok(false);
    }

    let r = conf_parser::parse_boolean(&val.unwrap())?;

    Ok(r)
}

/// read /proc/PID/cmdline and return
pub fn get_process_cmdline(pid: &Pid) -> String {
    let pid_str = pid.to_string();
    let cmdline_path = Path::new("/proc").join(pid_str).join("cmdline");
    let file = match File::open(cmdline_path) {
        Ok(file) => file,
        Err(_) => {
            return String::from("");
        }
    };
    let buf_reader = BufReader::new(file);
    let mut cmdline_content = String::new();
    for byte in buf_reader.bytes() {
        let b = match byte {
            Ok(b) => b,
            Err(_) => break,
        };
        let b = if b != 0 { b as char } else { ' ' };
        cmdline_content += &b.to_string();
    }
    cmdline_content
}
