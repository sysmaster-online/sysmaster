//!

use clap::Parser;
use libcmdproto::proto::{
    abi::{sys_comm, unit_comm, CommandRequest},
    mngr_comm, unit_file, ProstClientStream,
};
use libutils::Error;
use libutils::Result;
use std::net::{SocketAddr, TcpStream};

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
    /// [unit] start the unit
    #[clap(display_order = 1)]
    Start { unit_name: Option<String> },

    /// [unit] stop the unit
    #[clap(display_order = 2)]
    Stop { unit_name: Option<String> },

    /// [unit] status of the unit
    #[clap(display_order = 3)]
    Status { unit_name: Option<String> },

    /// [manager] list all units
    ListUnits {},

    /// [system] shutdown the system
    Shutdown {},

    /// manager command
    DaemonReload {},

    /// enable one unit file
    Enable { unit_file: Option<String> },

    /// enable one unit file
    Disable { unit_file: Option<String> },

    /// mask one unit file
    Mask { unit_file: Option<String> },

    /// unmask one unit file
    Unmask { unit_file: Option<String> },
}

enum CommAction {
    Unit(unit_comm::Action),
    Sys(sys_comm::Action),
    File(unit_file::Action),
    Mng(mngr_comm::Action),
}

fn main() -> Result<(), Error> {
    let args = Args::parse();

    let (action, unit_name) = match args.subcmd {
        SubCmd::Start { unit_name } => (CommAction::Unit(unit_comm::Action::Start), unit_name),
        SubCmd::Stop { unit_name } => (CommAction::Unit(unit_comm::Action::Stop), unit_name),
        SubCmd::Status { unit_name } => (CommAction::Unit(unit_comm::Action::Status), unit_name),
        SubCmd::Shutdown {} => (CommAction::Sys(sys_comm::Action::Shutdown), None),
        SubCmd::Enable { unit_file } => (CommAction::File(unit_file::Action::Enable), unit_file),
        SubCmd::Disable { unit_file } => (CommAction::File(unit_file::Action::Disable), unit_file),
        SubCmd::Mask { unit_file } => (CommAction::File(unit_file::Action::Mask), unit_file),
        SubCmd::Unmask { unit_file } => (CommAction::File(unit_file::Action::Unmask), unit_file),
        SubCmd::ListUnits {} => (CommAction::Mng(mngr_comm::Action::Listunits), None),
        _ => unreachable!(),
    };

    let addrs = [
        SocketAddr::from(([127, 0, 0, 1], 9526)),
        SocketAddr::from(([127, 0, 0, 1], 9527)),
    ];
    let stream = TcpStream::connect(&addrs[..]).unwrap();

    let mut client = ProstClientStream::new(stream);

    match action {
        CommAction::Unit(a) => {
            let cmd = CommandRequest::new_unitcomm(a, unit_name.unwrap());
            let data = client.execute(cmd).unwrap();
            if !data.message.is_empty() {
                println!("{}", data.message);
            }
        }
        CommAction::Sys(a) => {
            let cmd = CommandRequest::new_syscomm(a);
            println!("{:?}", cmd);
            let data = client.execute(cmd).unwrap();
            println!("{:?}", data);
        }
        CommAction::File(a) => {
            let cmd = CommandRequest::new_unitfile(a, unit_name.unwrap());
            let data = client.execute(cmd).unwrap();
            if !data.message.is_empty() {
                println!("{}", data.message);
            }
        }
        CommAction::Mng(a) => {
            let cmd = CommandRequest::new_mngrcomm(a);
            let data = client.execute(cmd).unwrap();
            if !data.message.is_empty() {
                println!("{}", data.message);
            }
        }
    }
    Ok(())
}
