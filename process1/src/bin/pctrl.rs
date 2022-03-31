use http::Error;
use process1::proto::{
    abi::{unit_comm::Action, CommandRequest},
    ProstClientStream,
};
use std::net::TcpStream;

fn main() -> Result<(), Error> {
    let addr = "127.0.0.1:9527";
    // 连接服务器
    let stream = TcpStream::connect(addr).unwrap();

    let mut client = ProstClientStream::new(stream);

    let cmd = CommandRequest::new_unitcomm(Action::Start, "test.service");

    println!("{:?}", cmd);

    let data = client.execute(cmd).unwrap();

    println!("{:?}", data);

    Ok(())
}
