//! devctrl is the client of devmaster
//!
// use kobject_uevent::UEvent;
use libdevmaster::*;
use netlink_sys::{protocols::NETLINK_KOBJECT_UEVENT, Socket, SocketAddr};
use std::os::unix::prelude::AsRawFd;
use std::process;
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

/// subcommand for monitoring uevent
fn subcommand_monitor() {
    let mut socket = Socket::new(NETLINK_KOBJECT_UEVENT).unwrap();
    match libutils::socket_util::set_receive_buffer_force(socket.as_raw_fd(), 1024 * 1024 * 128) {
        Ok(()) => {}
        Err(errno) => {
            println!("Failed to set receive buffer: {errno}");
        }
    }

    let sa = SocketAddr::new(process::id(), 1);
    socket.bind(&sa).unwrap();

    loop {
        let mut buf = vec![0; 1024 * 8];
        let n = socket.recv(&mut &mut buf[..], 0).unwrap();
        let s = std::str::from_utf8(&buf[..n]).unwrap();
        // let u = UEvent::from_netlink_packet(&buf[..n]).unwrap();
        println!(">> {s}");
        // println!("{:#?}", u);

        // let lines = s.split("\0");
        // for line in lines {
        //     println!("{}", line);
        // }
    }
}

fn main() {
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
