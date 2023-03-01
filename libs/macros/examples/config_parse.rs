// Copyright (c) 2022 Huawei Technologies Co.,Ltd. All rights reserved.
//
// sysMaster is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan
// PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY
// KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

/*
use macros::ConfigParseM;
use serde::{Deserialize, Serialize};
use std::io::{Error as IoError, ErrorKind};
use strum::Display;
use utils::config_parser::{toml_str_parse, ConfigParse};

//#[derive(Serialize, Deserialize,configParse)]
#[derive(Serialize, Deserialize, ConfigParseM)]
#[serdeName("Service")]
pub struct ConfTest {
    name: String,
}

#[derive(Serialize, Deserialize, ConfigParseM)]
#[serdeName("Service")]
#[serde(rename_all = "lowercase")]
struct ServiceConf {
    #[serde(alias = "Type", default = "ServiceType::default")]
    service_type: ServiceType,
    #[serde(alias = "ExecStart")]
    pub exec_start: Option<Vec<String>>,
    #[serde(alias = "ExecStop")]
    pub exec_stop: Option<Vec<String>>,
    #[serde(alias = "Sockets")]
    pub sockets: Option<String>,
    #[serde(alias = "Restart")]
    pub restart: Option<String>,
    #[serde(alias = "RestrictRealtime")]
    pub restrict_realtime: Option<String>,
    #[serde(alias = "RebootArgument")]
    pub reboot_argument: Option<String>,
    #[serde(alias = "ExecReload")]
    pub exec_reload: Option<Vec<String>>,
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

#[derive(PartialEq, Eq, Serialize, Deserialize, Display, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ServiceType {
    #[serde(alias = "simple")]
    ServiceSimple,
    #[serde(alias = "forking")]
    ServiceForking,
    #[serde(alias = "oneshot")]
    ServiceOneshot,
    #[serde(alias = "dbus")]
    ServiceDbus,
    #[serde(alias = "notify")]
    ServiceNotify,
    #[serde(alias = "idle")]
    ServiceIdle,
    #[serde(alias = "exec")]
    ServiceExec,
    ServiceTypeMax,
    ServiceTypeInvalid = -1,
}

impl Default for ServiceType {
    fn default() -> Self {
        ServiceType::ServiceSimple
    }
}

fn main() {
    let a = ConfTest::builder_parser();
    let str = "
    [Service]
    name='sysmaster'
    ";

    let service_str = r###"
    [Service]
Type = "forking"
ExecCondition = ["/usr/bin/sleep 5"]
ExecStart = ["/usr/bin/echo 'test'"]
ExecStop = ["/usr/bin/kill $MAINPID"]
    "###;

    let r = a.conf_file_parse(r#str);
    // r.map(|x|);
    let b = r.unwrap();
    assert_eq!("sysmaster", b.name.as_str());
    let sp = ServiceConf::builder_parser();
    let _service = sp.conf_file_parse(service_str).unwrap();
    assert_eq!(_service.get_service_type(), ServiceType::ServiceForking);
    assert_eq!(
        _service.get_exec_stop().unwrap(),
        vec!["/usr/bin/kill $MAINPID"]
    );
    assert_eq!(_service.get_exec_reload(), None);
    assert_eq!(
        _service.get_exec_start().unwrap(),
        vec!["/usr/bin/echo 'test'"]
    );

    let default_service_str = r###"
    [Service]
ExecCondition = ["/usr/bin/sleep 5"]
ExecStart = ["/usr/bin/echo 'test'"]
ExecStop = ["/usr/bin/kill $MAINPID"]
    "###;
    let _d_s = sp.conf_file_parse(default_service_str).unwrap();
    assert_eq!(_d_s.get_service_type(), ServiceType::ServiceSimple);
}
*/

//! This is an example about how to use config_parse macro
fn main() {}
