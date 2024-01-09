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

//! sysmaster-run

#![allow(deprecated)]
use basic::env;
use basic::id128;
use clap::Parser;
use cmdproto::proto::transient_unit_comm::UnitConfig;
use cmdproto::proto::transient_unit_comm::UnitProperty;
use cmdproto::proto::ProstClientStream;
use cmdproto::{error::ERROR_CODE_MASK_PRINT_STDOUT, proto::abi::CommandRequest};
use constants::PRIVATE_SOCKET;
use core::unit;
use core::unit::unit_name_is_valid;
use core::unit::UnitNameFlags;
use log::Level;
use std::path::PathBuf;
use std::process::exit;
use std::str::FromStr;
use std::{io::Write, os::unix::net::UnixStream};

/// parse program arguments
#[derive(Parser, Debug, Default)]
#[clap(author, version, about, long_about = None)]

struct Args {
    /// unit name
    #[clap(long, default_value_t = String::new())]
    unit: String,

    /// property
    #[clap(short('p'), long)]
    property: Vec<String>,

    /// path property
    #[clap(long)]
    path_property: Vec<String>,

    /// socket property
    #[clap(long)]
    socket_property: Vec<String>,

    /// timer property
    #[clap(long)]
    timer_property: Vec<String>,

    /// description
    #[clap(long, default_value_t = String::new())]
    description: String,

    /// remain-after-exit
    #[clap(long, short('r'), required = false)]
    remain_after_exit: bool,

    /// service type
    #[clap(long, default_value_t = String::new())]
    service_type: String,

    /// uid
    #[clap(long, default_value_t = String::new())]
    uid: String,

    /// gid
    #[clap(long, default_value_t = String::new())]
    gid: String,

    /// working directory
    #[clap(long, default_value_t = String::new())]
    working_directory: String,

    /// same dir
    #[clap(short('d'), long, required = false)]
    same_dir: bool,

    /// set env
    #[clap(short('E'), long)]
    setenv: Vec<String>,

    /// no ask password
    #[clap(long, required = false)]
    no_ask_password: bool,

    /// no block
    #[clap(long, required = false)]
    no_block: bool,

    /// user
    #[clap(long, required = false)]
    user: bool,

    /// system
    #[clap(long, required = false)]
    system: bool,

    /// send sighup
    #[clap(long, required = false)]
    send_sighup: bool,

    /// pty
    #[clap(short('t'), long, required = false)]
    pty: bool,

    /// pipe
    #[clap(short('P'), long, required = false)]
    pipe: bool,

    /// quiet
    #[clap(short('q'), long, required = false)]
    quiet: bool,

    /// on active
    #[clap(long, default_value_t = String::new())]
    on_active: String,

    /// on boot
    #[clap(long, default_value_t = String::new())]
    on_boot: String,

    /// on startup
    #[clap(long, default_value_t = String::new())]
    on_startup: String,

    /// on unit active
    #[clap(long, default_value_t = String::new())]
    on_unit_active: String,

    /// on unit inactive
    #[clap(long, default_value_t = String::new())]
    on_unit_inactive: String,

    /// on calendar
    #[clap(long, default_value_t = String::new())]
    on_calendar: String,

    /// collect
    #[clap(short('G'), long, required = false)]
    collect: bool,

    /// args cmdline
    #[clap()]
    args_cmdline: Vec<String>,
}

fn deal_working_directory(working_directory: &str) -> basic::error::Result<String> {
    let path_buf = if working_directory.starts_with('/') {
        PathBuf::from(working_directory)
    } else {
        match std::env::current_dir() {
            Ok(mut dir) => {
                dir.push(working_directory);
                dir
            }
            Err(e) => {
                log::error!("Failed to get current working directory: {}", e);
                return Err(basic::Error::Io { source: e });
            }
        }
    };

    match path_buf.canonicalize() {
        Ok(path_buf) => Ok(path_buf.to_string_lossy().to_string()),
        Err(e) => {
            log::error!(
                "Failed to parse path {} and make it absolute: {}",
                working_directory,
                e
            );
            Err(basic::Error::Io { source: e })
        }
    }
}

