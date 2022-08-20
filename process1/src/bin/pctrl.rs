use clap::Parser;
use std::net::TcpStream;

use process1::proto::{
    abi::{sys_comm, unit_comm, CommandRequest},
    ProstClientStream,
};
use utils::Error;
use utils::Result;

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

    /// [system] shutdown the system
    Shutdown {},

    /// manager command
    DaemonReload {},
}

enum CommAction {
    UnitAction(unit_comm::Action),
    SysAction(sys_comm::Action),
}

fn main() -> Result<(), Error> {
    let args = Args::parse();

    let (action, unit_name) = match args.subcmd {
        SubCmd::Start { unit_name } => {
            (CommAction::UnitAction(unit_comm::Action::Start), unit_name)
        }
        SubCmd::Stop { unit_name } => (CommAction::UnitAction(unit_comm::Action::Stop), unit_name),
        SubCmd::Status { unit_name } => {
            (CommAction::UnitAction(unit_comm::Action::Status), unit_name)
        }
        SubCmd::Shutdown {} => (CommAction::SysAction(sys_comm::Action::Shutdown), None),
        _ => unreachable!(),
    };

    let addr = "127.0.0.1:9527";
    // 连接服务器
    let stream = TcpStream::connect(addr).unwrap();

    let mut client = ProstClientStream::new(stream);

    match action {
        CommAction::UnitAction(a) => {
            let cmd = CommandRequest::new_unitcomm(a, unit_name.unwrap());
            println!("{:?}", cmd);
            let data = client.execute(cmd).unwrap();
            println!("{:?}", data);
        }
        CommAction::SysAction(a) => {
            let cmd = CommandRequest::new_syscomm(a);
            println!("{:?}", cmd);
            let data = client.execute(cmd).unwrap();
            println!("{:?}", data);
        }
    }
    Ok(())
}
