use prost::bytes::{BufMut, BytesMut};
use prost::Message;
use std::{
    io::{Error, Read, Write},
    rc::Rc,
};

use super::{execute, CommandRequest, CommandResponse, Manager};

/// const MAX_FRAME: usize = 1436;
const MAX_FRAME: usize = 1436;

/// 处理 Frame 的 encode/decode
pub trait FrameCoder
where
    Self: Message + Sized + Default,
{
    /// 把一个 Message encode 成一个 frame
    fn encode_frame(&self, buf: &mut BytesMut) -> Result<(), Error> {
        let size = self.encoded_len();

        if size > MAX_FRAME {
            return Err(std::io::Error::last_os_error());
        }

        self.encode(buf)?;
        Ok(())
    }

    /// 把一个完整的 frame decode 成一个 Message
    fn decode_frame(buf: &mut BytesMut) -> Result<Self, Error> {
        let msg = Self::decode(&buf[..])?;
        Ok(msg)
    }
}

impl FrameCoder for CommandRequest {}
impl FrameCoder for CommandResponse {}

pub fn read_frame<S>(stream: &mut S, buf: &mut BytesMut) -> Result<(), Error>
where
    S: Read + Unpin + Send,
{
    let mut tmp = [0u8; MAX_FRAME];
    let len = stream.read(&mut tmp)?;
    buf.put_slice(&tmp[..len]);
    Ok(())
}

/// 处理服务器端 socket 的读写
pub struct ProstServerStream<S> {
    inner: S,
    manager: Rc<Manager>,
}

/// 处理客户端 socket 的读写
pub struct ProstClientStream<S> {
    inner: S,
}

impl<S> ProstServerStream<S>
where
    S: Read + Write + Unpin + Send,
{
    pub fn new(stream: S, manager: Rc<Manager>) -> Self {
        Self {
            inner: stream,
            manager,
        }
    }

    pub fn process(mut self) -> Result<(), Error> {
        if let Ok(cmd) = self.recv() {
            let res = execute::dispatch(cmd, self.manager.clone());
            self.send(res)?;
        };
        Ok(())
    }

    pub fn send(&mut self, msg: CommandResponse) -> Result<(), Error> {
        let mut buf = BytesMut::new();
        msg.encode_frame(&mut buf)?;
        let encoded = buf.freeze();
        self.inner.write_all(&encoded[..])?;
        self.inner.flush()?;
        Ok(())
    }

    pub fn recv(&mut self) -> Result<CommandRequest, Error> {
        let mut buf = BytesMut::new();
        let stream = &mut self.inner;
        read_frame(stream, &mut buf)?;
        CommandRequest::decode_frame(&mut buf)
    }
}

impl<S> ProstClientStream<S>
where
    S: Read + Write + Unpin + Send,
{
    pub fn new(stream: S) -> Self {
        Self { inner: stream }
    }

    pub fn execute(&mut self, cmd: CommandRequest) -> Result<CommandResponse, Error> {
        self.send(cmd)?;
        Ok(self.recv()?)
    }

    fn send(&mut self, msg: CommandRequest) -> Result<(), Error> {
        let mut buf = BytesMut::new();
        msg.encode_frame(&mut buf)?;
        let encoded = buf.freeze();
        self.inner.write_all(&encoded[..])?;
        self.inner.flush()?;
        Ok(())
    }

    fn recv(&mut self) -> Result<CommandResponse, Error> {
        let mut buf = BytesMut::new();
        let stream = &mut self.inner;
        read_frame(stream, &mut buf)?;
        CommandResponse::decode_frame(&mut buf)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::net::TcpStream;
    use std::thread;
    use std::time::Duration;

    use crate::manager::manager::{Action, Mode};
    use event::Events;

    use super::super::abi::unit_comm::Action as UnitAction;
    use super::*;

    #[test]
    #[should_panic]
    fn test_send_and_recv() {
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(1));
            let addr = "127.0.0.1:9527";
            let stream = TcpStream::connect(addr).unwrap();
            let mut client = ProstClientStream::new(stream);
            let cmd = CommandRequest::new_unitcomm(UnitAction::Start, "test.service");
            let _ = client.execute(cmd).unwrap();
        });

        let fd = std::net::TcpListener::bind("127.0.0.1:9527").unwrap();
        const MODE: Mode = Mode::SYSTEM;
        const ACTION: Action = Action::RUN;
        let _event = Rc::new(RefCell::new(Events::new().unwrap()));
        let manager = Rc::new(Manager::new(MODE, ACTION, Rc::clone(&_event)));
        loop {
            for stream in fd.incoming() {
                match stream {
                    Err(e) => eprintln!("failed: {}", e),
                    Ok(stream) => {
                        panic!("has receive a command request");
                    }
                }
            }
        }
    }
}
