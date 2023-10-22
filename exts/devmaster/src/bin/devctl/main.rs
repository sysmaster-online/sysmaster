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
mod daemon;
mod subcmds;
use basic::argv_util::invoked_as;
use clap::Parser;
use daemon::run_daemon;
use libdevmaster::framework::control_manager::CONTROL_MANAGER_LISTEN_ADDR;
use log::init_log_to_console_syslog;
use log::Level;
use std::{io::Write, os::unix::net::UnixStream};
use subcmds::devctl_hwdb::subcommand_hwdb;
use subcmds::devctl_info::InfoArgs;
use subcmds::devctl_monitor::subcommand_monitor;
use subcmds::devctl_test_builtin::subcommand_test_builtin;
use subcmds::devctl_trigger::subcommand_trigger;

type Result<T> = std::result::Result<T, nix::Error>;

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
    /// Query sysfs or the devmaster database
    #[clap(display_order = 1)]
    Info {
        #[clap(short, long, possible_values(&["name", "symlink", "path", "property", "env", "all"]), help(
            "Query device information:\n\
                name                     Name of device node\n\
                symlink                  Pointing to node\n\
                path                     sysfs device path\n\
                property or env          The device properties\n\
                all                      All values\n")
        )]
        query: Option<String>,

        /// Print all key matches walking along the chain
        /// of parent devices
        #[clap(short, long)]
        attribute_walk: bool,

        /// Print major:minor of device containing this file
        #[clap(short, long)]
        device_id_of_file: Option<String>,

        /// Export key/value pairs
        #[clap(short('x'), long)]
        export: bool,

        /// Export the key name with a prefix
        #[clap(short('P'), long)]
        export_prefix: Option<String>,

        /// Export the content of the devmaster database
        #[clap(short('e'), long)]
        export_db: bool,

        /// Clean up the devmaster database
        #[clap(short, long)]
        cleanup_db: bool,

        /// Prepend dev directory to path names
        #[clap(short, long)]
        root: bool,

        ///
        #[clap(required = false)]
        devices: Vec<String>,
    },

    /// Monitor device events from kernel and userspace
    #[clap(display_order = 2)]
    Monitor {},

    /// Kill all devmaster workers
    #[clap(display_order = 3)]
    Kill {},

    /// Trigger a fake device action, then the kernel will report an uevent
    #[clap(display_order = 4)]
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
    #[clap(display_order = 5)]
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
    /// Test builtin command on a device
    #[clap(display_order = 6)]
    Hwdb {
        /// update the hardware database
        #[clap(short('u'), long)]
        update: bool,
        /// query database and print result
        #[clap(short('t'), long)]
        test: Option<String>,
        /// Custom .hwdb file path
        #[clap(long, value_parser)]
        path: Option<String>,
        /// generate in /usr/lib/devmaster instead of /etc/devmaster
        #[clap(long)]
        usr: bool,
        /// when updating, return non-zero exit value on any parsing error
        #[clap(short, long)]
        strict: Option<bool>,
        /// alternative root path in the filesystem
        #[clap(short, long)]
        root: Option<String>,
    },
}

/// subcommand for killing workers
fn subcommand_kill() {
    let mut stream = UnixStream::connect(CONTROL_MANAGER_LISTEN_ADDR).unwrap();
    stream.write_all(b"kill ").unwrap();
}

fn main() -> Result<()> {
    let argv: Vec<String> = std::env::args().collect();
    if invoked_as(argv, "devmaster") {
        run_daemon();
        return Ok(());
    }

    init_log_to_console_syslog("devctl", Level::Debug);
    let args = Args::parse();

    match args.subcmd {
        SubCmd::Info {
            query,
            attribute_walk,
            device_id_of_file,
            export,
            export_prefix,
            export_db,
            cleanup_db,
            root,
            devices,
        } => {
            return InfoArgs::new(
                query,
                attribute_walk,
                device_id_of_file,
                export,
                export_prefix,
                export_db,
                cleanup_db,
                root,
                devices,
            )
            .subcommand_info()
        }
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
        SubCmd::Hwdb {
            update,
            test,
            path,
            usr,
            strict,
            root,
        } => subcommand_hwdb(update, test, path, usr, strict, root),
    }

    Ok(())
}