fn parse_args(args: &mut Args) -> basic::error::Result<()> {
    if args.user {
        args.no_ask_password = true;
    }

    if !args.working_directory.is_empty() {
        args.working_directory = deal_working_directory(&args.working_directory)?;
    }

    if args.same_dir {
        args.working_directory = match std::env::current_dir() {
            Ok(dir) => dir.to_string_lossy().to_string(),
            Err(e) => {
                log::error!("Failed to get current working directory: {}", e);
                return Err(basic::Error::Io { source: e });
            }
        };
    }

    for env in &args.setenv {
        env::Env::new(env)?;
    }

    let mut arg_with_timer = false;
    for timer_property in &args.timer_property {
        for start in [
            "OnActiveSec=",
            "OnBootSec=",
            "OnStartupSec=",
            "OnUnitActiveSec=",
            "OnUnitInactiveSec=",
            "OnCalendar=",
        ] {
            if timer_property.starts_with(start) {
                arg_with_timer = true;
                break;
            }
        }
    }

    arg_with_timer = arg_with_timer
        || !args.on_active.is_empty()
        || !args.on_boot.is_empty()
        || !args.on_calendar.is_empty()
        || !args.on_startup.is_empty()
        || !args.on_unit_inactive.is_empty()
        || !args.on_unit_active.is_empty();

    if !args.on_active.is_empty() {
        args.timer_property
            .push(format!("OnActiveSec={}", args.on_active));
    }
    if !args.on_boot.is_empty() {
        args.timer_property
            .push(format!("OnBootSec={}", args.on_boot));
    }
    if !args.on_calendar.is_empty() {
        args.timer_property
            .push(format!("OnCalendar={}", args.on_calendar));
    }
    if !args.on_startup.is_empty() {
        args.timer_property
            .push(format!("OnStartupSec={}", args.on_startup));
    }
    if !args.on_unit_inactive.is_empty() {
        args.timer_property
            .push(format!("OnUnitInactiveSec={}", args.on_unit_inactive));
    }
    if !args.on_unit_active.is_empty() {
        args.timer_property
            .push(format!("OnUnitActiveSec={}", args.on_unit_active));
    }

    let with_trigger =
        !args.path_property.is_empty() || !args.socket_property.is_empty() || arg_with_timer;

    if (args.path_property.len() + args.socket_property.len() + if arg_with_timer { 1 } else { 0 })
        > 1
    {
        log::error!("Only single trigger (path, socket, timer) unit can be created.");
        return Err(basic::Error::Other {
            msg: "Only single trigger (path, socket, timer) unit can be created.".to_string(),
        });
    }

    if args.args_cmdline.is_empty() && args.unit.is_empty() && !with_trigger {
        log::error!("Command line to execute required.");
        return Err(basic::Error::Other {
            msg: "Command line to execute required.".to_string(),
        });
    }

    if !args.timer_property.is_empty() && !arg_with_timer {
        log::error!("--timer-property= has no effect without any other timer options.");
        return Err(basic::Error::Other {
            msg: "--timer-property= has no effect without any other timer options.".to_string(),
        });
    }

    if args.description.is_empty() {
        args.description = args
            .args_cmdline
            .iter()
            .fold(String::new(), |args, arg| args + arg + " ")
            .trim()
            .to_string();
    }

    Ok(())
}

fn get_unit_name(name: &str, suffix: &str) -> nix::Result<String> {
    let unit_name = if name.is_empty() {
        format!(
            "run-r{}{}",
            basic::id128::id128_randomize(id128::Id128FormatFlag::ID128_FORMAT_PLAIN).map_err(
                |e| {
                    log::error!("Failed to generate random run unit name:{}", e);
                    e
                }
            )?,
            suffix
        )
    } else if name.ends_with(suffix) {
        name.to_string()
    } else {
        format!("{}{}", name, suffix)
    };

    if !unit_name_is_valid(&unit_name, UnitNameFlags::PLAIN) {
        log::debug!("Invalid unit name: {}", name);
        return Err(nix::errno::Errno::EINVAL);
    }

    Ok(unit_name)
}

fn unit_suffix_is_valid(suffix: &str) -> bool {
    if suffix.is_empty() {
        return false;
    }
    if !suffix.starts_with('.') {
        return false;
    }

    unit::UnitType::from_str(&suffix[1..]).map_or(false, |unit_type| {
        unit_type != unit::UnitType::UnitTypeInvalid
    })
}

fn unit_name_change_suffix(name: &str, suffix: &str) -> nix::Result<String> {
    if !unit_name_is_valid(name, UnitNameFlags::PLAIN) {
        return Err(nix::errno::Errno::EINVAL);
    }

    if !unit_suffix_is_valid(suffix) {
        return Err(nix::errno::Errno::EINVAL);
    }

    let unit_name = match name.rfind('.') {
        Some(pos) => format!("{}{}", name[..pos].to_string(), suffix.to_string()),
        None => return Err(nix::errno::Errno::EINVAL),
    };

    if !unit_name_is_valid(&unit_name, UnitNameFlags::PLAIN) {
        return Err(nix::errno::Errno::EINVAL);
    }

    Ok(unit_name)
}

