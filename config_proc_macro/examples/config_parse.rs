use config_proc_macro::ConfigParseM;
use serde::{Serialize, Deserialize};
use std::io::{Error as IoError};
use utils::config_parser::{ConfigParse, toml_str_parse};
use utils::unit_conf::{Confs};

//#[derive(Serialize, Deserialize,configParse)]
#[derive(Serialize, Deserialize,ConfigParseM)]
#[serdeName("Service")]
pub struct ConfTest{
    name:String
}

#[derive(Serialize, Deserialize,ConfigParseM)]
#[serdeName("Service")]
pub struct ServiceConf {
    #[serde(alias = "Type")]
    pub service_type: Option<String>,
    /*#[serde(alias = "ExecStart")]
    pub exec_start: Option<String>,
    #[serde(alias = "Sockets")]
    pub sockets: Option<String>,
    #[serde(alias = "Restart")]
    pub restart: Option<String>,
    #[serde(alias = "RestrictRealtime")]
    pub restrict_realtime: Option<String>,
    #[serde(alias = "RebootArgument")]
    pub reboot_argument: Option<String>,
    #[serde(alias = "ExecReload")]
    pub exec_reload: Option<String>,
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
    pub memory_swap_max: Option<u64>,*/
}

fn main() {
    let a = ConfTest::builder_paser();
    let str = "
    [Service]
    name='Hushiyuan'
    ";
    let value: toml::Value = toml::from_str(r#str).unwrap();
    println!("{}",value);
    let r = a.conf_file_parser(r#str);
    let b:ConfTest = r.unwrap().unwrap();
    assert_eq!("Hushiyuan",b.name.as_str());
}