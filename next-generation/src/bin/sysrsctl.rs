use clap::{Parser, Subcommand};
use zbus::blocking::Connection;

#[derive(Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Clone, Debug)]
enum Command {
    Start { unit: String },
    Stop { unit: String },
    Restart { unit: String },
}

fn main() {
    let args = Cli::parse();
    let conn = Connection::session().unwrap();
    let dest = Some("org.sysrs.sysrs1");
    let path = "/org/sysrs/sysrs1";
    let iface = Some("org.sysrs.sysrs1");
    let m = match args.command {
        Command::Start { unit } => conn.call_method(dest, path, iface, "StartUnit", &unit),
        Command::Stop { unit } => conn.call_method(dest, path, iface, "StopUnit", &unit),
        Command::Restart { unit } => conn.call_method(dest, path, iface, "RestartUnit", &unit),
    }
    .unwrap();
    let reply: u8 = m.body().unwrap();
    println!("{reply}");
    // let _ = conn.call_method(
    //     Some("org.sysrs.sysrs1"),
    //     "/org/sysrs/sysrs1",
    //     Some("org.sysrs.sysrs1"),
    //     "PrintStore",
    //     &(),
    // );
    // let _ = conn.call_method(
    //     Some("org.sysrs.sysrs1"),
    //     "/org/sysrs/sysrs1",
    //     Some("org.sysrs.sysrs1"),
    //     "PrintState",
    //     &(),
    // );
}
