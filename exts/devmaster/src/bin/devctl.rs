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
use basic::logger::init_log_with_console;
use clap::Parser;
use libdevmaster::{
    control_manager::CONTROL_MANAGER_LISTEN_ADDR, devctl_monitor::subcommand_monitor,
    devctl_trigger::subcommand_trigger,
};
use log::LevelFilter;
use std::{io::Write, net::TcpStream};

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

    /// Send a fake device to devmaster
    #[clap(display_order = 3)]
    Test {
        /// the device name to be sent
        #[clap(required = true)]
        devname: String,
    },

    /// Trigger a fake device action, then the kernel will report an uevent
    #[clap(display_order = 4)]
    Trigger {
        /// the kind of device action to trigger
        #[clap(short, long)]
        action: Option<String>,

        /// the devices to be triggered
        #[clap(required = false)]
        devices: Vec<String>,
    },
}

/// subcommand for testing communication
fn subcommand_test(devname: String) {
    let mut stream = TcpStream::connect(CONTROL_MANAGER_LISTEN_ADDR).unwrap();
    let msg = format!("test {devname}");
    stream.write_all(msg.as_bytes()).unwrap();
}

/// subcommand for killing workers
fn subcommand_kill() {
    let mut stream = TcpStream::connect(CONTROL_MANAGER_LISTEN_ADDR).unwrap();
    stream.write_all(b"kill ").unwrap();
}

fn main() {
    init_log_with_console("devctl", LevelFilter::Debug);
    let args = Args::parse();

    match args.subcmd {
        SubCmd::Monitor {} => subcommand_monitor(),
        SubCmd::Kill {} => subcommand_kill(),
        SubCmd::Test { devname } => subcommand_test(devname),
        SubCmd::Trigger { action, devices } => subcommand_trigger(devices, action),
    }
}
