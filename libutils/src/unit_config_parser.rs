extern crate toml;

use std::fs::File;
use std::io::prelude::*;
use std::io::{Error, ErrorKind};
 
#[derive(Debug, Deserialize)]
pub struct ConfUnit {
    #[serde(alias = "Description")]
    pub description: Option<String>,
    #[serde(alias = "Documentation")]
    pub documentation: Option<String>,
    #[serde(alias = "Requires")]
    pub requires: Option<String>,
    #[serde(alias = "Wants")]
    pub wants: Option<String>,
    #[serde(alias = "Before")]
    pub before: Option<String>,
    #[serde(alias = "After")]
    pub after: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ConfService {
    #[serde(alias = "Type")]
    pub service_type: Option<String>,
    #[serde(alias = "ExecCondition")]
    pub exec_condition: Option<Vec<String>>,
    #[serde(alias = "ExecStartPre")]
    pub exec_prestart: Option<Vec<String>>,
    #[serde(alias = "ExecStart")]
    pub exec_start: Option<Vec<String>>,
    #[serde(alias = "ExecStartPost")]
    pub exec_startpost: Option<Vec<String>>,
    #[serde(alias = "ExecReload")]
    pub exec_reload: Option<Vec<String>>,
    #[serde(alias = "ExecStop")]
    pub exec_stop: Option<Vec<String>>,
    #[serde(alias = "ExecStopPost")]
    pub exec_stoppost: Option<Vec<String>>,
    #[serde(alias = "Sockets")]
    pub sockets: Option<String>,
    #[serde(alias = "Restart")]
    pub restart: Option<String>,
    #[serde(alias = "RestrictRealtime")]
    pub restrict_realtime: Option<String>,
    #[serde(alias = "RebootArgument")]
    pub reboot_argument: Option<String>,
    #[serde(alias = "OOMScoreAdjust")]
    pub oom_score_adjust: Option<String>,
    #[serde(alias = "RestartSec")]
    pub restart_sec: Option<u64>,
    #[serde(alias = "Slice")]
    pub slice: Option<String>,
    #[serde(alias = "MemoryLimit")]
    pub memory_limit: Option<u64>,
    #[serde(alias = "MemoryLow")]
    pub memory_low: Option<u64>,
    #[serde(alias = "MemoryMin")]
    pub memory_min: Option<u64>,
    #[serde(alias = "MemoryMax")]
    pub memory_max: Option<u64>,
    #[serde(alias = "MemoryHigh")]
    pub memory_high: Option<u64>,
    #[serde(alias = "MemorySwapMax")]
    pub memory_swap_max: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct ConfInstall {
    #[serde(alias = "WantedBy")]
    pub wanted_by: Option<String>,
    #[serde(alias = "Alias")]
    pub alias: Option<String>,
    #[serde(alias = "RequiredBy")]
    pub required_by: Option<String>,
    #[serde(alias = "DefaultInstance")]
    pub default_instance: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Conf {
    #[serde(alias = "Unit")]
    pub unit: Option<ConfUnit>,
    #[serde(alias = "Service")]
    pub service: Option<ConfService>,
    #[serde(alias = "Install")]
    pub install: Option<ConfInstall>,
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
        let file: String = String::from("config.service");
        match unit_file_load(file) {
            Ok(conf) => {
                match conf.install {
                    Some(c) => assert_eq!(c.wanted_by, Some("dbus".to_string())),
                    None => {
                        return Err(Error::new(ErrorKind::Other,
                            "no install field"));
                    }
                }
            }
            Err(e) => {
                return Err(Error::new(ErrorKind::Other,
                e.to_string()));}
        };
        
        Ok(())
    }
}
