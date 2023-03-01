#Unit 配置文件解析#
Unit的配置文件以toml格式来组织，整体包含三段
```
[Unit]
[XXX]   #为客户自定义的配置段
[Install]
```
对于配置文件的解析，定义了ConfigParseM，来自动完成配置文件的解析，ConfigParseM包含一个属性serdeName，指定客户自定配置章节名称,
对于自定义的配置文件，只要定义好配置字段对应的结构体，就会可以自动将配置字符串转换成对应的结构体，例如:

```
use macros::ConfigParseM;
use std::io::{Error as IoError, ErrorKind};
use utils::config_parser::{toml_str_parse, ConfigParse};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, ConfigParseM)]
#[serdeName("Service")]
#[serde(rename_all = "lowercase")] #也可以指定成[serde(rename_all = "PascalCase")]，这样下面的alias字段可以不用配置
pub struct ServiceConf {
    #[serde(alias = "Type", default = "ServiceType::default")]
    service_type: ServiceType,
    #[serde(alias = "ExecStart")]
    exec_start: Option<Vec<String>>,
    #[serde(alias = "ExecStop")]
    exec_stop: Option<Vec<String>>,
    #[serde(alias = "Sockets")]
    sockets: Option<String>,
    #[serde(alias = "Restart",default = "get_default_restart")] //设置配置项的默认值，如果输入参数中没有配置
    pub restart: String,
}

#[derive(Serialize, Deserialize,Clone)]
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
fn get_default_restart() ->String{
    "/usr/bin/example restart"
}
```

ConfigParseM要求接口体实现Serialize和Deserialize的trait，可以直接使用Desrialize和Deserialize相关的宏，具体可以参考[https://serde.rs/]
ConfigparseM生成的get函数，要求成员支持Clone方法,这里是配置解析，通常都是一次加载，多次使用，而且都属于静态配置，所以使用的是直接clone。
定义好宏之后，可以直接使用如下方式获取到对应的配置对象实例化,例如：

```
#let service_str = r###"
[Service]
Type = "forking"
ExecCondition = ["/usr/bin/sleep 5"]
ExecStart = ["/usr/bin/echo 'test'"]
ExecStop = ["/usr/bin/kill $MAINPID"]
"###;
let sp = ServiceConf::builder_parser();
let _service = sp.conf_file_parse(service_str).unwrap();
assert_eq!(_service.get_service_type(), ServiceType::ServiceForking);
assert_eq!(_service.get_exec_stop().unwrap(),vec!["/usr/bin/kill $MAINPID"]);
```
