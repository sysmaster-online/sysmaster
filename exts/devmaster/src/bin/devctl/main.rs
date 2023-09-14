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

//! devctrl is the client of devmaster
//!
#![allow(deprecated)]
mod subcmds;
use clap::Parser;
use libdevmaster::framework::control_manager::CONTROL_MANAGER_LISTEN_ADDR;
use log::logger::init_log_to_console_syslog;
use log::Level;
use std::{io::Write, os::unix::net::UnixStream};
use subcmds::devctl_monitor::subcommand_monitor;
use subcmds::devctl_test_builtin::subcommand_test_builtin;
use subcmds::devctl_trigger::subcommand_trigger;

/// parse program arguments
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Kinds of subcommands
    #[clap(subcommand)]
    subcmd: SubCmd,
}

/// Kinds of subcommands
#[derive(Parser, Debug)]
enum SubCmd {
    /// Monitor device events from kernel and userspace
    #[clap(display_order = 1)]
    Monitor {},

    /// Kill all devmaster workers
    #[clap(display_order = 2)]
    Kill {},

    /// Trigger a fake device action, then the kernel will report an uevent
    #[clap(display_order = 3)]
    Trigger {
        /// the kind of device action to trigger
        #[clap(short, long)]
        action: Option<String>,

        /// the enumerator type, can be devices (default) or subsystems
        #[clap(short, long)]
        r#type: Option<String>,

        /// print searched devices by enumerator
        #[clap(short, long)]
        verbose: bool,

        /// the devices to be triggered
        #[clap(required = false)]
        devices: Vec<String>,

        /// do not actually trigger the device events
        #[clap(short('n'), long)]
        dry_run: bool,
    },

    /// Test builtin command on a device
    #[clap(display_order = 4)]
    TestBuiltin {
        /// device action
        #[clap(short, long)]
        action: Option<String>,
        /// builtin command
        #[clap(required = true)]
        builtin: String,
        /// device syspath
        #[clap(required = true)]
        syspath: String,
    },
}

/// subcommand for killing workers
fn subcommand_kill() {
    let mut stream = UnixStream::connect(CONTROL_MANAGER_LISTEN_ADDR).unwrap();
    stream.write_all(b"kill ").unwrap();
}

fn main() {
    init_log_to_console_syslog("devctl", Level::Debug);
    let args = Args::parse();

    match args.subcmd {
        SubCmd::Monitor {} => subcommand_monitor(),
        SubCmd::Kill {} => subcommand_kill(),
        SubCmd::Trigger {
            action,
            r#type,
            verbose,
            devices,
            dry_run,
        } => subcommand_trigger(devices, r#type, verbose, action, dry_run),
        SubCmd::TestBuiltin {
            action,
            builtin,
            syspath,
        } => subcommand_test_builtin(action, builtin, syspath),
    }
}