fn to_unit_property(property: &str) -> nix::Result<UnitProperty> {
    if property.is_empty() {
        return Err(nix::errno::Errno::EINVAL);
    }

    match property.find('=') {
        None => Err(nix::errno::Errno::EINVAL),
        Some(pos) => {
            if pos == 0 {
                Err(nix::errno::Errno::EINVAL)
            } else {
                Ok(UnitProperty {
                    key: property[0..pos].to_string(),
                    value: property[pos + 1..].to_string(),
                })
            }
        }
    }
}

fn transient_unit_set_properties(
    description: &str,
    collect: bool,
    properties: &[String],
) -> nix::Result<Vec<UnitProperty>> {
    let mut unit_properties = vec![];
    if !description.is_empty() {
        unit_properties.push(UnitProperty {
            key: "Description".to_string(),
            value: description.to_string(),
        });
    }

    if collect {
        unit_properties.push(UnitProperty {
            key: "CollectMode".to_string(),
            value: "inactive-or-failed".to_string(),
        });
    }

    for property in properties {
        unit_properties.push(to_unit_property(property)?);
    }
    Ok(unit_properties)
}

fn transient_service_set_properties(args: &Args) -> nix::Result<Vec<UnitProperty>> {
    let mut properties =
        transient_unit_set_properties(&args.description, args.collect, &args.property)?;

    if args.send_sighup {
        properties.push(UnitProperty {
            key: "SendSIGHUP".to_string(),
            value: "true".to_string(),
        });
    }

    if args.remain_after_exit {
        properties.push(UnitProperty {
            key: "RemainAfterExit".to_string(),
            value: "true".to_string(),
        });
    }

    if !args.service_type.is_empty() {
        properties.push(UnitProperty {
            key: "Type".to_string(),
            value: args.service_type.clone(),
        });
    }

    if !args.uid.is_empty() {
        properties.push(UnitProperty {
            key: "User".to_string(),
            value: args.uid.clone(),
        });
    }

    if !args.gid.is_empty() {
        properties.push(UnitProperty {
            key: "Group".to_string(),
            value: args.gid.clone(),
        });
    }

    if !args.working_directory.is_empty() {
        properties.push(UnitProperty {
            key: "WorkingDirectory".to_string(),
            value: args.working_directory.clone(),
        });
    }

    if !args.setenv.is_empty() {
        let mut envs = String::new();
        for env in &args.setenv {
            if !env::Env::is_valid(env) {
                return Err(nix::errno::Errno::EINVAL);
            }
            envs.push_str(env);
            envs.push(' ');
        }
        properties.push(UnitProperty {
            key: "Environment".to_string(),
            value: envs.trim().to_string(),
        });
    }

    if !args.args_cmdline.is_empty() {
        properties.push(UnitProperty {
            key: "ExecStart".to_string(),
            value: args
                .args_cmdline
                .iter()
                .fold(String::new(), |args, arg| args + arg + " "),
        });
    }
    Ok(properties)
}

fn generate_command_request(args: Args) -> nix::Result<CommandRequest> {
    let unit_name;
    let mut unit_properties;
    let aux_name;
    let aux_unit_properties;

    if !args.path_property.is_empty() {
        unit_name = get_unit_name(&args.unit, ".path")?;
        unit_properties =
            transient_unit_set_properties(&args.description, args.collect, &args.path_property)?;
        aux_name = unit_name_change_suffix(&unit_name, ".service")?;
        aux_unit_properties = transient_service_set_properties(&args)?;
    } else if !args.socket_property.is_empty() {
        unit_name = get_unit_name(&args.unit, ".socket")?;
        unit_properties =
            transient_unit_set_properties(&args.description, args.collect, &args.socket_property)?;
        aux_name = unit_name_change_suffix(&unit_name, ".service")?;
        aux_unit_properties = transient_service_set_properties(&args)?;
    } else if !args.timer_property.is_empty() {
        unit_name = get_unit_name(&args.unit, ".timer")?;
        aux_name = unit_name_change_suffix(&unit_name, ".service")?;
        unit_properties =
            transient_unit_set_properties(&args.description, args.collect, &args.timer_property)?;
        aux_unit_properties = transient_service_set_properties(&args)?;
        unit_properties.push(UnitProperty {
            key: "RemainAfterElapse".to_string(),
            value: "false".to_string(),
        });
    } else {
        unit_name = get_unit_name(&args.unit, ".service")?;
        unit_properties = transient_service_set_properties(&args)?;
        aux_name = "".to_string();
        aux_unit_properties = vec![];
    }

    let mut aux_units: Vec<UnitConfig> = vec![];
    if !aux_name.is_empty() {
        aux_units.push(UnitConfig {
            unit_name: aux_name,
            unit_properties: aux_unit_properties,
        })
    }

    let s = CommandRequest::new_transient_unit_comm(
        "fail",
        &UnitConfig {
            unit_name,
            unit_properties,
        },
        &aux_units,
    );
    Ok(s)
}

