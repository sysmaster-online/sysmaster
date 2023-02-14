//! devctrl is the client of devmaster
//!
use clap::Parser;
use libdevmaster::*;
use libutils::logger::init_log_with_console;
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
    }
}
