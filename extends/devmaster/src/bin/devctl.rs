//! devctrl is the client of devmaster
//!
use libdevmaster::*;
use libutils::logger::init_log_with_console;
use log::LevelFilter;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
};

/// subcommand for testing communication
fn subcommand_test(args: Vec<String>) {
    let mut stream = TcpStream::connect(CONTROL_MANAGER_LISTEN_ADDR).unwrap();
    println!("{args:?}");
    let msg = args.join(" ");
    let msg = format!("test {msg}");
    println!("{msg}");
    stream.write_all(msg.as_bytes()).unwrap();
}

/// subcommand for testing worker manager connection
fn subcommand_test_tcp() {
    let mut stream = TcpStream::connect(WORKER_MANAGER_LISTEN_ADDR).unwrap();
    let msg = "test_tcp".to_string();
    println!("{msg}");
    stream.write_all(msg.as_bytes()).unwrap();
}

/// subcommand for killing workers
fn subcommand_kill(args: Vec<String>) {
    let mut stream = TcpStream::connect(CONTROL_MANAGER_LISTEN_ADDR).unwrap();
    println!("{args:?}");
    let msg = args.join(" ");
    let msg = format!("kill {msg}");
    stream.write_all(msg.as_bytes()).unwrap();
}

/// subcommand for worker manager listen
fn subcommand_listen() {
    let listener = TcpListener::bind(WORKER_MANAGER_LISTEN_ADDR).unwrap();
    for i in listener.incoming() {
        let mut stream = i.unwrap();
        let mut msg = String::new();
        stream.read_to_string(&mut msg).unwrap();
        println!("{msg}");
    }
}

fn main() {
    init_log_with_console("devctl", LevelFilter::Debug);
    let mut args = std::env::args();
    match args.nth(1).unwrap().as_str() {
        "test" => subcommand_test(args.collect::<Vec<String>>()),

        "kill" => subcommand_kill(args.collect::<Vec<String>>()),

        "test_tcp" => subcommand_test_tcp(),

        "listen" => subcommand_listen(),

        "monitor" => subcommand_monitor(),

        _ => {
            todo!()
        }
    }
}
