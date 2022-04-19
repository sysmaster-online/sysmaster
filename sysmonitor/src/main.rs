/// sysmonitor主进程。作为系统监控的一部分，目前实现的监控类型主要和进程相关，分别是进程数量监控、僵尸进程数量监控、进程fd数量监控和关键进程监控
use std::default::Default;
use std::fs;
use std::fs::File;
use std::io::{self, Error, Read};
use std::num::ParseIntError;
use std::string::FromUtf8Error;

use procfs::ProcError;
use serde_derive::Deserialize;

use crate::process::ProcessMonitor;
use crate::process_count::ProcessCount;
use crate::process_fd::ProcessFd;
use crate::zombie::ZombieCount;

#[allow(dead_code)]
mod process;
mod process_count;
mod process_fd;
mod zombie;

#[derive(Debug)]
pub enum SysMonitorError {
    ProcfsError(ProcError),
    IOError(Error),
    ParseError(ParseIntError),
    UtfError(FromUtf8Error),
}

impl std::convert::From<ProcError> for SysMonitorError {
    fn from(e: ProcError) -> Self {
        SysMonitorError::ProcfsError(e)
    }
}

impl std::convert::From<Error> for SysMonitorError {
    fn from(e: Error) -> Self {
        SysMonitorError::IOError(e)
    }
}

impl std::convert::From<ParseIntError> for SysMonitorError {
    fn from(e: ParseIntError) -> Self {
        SysMonitorError::ParseError(e)
    }
}

impl std::convert::From<FromUtf8Error> for SysMonitorError {
    fn from(e: FromUtf8Error) -> Self {
        SysMonitorError::UtfError(e)
    }
}

const CONFIG_FILE_PATH: &str = "/etc/sysconfig/sysmonitor";

/// Sysmonitor框架。首先定义了一个trait，封装了一个监控的几个特性
/// ```
/// pub trait Monitor {
///     fn config_path(&self) -> &str;
///     fn load(&mut self, content: String, sysmonitor: SysMonitor);
///     fn is_valid(&self) -> bool;
///     fn check_status(&mut self) -> Result<(), SysMonitorError>;
///     fn report_alarm(&self);
/// }
/// ```
/// config_path每个监控会有自己的配置文件路径，load加载函数就是把配置文件转换为结构体，is_valid检查配置项是否合法，
/// check_status检查当前监控关注的指标情况，report_alarm上报告警
/// 这样在新增一个监控项时，只需要实现对应的trait，在主进程的monitor数组中新增一个成员即可
pub trait Monitor {
    fn config_path(&self) -> &str;
    fn load(&mut self, content: String, sysmonitor: SysMonitor);
    fn is_valid(&self) -> bool;
    fn check_status(&mut self) -> Result<(), SysMonitorError>;
    fn report_alarm(&self);
}

/// 首先定义了一个结构体，使用serde crate自动实现Deserialize trait，这样可以从配置文件反序列化成一个结构体，不需要自己写代码。
/// 其次结构体使用了serde提供的default特性，可以通过default方法生成一个空结构体，所有字段为默认值，比如int为0，bool为false。
/// 还使用了rename_all特性，这样可以读取配置文件中字段为结构体中成员对应大写的情况，以适配已有的配置文件。
/// 最后，每个字段使用了serde的default方法特性，默认值可以通过函数的返回值指定，而不是系统默认，比如process_monitor关键进程监控默认为true，
/// 这样可以支持配置文件中没有这个字段但默认值不想设成serde default为false的情况。
#[allow(dead_code)]
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "UPPERCASE")]
pub struct SysMonitor {
    #[serde(default = "on")]
    process_monitor: bool,
    #[serde(default = "process_monitor_period_default")]
    process_monitor_period: u64,
    #[serde(default = "process_recall_default_period")]
    process_recall_period: u32,
    #[serde(default = "process_restart_default_timeout")]
    process_restart_timeout: u32,
    #[serde(default = "process_alarm_suppress_num_default")]
    process_alarm_suppress_num: u32,
    process_alarm: bool,
    #[serde(default = "on")]
    pscnt_monitor: bool,
    pscnt_alarm: bool,
    #[serde(default = "on")]
    process_fd_num_monitor: bool,
    process_fd_num_alarm: bool,
    zombie_monitor: bool,
    zombie_alarm: bool,
}

/// 因为每个监控项都有共同的控制选项，监控默认打开，而告警默认关闭，故把这两个字段组合为一个结构体Switch
#[allow(dead_code)]
#[derive(Debug, Default, Deserialize)]
pub struct Switch {
    monitor: bool,
    alarm: bool,
}

pub fn config_file_load(file_path: &str) -> io::Result<String> {
    let mut file = File::open(file_path)?;

    let mut buf = String::new();
    file.read_to_string(&mut buf)?;
    Ok(buf)
}

fn on() -> bool {
    true
}

pub fn process_monitor_period_default() -> u64 {
    3
}

fn process_recall_default_period() -> u32 {
    1
}

fn process_restart_default_timeout() -> u32 {
    90
}

fn process_alarm_suppress_num_default() -> u32 {
    5
}

fn main() -> io::Result<()> {
    // 从配置文件生成sysmonitor结构体
    let toml_str = fs::read_to_string(CONFIG_FILE_PATH)?;
    let sysmonitor: SysMonitor = toml::from_str(&toml_str).unwrap();

    // 当前支持四项监控，进程数量、僵尸进程数量、进程fd数量监控和关键进程监控，后期可以修改数组
    let monitors: [&mut dyn Monitor; 4] = [
        &mut ProcessCount::default(),
        &mut ZombieCount::default(),
        &mut ProcessFd::default(),
        &mut ProcessMonitor::default(),
    ];
    for monitor in monitors {
        // 读取每个监控项的配置文件
        let contents = fs::read_to_string(monitor.config_path())?;
        // 将配置文件反序列化为一个结构体，赋值给当前monitor
        monitor.load(contents, sysmonitor.clone());
        // 检查配置是否合法，并检查当前状态
        monitor.is_valid();
        let _ = monitor.check_status();
    }
    Ok(())
}
