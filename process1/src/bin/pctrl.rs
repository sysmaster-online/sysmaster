use clap::Parser;
use std::net::TcpStream;

use process1::proto::{
    abi::{unit_comm::Action, CommandRequest},
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
    start { unit_name: Option<String> },

    /// [unit] stop the unit
    #[clap(display_order = 2)]
    stop { unit_name: Option<String> },

    /// [unit] status of the unit
    #[clap(display_order = 3)]
    status { unit_name: Option<String> },

    /// manager command
    daemon_reload {},
}

fn main() -> Result<(), Error> {
    let args = Args::parse();

    // let cmd = args.subcmd;

    let (action, unit_name) = match args.subcmd {
        SubCmd::start { unit_name } => (Action::Start, unit_name),
        SubCmd::stop { unit_name } => (Action::Stop, unit_name),
        SubCmd::status { unit_name } => (Action::Status, unit_name),
        _ => unreachable!(),
    };
    // let action = Action::from_str(&args.action)?;
    let addr = "127.0.0.1:9527";
    // 连接服务器
    let stream = TcpStream::connect(addr).unwrap();

    let mut client = ProstClientStream::new(stream);

    let cmd = CommandRequest::new_unitcomm(action, unit_name.unwrap());

    println!("{:?}", cmd);

    let data = client.execute(cmd).unwrap();

    println!("{:?}", data);

    Ok(())
}
