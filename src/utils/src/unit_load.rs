extern crate toml;

use std::fs::File;
use std::io::prelude::*;
use std::io::{Error, ErrorKind};
 
#[derive(Deserialize)]
#[derive(Debug)]
struct ConfUnit {
    #[serde(alias = "Description")]
    description: Option<String>,
    #[serde(alias = "Documentation")]
    documentation: Option<String>,
    #[serde(alias = "Requires")]
    requires: Option<String>,
    #[serde(alias = "Wants")]
    wants: Option<String>,
}

#[derive(Deserialize)]
#[derive(Debug)]
struct ConfService {
    #[serde(alias = "ExecStart")]
    exec_start: Option<String>,
    #[serde(alias = "Sockets")]
    sockets: Option<String>,
    #[serde(alias = "Restart")]
    restart: Option<String>,
    #[serde(alias = "RestrictRealtime")]
    restrict_realtime: Option<String>,
    #[serde(alias = "RebootArgument")]
    reboot_argument: Option<String>,
    #[serde(alias = "ExecReload")]
    exec_reload: Option<String>,
    #[serde(alias = "OOMScoreAdjust")]
    oom_score_adjust: Option<String>,
    #[serde(alias = "RestartSec")]
    restart_sec: Option<u64>,
    #[serde(alias = "Slice")]
    slice: Option<String>,
    #[serde(alias = "MemoryLimit")]
    memory_limit: Option<u64>,
    #[serde(alias = "MemoryLow")]
    memory_low: Option<u64>,
    #[serde(alias = "MemoryMin")]
    memory_min: Option<u64>,
    #[serde(alias = "MemoryMax")]
    memory_max: Option<u64>,
    #[serde(alias = "MemoryHigh")]
    memory_high: Option<u64>,
    #[serde(alias = "MemorySwapMax")]
    memory_swap_max: Option<u64>,
}

#[derive(Deserialize)]
#[derive(Debug)]
struct ConfInstall {
    #[serde(alias = "WantedBy")]
    wanted_by: Option<String>,
    #[serde(alias = "Alias")]
    alias: Option<String>,
    #[serde(alias = "RequiredBy")]
    required_by: Option<String>,
    #[serde(alias = "DefaultInstance")]
    default_instance: Option<String>,
}

#[derive(Deserialize)]
#[derive(Debug)]
pub struct Conf {
    Unit: Option<ConfUnit>,
    Service: Option<ConfService>,
    Install: Option<ConfInstall>,
}

pub fn unit_file_load(file_path: String) -> Result<Conf, Error> {
    let mut file = match File::open(file_path) {
        Ok(f) => f,
        Err(_e) => { return Err(Error::new(ErrorKind::Other,
            "Error: Open file failed"));}
    };

    let mut buf = String::new();
    match file.read_to_string(&mut buf) {
        Ok(s) => s,
        Err(_e) => {return Err(Error::new(ErrorKind::Other,
            "read file content failed"));}
    };

    let conf: Conf = match toml::from_str(&buf) {
        Ok(conf) => conf,
        Err(_e) => {return Err(Error::new(ErrorKind::Other,
            "translate string to struct failed"));}
    };

    return Ok(conf);
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn  test_unit_file_load() -> Result<(), Error>{
        let file: String = String::from("config.toml");
        let conf = match unit_file_load(file) {
            Ok(conf) => {
                match conf.Install {
                    Some(c) => assert_eq!(c.wanted_by, Some("dbus".to_string())),
                    None => {
                        return Err(Error::new(ErrorKind::Other,
                            "no install field"));
                    }
                }
            }
            Err(e) => { return Err(Error::new(ErrorKind::Other,
                e.to_string()));}
        };
        
        Ok(())
    }
}
