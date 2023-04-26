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

use clap::Parser;
use cmdproto::{
    error::ERROR_CODE_MASK_PRINT_STDOUT,
    proto::{
        abi::{sys_comm, unit_comm, CommandRequest},
        mngr_comm, unit_file, ProstClientStream,
    },
};
use std::io::Write;
use std::{
    net::{SocketAddr, TcpStream},
    process::{ExitCode, Termination},
};

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
    /// [units] start the units
    #[clap(display_order = 1)]
    Start {
        #[clap(required = true)]
        units: Vec<String>,
    },

    /// [units] stop the units
    #[clap(display_order = 2)]
    Stop {
        #[clap(required = true)]
        units: Vec<String>,
    },

    /// [units] restart the units
    #[clap(display_order = 3)]
    Restart { units: Vec<String> },

    /// [units] status of the units
    #[clap(display_order = 4)]
    Status {
        #[clap(required = true)]
        units: Vec<String>,
    },

    /// [units] reload the units
    #[clap(display_order = 5)]
    Reload {
        #[clap(required = true)]
        units: Vec<String>,
    },

    /// [units] reset the failed units
    ResetFailed {
        #[clap(required = true)]
        units: Vec<String>,
    },

    /// [manager] list all units
    ListUnits {},

    /// [system] shutdown the system
    Shutdown {
        #[clap(short, required = false)]
        force: bool,
    },

    /// [system] reboot the system
    Reboot {
        #[clap(short, required = false)]
        force: bool,
    },

    /// [system] halt the system
    Halt {
        #[clap(short, required = false)]
        force: bool,
    },

    /// [system] poweroff the system
    Poweroff {
        #[clap(short, required = false)]
        force: bool,
    },

    /// [manager] Reload sysmaster manager configuration
    DaemonReload {},

    /// [manager] Reexecute sysmaster manager
    DaemonReexec {},

    /// enable one unit file
    Enable {
        #[clap(required = true)]
        unit_file: Vec<String>,
    },

    /// enable one unit file
    Disable {
        #[clap(required = true)]
        unit_file: Vec<String>,
    },

    // mask one unit file
    Mask {
        #[clap(required = true)]
        unit_file: Vec<String>,
    },

    // unmask one unit file
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
            CommandRequest::new_syscomm(sys_comm::Action::Shutdown, force)
        }

        SubCmd::Reboot { force } => CommandRequest::new_syscomm(sys_comm::Action::Shutdown, force),

        SubCmd::Halt { force } => CommandRequest::new_syscomm(sys_comm::Action::Shutdown, force),

        SubCmd::Poweroff { force } => {
            CommandRequest::new_syscomm(sys_comm::Action::Shutdown, force)
        }

        SubCmd::DaemonReload {} => CommandRequest::new_mngrcomm(mngr_comm::Action::Reload),

        SubCmd::DaemonReexec {} => CommandRequest::new_mngrcomm(mngr_comm::Action::Reexec),

        SubCmd::ListUnits {} => CommandRequest::new_mngrcomm(mngr_comm::Action::Listunits),
    };
    Some(command_request)
}

/// Result used in sctl
pub enum Result {
    ///
    OK,
    ///
    Failure(String, u32),
}

/*
 * Implementing Termination is important. Because the default implementation
 * always adds a "Error: " prefix to our error message, this is ugly. And it
 * always exits with 1 if we return Err, this is bad, we want sctl to return
 * many other positive errors like systemctl has done.
 */

impl Termination for Result {
    fn report(self) -> ExitCode {
        match self {
            Result::OK => ExitCode::SUCCESS,
            Result::Failure(s, error_code) => {
                if error_code & ERROR_CODE_MASK_PRINT_STDOUT != 0 {
                    let _ = writeln!(std::io::stdout(), "{s}");
                } else {
                    let _ = writeln!(std::io::stderr(), "{s}");
                }
                let error_code = error_code ^ ERROR_CODE_MASK_PRINT_STDOUT;
                if error_code > u8::MAX.into() {
                    return ExitCode::FAILURE;
                }
                ExitCode::from(error_code as u8)
            }
        }
    }
}

fn main() -> Result {
    let args = Args::parse();

    let command_request = match generate_command_request(args) {
        None => {
            return Result::Failure(
                "This command is currently not supported".to_string(),
                nix::Error::ENOTSUP as u32,
            );
        }
        Some(v) => v,
    };

    let addrs = [
        SocketAddr::from(([127, 0, 0, 1], 9526)),
        SocketAddr::from(([127, 0, 0, 1], 9527)),
    ];

    let stream = match TcpStream::connect(&addrs[..]) {
        Err(e) => {
            let err_msg = format!("Failed to connect to sysmaster: {}", e);
            return Result::Failure(err_msg, e.raw_os_error().unwrap() as u32);
        }
        Ok(v) => v,
    };

    let mut client = ProstClientStream::new(stream);

    let data = match client.execute(command_request) {
        Err(e) => {
            let err_msg = format!("Failed to execute the given command: {}", e);
            // don't want to map all kinds of errors the message is clear, just return 1.
            return Result::Failure(err_msg, 1);
        }
        Ok(v) => v,
    };

    /* We should always print the error message if the returned error code is not 0. */
    if data.message.is_empty() {
        return Result::OK;
    }

    if data.error_code == 0 {
        /* Don't care if we fail to write the error out. */
        let _ = writeln!(std::io::stdout(), "{}", data.message);
        return Result::OK;
    }

    Result::Failure(data.message, data.error_code)
}