fn main() {
    log::init_log_to_console("sysmaster-run", Level::Debug);
    let mut args = Args::parse();
    if let Err(e) = parse_args(&mut args) {
        log::debug!("parse args error: {}", e);
        std::process::exit(-1);
    }

    let command_request = match generate_command_request(args) {
        Err(e) => {
            eprintln!("Unknown unit name or property:{}", e);
            exit(e as i32);
        }
        Ok(v) => v,
    };

    let stream = match UnixStream::connect(PRIVATE_SOCKET) {
        Err(e) => {
            eprintln!("Failed to connect to sysmaster: {}", e);
            exit(e.raw_os_error().unwrap());
        }
        Ok(v) => v,
    };

    let mut client = ProstClientStream::new(stream);

    let data = match client.execute(command_request) {
        Err(e) => {
            eprintln!("Failed to execute the given command: {}", e);
            exit(1);
        }
        Ok(v) => v,
    };

    /* We should always print the error message if the returned error code is not 0. */
    if data.message.is_empty() {
        exit(0);
    }

    if data.error_code == 0 || (data.error_code & ERROR_CODE_MASK_PRINT_STDOUT != 0) {
        /* Don't care if we fail to write the message out. */
        let _ = writeln!(std::io::stdout(), "{}", data.message);
    } else {
        eprintln!("{}", data.message);
    }

    exit((data.error_code & !ERROR_CODE_MASK_PRINT_STDOUT) as i32);
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{create_dir, remove_dir_all};

    #[test]
    fn test_get_unit_name() {
        assert_eq!(
            get_unit_name("test", ".service").unwrap(),
            "test.service".to_string()
        );
        assert_eq!(
            get_unit_name("test.service", ".service").unwrap(),
            "test.service".to_string()
        );
        assert!(get_unit_name("", ".service").unwrap().starts_with("run-r"));
        assert!(get_unit_name("test.mount", ".service").is_err());
    }

    #[test]
    fn test_unit_suffix_is_valid() {
        assert!(unit_suffix_is_valid(".service"));
        assert!(unit_suffix_is_valid(".socket"));
        assert!(!unit_suffix_is_valid("service"));
        assert!(!unit_suffix_is_valid(".test"));
    }

    #[test]
    fn test_deal_working_directory() {
        let abs_path = "/root".to_string();
        assert_eq!(deal_working_directory(&abs_path).unwrap(), abs_path);
        let rel_path = "test";
        let mut cur_path = std::env::current_dir().unwrap();
        cur_path.push(rel_path);
        create_dir(rel_path).unwrap();
        assert_eq!(
            deal_working_directory(rel_path).unwrap(),
            cur_path.to_string_lossy().to_string()
        );
        remove_dir_all(rel_path).unwrap();
    }

    #[test]
    fn test_parse_args() {
        let mut args = Args::parse_from(vec![
            "sysmaster-run",
            "--unit",
            "test",
            "--user",
            "--same-dir",
            "--remain-after-exit",
            "-E",
            "aa=bb",
            "/bin/sleep",
        ]);
        assert!(parse_args(&mut args).is_ok());
        assert!(args.user);
        assert!(args.no_ask_password);
        assert_eq!(
            args.working_directory,
            std::env::current_dir()
                .unwrap()
                .to_string_lossy()
                .to_string()
        );
        let mut args = Args::parse_from(vec![
            "sysmaster-run",
            "--unit",
            "test",
            "--socket-property",
            "ListenStream=/tmp/server.socket",
            "sleep",
        ]);
        assert!(parse_args(&mut args).is_ok());
        assert!(parse_args(&mut Args::parse_from(vec![
            "sysmaster-run",
            "--unit",
            "test",
            "--socket-property",
            "ListenStream=/tmp/server.socket",
            "--timer-property",
            "OnActiveSec=1",
            "--on-boot",
            "1",
            "sleep"
        ]))
        .is_err());
    }

    #[test]
    fn test_to_unit_property() {
        assert_eq!(
            to_unit_property("aa=bb").unwrap(),
            UnitProperty {
                key: "aa".to_string(),
                value: "bb".to_string()
            }
        );
        assert_eq!(
            to_unit_property("aa==bb").unwrap(),
            UnitProperty {
                key: "aa".to_string(),
                value: "=bb".to_string()
            }
        );
        assert_eq!(
            to_unit_property("aa=").unwrap(),
            UnitProperty {
                key: "aa".to_string(),
                value: "".to_string()
            }
        );
        assert!(to_unit_property("=").is_err());
    }
}
