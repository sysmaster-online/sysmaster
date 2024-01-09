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

//!

#![allow(deprecated)]
use clap::Parser;
use cmdproto::{
    error::ERROR_CODE_MASK_PRINT_STDOUT,
    proto::{
        abi::{sys_comm, unit_comm, CommandRequest},
        mngr_comm, unit_file, ProstClientStream,
    },
};
use constants::PRIVATE_SOCKET;
use std::process::exit;
use std::{io::Write, os::unix::net::UnixStream};

/// parse program arguments
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]

struct Args {
    /// Name of unit
    #[clap(subcommand)]
    subcmd: SubCmd,

    /// Number of times
    #[clap(short, long, default_value_t = 1)]
    count: u8,
}

#[derive(Parser, Debug)]
enum SubCmd {
    /// `[unit]` Isolate one or more units
    #[clap(display_order = 1)]
    Isolate {
        #[clap(required = true)]
        units: Vec<String>,
    },

    /// `[unit]` Reload one or more units
    #[clap(display_order = 2)]
    Reload {
        #[clap(required = true)]
        units: Vec<String>,
    },

    /// `[unit]` Reset the failed state of one or more units
    #[clap(display_order = 3)]
    ResetFailed {
        #[clap(required = true)]
        units: Vec<String>,
    },

    /// `[unit]` Restart one or more units
    #[clap(display_order = 4)]
    Restart {
        #[clap(required = true)]
        units: Vec<String>,
    },

    /// `[unit]` Show the status of one or more units
    #[clap(display_order = 5)]
    Status {
        #[clap(required = true)]
        units: Vec<String>,
    },

    /// `[unit]` Start one or more units
    #[clap(display_order = 6)]
    Start {
        #[clap(required = true)]
        units: Vec<String>,
    },

    /// `[unit]` Stop one or more units
    #[clap(display_order = 7)]
    Stop {
        #[clap(required = true)]
        units: Vec<String>,
    },

    /// `[manager]` Reload sysmaster manager configuration
    #[clap(display_order = 8)]
    DaemonReload {},

    /// `[manager]` Reexecute sysmaster manager
    #[clap(display_order = 9)]
    DaemonReexec {},

    /// `[manager]` List all units
    #[clap(display_order = 10)]
    ListUnits {},

    /// `[system]` Halt the system
    #[clap(display_order = 11)]
    Halt {
        #[clap(short, required = false)]
        force: bool,
    },

    /// `[system]` Poweroff the system
    #[clap(display_order = 12)]
    Poweroff {
        #[clap(short, required = false)]
        force: bool,
    },

    /// `[system]` Reboot the system
    #[clap(display_order = 13)]
    Reboot {
        #[clap(short, required = false)]
        force: bool,
    },

    /// `[system]` Shutdown the system
    #[clap(display_order = 14)]
    Shutdown {
        #[clap(short, required = false)]
        force: bool,
    },

    /// `[system]` Switch to other root file system
    #[clap(display_order = 15)]
    SwitchRoot {
        #[clap(required = false)]
        init: Vec<String>,
    },

    /// `[unit-file]` Enable one or more units
    #[clap(display_order = 16)]
    Enable {
        #[clap(required = true)]
        unit_file: Vec<String>,
    },

    /// `[unit-file]` Disable one or more units
    #[clap(display_order = 17)]
    Disable {
        #[clap(required = true)]
        unit_file: Vec<String>,
    },

    /// `[unit-file]` Mask one or more units
    #[clap(display_order = 18)]
    Mask {
        #[clap(required = true)]
        unit_file: Vec<String>,
    },

    /// `[unit-file]` Unmask one or more units
    #[clap(display_order = 19)]
    Unmask {
        #[clap(required = true)]
        unit_file: Vec<String>,
    },
}

/// Generate CommandRequest based on parsed args
/// clap Args => protobuf based CommandRequest
fn generate_command_request(args: Args) -> Option<CommandRequest> {
    let command_request = match args.subcmd {
        SubCmd::Start { units } => CommandRequest::new_unitcomm(unit_comm::Action::Start, units),
        SubCmd::Stop { units } => CommandRequest::new_unitcomm(unit_comm::Action::Stop, units),
        SubCmd::Restart { units } => {
            CommandRequest::new_unitcomm(unit_comm::Action::Restart, units)
        }
        SubCmd::Status { units } => CommandRequest::new_unitcomm(unit_comm::Action::Status, units),

        SubCmd::Reload { units } => CommandRequest::new_unitcomm(unit_comm::Action::Reload, units),
        SubCmd::Isolate { units } => {
            CommandRequest::new_unitcomm(unit_comm::Action::Isolate, units)
        }
        SubCmd::ResetFailed { units } => {
            CommandRequest::new_unitcomm(unit_comm::Action::Resetfailed, units)
        }

        SubCmd::Mask { unit_file } => {
            CommandRequest::new_unitfile(unit_file::Action::Mask, unit_file)
        }
        SubCmd::Unmask { unit_file } => {
            CommandRequest::new_unitfile(unit_file::Action::Unmask, unit_file)
        }
        SubCmd::Enable { unit_file } => {
            CommandRequest::new_unitfile(unit_file::Action::Enable, unit_file)
        }
        SubCmd::Disable { unit_file } => {
            CommandRequest::new_unitfile(unit_file::Action::Disable, unit_file)
        }

        SubCmd::Shutdown { force } => {
            CommandRequest::new_syscomm(sys_comm::Action::Poweroff, force)
        }

        SubCmd::Reboot { force } => CommandRequest::new_syscomm(sys_comm::Action::Reboot, force),

        SubCmd::Halt { force } => CommandRequest::new_syscomm(sys_comm::Action::Halt, force),

        SubCmd::Poweroff { force } => {
            CommandRequest::new_syscomm(sys_comm::Action::Poweroff, force)
        }

        SubCmd::DaemonReload {} => CommandRequest::new_mngrcomm(mngr_comm::Action::Reload),

        SubCmd::DaemonReexec {} => CommandRequest::new_mngrcomm(mngr_comm::Action::Reexec),

        SubCmd::ListUnits {} => CommandRequest::new_mngrcomm(mngr_comm::Action::Listunits),
        SubCmd::SwitchRoot { init } => CommandRequest::new_switch_root_comm(init),
    };
    Some(command_request)
}

fn parse_args() -> Args {
    let mut str_args: Vec<String> = std::env::args().collect();

    if str_args.is_empty() {
        return Args::parse();
    }

    let subcmd = match std::path::Path::new(&str_args[0]).file_name() {
        Some(file_name) => file_name.to_string_lossy().to_string(),
        None => return Args::parse(),
    };

    if [
        "halt".to_string(),
        "reboot".to_string(),
        "poweroff".to_string(),
        "shutdown".to_string(),
    ]
    .contains(&subcmd)
    {
        str_args.insert(1, subcmd);
        return Args::parse_from(str_args);
    }

    Args::parse()
}

fn main() {
    let args = parse_args();

    let command_request = match generate_command_request(args) {
        None => {
            eprintln!("This command is currently not supported");
            exit(nix::Error::ENOTSUP as i32);
        }
        Some(v) => v,
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
