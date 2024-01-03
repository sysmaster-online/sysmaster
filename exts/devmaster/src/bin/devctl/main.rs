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
use basic::argv::invoked_as;
use clap::Parser;
use daemon::run_daemon;
use libdevmaster::config::devmaster_conf::DEFAULT_CONFIG;
use libdevmaster::framework::control_manager::CONTROL_MANAGER_LISTEN_ADDR;
use log::init_log_to_console_syslog;
use log::Level;
use std::{io::Write, os::unix::net::UnixStream};
use subcmds::hwdb::HwdbArgs;
use subcmds::info::InfoArgs;
use subcmds::monitor::MonitorArgs;
use subcmds::settle::SettleArgs;
use subcmds::test_builtin::subcommand_test_builtin;
use subcmds::trigger::TriggerArgs;
use subcmds::Result;

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
    Monitor {
        /// Print the event properties
        #[clap(short('p'), long)]
        property: bool,

        /// Print the event properties (alias for -p)
        #[clap(short('e'), long)]
        environment: bool,

        /// Print kernel uevents
        #[clap(short('k'), long)]
        kernel: bool,

        /// Print events broadcasted from devmaster other applications in userspace
        #[clap(short('u'), long)]
        userspace: bool,

        /// Filter events by subsystem
        #[clap(short('s'), long)]
        subsystem_match: Option<Vec<String>>,

        /// Filter events by tag
        #[clap(short('t'), long)]
        tag_match: Option<Vec<String>>,
    },

    /// Kill all devmaster workers
    #[clap(display_order = 3)]
    Kill {},

    /// Trigger a fake device action, then the kernel will report an uevent
    #[clap(display_order = 4)]
    Trigger {
        /// the kind of device action to trigger
        #[clap(short('c'), long)]
        action: Option<String>,

        /// Type of events to trigger
        #[clap(short, long, possible_values(&["devices", "subsystems", "all"]), help(
            "Query device information:\n\
                devices                     sysfs devices (default)\n\
                subsystems                  sysfs subsystems and drivers\n\
                all                         sysfs devices, subsystems, and drivers\n")
        )]
        r#type: Option<String>,

        /// Print searched devices by enumerator
        #[clap(short, long)]
        verbose: bool,

        /// Do not actually trigger the device events
        #[clap(short('n'), long)]
        dry_run: bool,

        /// Trigger devices from a matching subsystem
        #[clap(short('s'), long)]
        subsystem_match: Option<Vec<String>>,

        /// Exclude devices from a matching subsystem
        #[clap(short('S'), long)]
        subsystem_nomatch: Option<Vec<String>>,

        /// Trigger devices with a matching attribute
        #[clap(short('a'), long)]
        attr_match: Option<Vec<String>>,

        /// Exclude devices with a matching attribute
        #[clap(short('A'), long)]
        attr_nomatch: Option<Vec<String>>,

        /// Trigger devices with a matching property
        #[clap(short('p'), long)]
        property_match: Option<Vec<String>>,

        /// Trigger devices with a matching tag
        #[clap(short('g'), long)]
        tag_match: Option<Vec<String>>,

        /// Trigger devices with this /sys path
        #[clap(short('y'), long)]
        sysname_match: Option<Vec<String>>,

        /// Trigger devices with this /dev name
        #[clap(long)]
        name_match: Option<Vec<String>>,

        /// Trigger devices with this /sys path
        #[clap(short('b'), long)]
        parent_match: Option<Vec<String>>,

        /// Wait for the triggered events to complete
        #[clap(short('w'), long)]
        settle: bool,

        /// Print synthetic uevent UUID
        #[clap(long)]
        uuid: bool,

        /// the devices to be triggered
        #[clap(required = false)]
        devices: Vec<String>,
    },

    /// Wait for pending devmaster events
    #[clap(display_order = 5)]
    Settle {
        /// Maximum time to wait for events
        #[clap(short('t'), long)]
        timeout: Option<String>,

        /// Stop waiting if file exists
        #[clap(short('E'), long)]
        exit_if_exists: Option<String>,
    },

    /// Test builtin command on a device
    #[clap(display_order = 6)]
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
    /// The sub-command 'hwdb' is deprecated, and is left for backwards compatibility.
    /// Please use sysmaster-hwdb instead.
    #[clap(display_order = 7)]
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
    /// Control the devmaster daemon.
    #[clap(display_order = 7)]
    Control {
        #[clap(short, long)]
        exit: bool,
    },
}

/// subcommand for killing workers
fn subcommand_kill() {
    let mut stream = UnixStream::connect(CONTROL_MANAGER_LISTEN_ADDR).unwrap();
    stream.write_all(b"kill ").unwrap();
}

/// subcommand for controlling devmaster
fn subcommand_control(exit: bool) {
    let mut stream = UnixStream::connect(CONTROL_MANAGER_LISTEN_ADDR).unwrap();

    if exit {
        stream.write_all(b"exit ").unwrap();
    }
}

fn main() -> Result<()> {
    let argv: Vec<String> = std::env::args().collect();
    if invoked_as(argv, "devmaster") {
        run_daemon(DEFAULT_CONFIG);
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
            .subcommand()
        }
        SubCmd::Monitor {
            property,
            environment,
            kernel,
            userspace,
            subsystem_match,
            tag_match,
        } => {
            return MonitorArgs::new(
                property,
                environment,
                kernel,
                userspace,
                subsystem_match,
                tag_match,
            )
            .subcommand()
        }
        SubCmd::Kill {} => subcommand_kill(),
        SubCmd::Trigger {
            action,
            r#type,
            verbose,
            dry_run,
            subsystem_match,
            subsystem_nomatch,
            attr_match,
            attr_nomatch,
            property_match,
            tag_match,
            sysname_match,
            name_match,
            parent_match,
            settle,
            uuid,
            devices,
        } => {
            return TriggerArgs::new(
                action,
                r#type,
                verbose,
                dry_run,
                subsystem_match,
                subsystem_nomatch,
                attr_match,
                attr_nomatch,
                property_match,
                tag_match,
                sysname_match,
                name_match,
                parent_match,
                settle,
                uuid,
                devices,
            )
            .subcommand()
        }
        SubCmd::Settle {
            timeout,
            exit_if_exists,
        } => {
            return SettleArgs::new(timeout, exit_if_exists).subcommand();
        }
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
        } => return HwdbArgs::new(update, test, path, usr, strict, root).subcommand(),
        SubCmd::Control { exit } => subcommand_control(exit),
    }

    Ok(())
}
