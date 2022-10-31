//! Encapsulate the command request into a frame
use prost::bytes::{BufMut, BytesMut};
use prost::Message;
use std::{
    io::{Error, Read, Write},
    rc::Rc,
};

use super::{execute, CommandRequest, CommandResponse, Manager};

/// const MAX_FRAME: usize = 1436;
const MAX_FRAME: usize = 1436;

/// Frame : encode/decode
pub trait FrameCoder
where
    Self: Message + Sized + Default,
{
    /// Encode message into frame
    fn encode_frame(&self, buf: &mut BytesMut) -> Result<(), Error> {
        let size = self.encoded_len();

        if size > MAX_FRAME {
            return Err(std::io::Error::last_os_error());
        }

        self.encode(buf)?;
        Ok(())
    }

    /// frame decode frame into Message
    fn decode_frame(buf: &mut BytesMut) -> Result<Self, Error> {
        let msg = Self::decode(&buf[..])?;
        Ok(msg)
    }
}

impl FrameCoder for CommandRequest {}
impl FrameCoder for CommandResponse {}

/// read frame from stream
pub fn read_frame<S>(stream: &mut S, buf: &mut BytesMut) -> Result<(), Error>
where
    S: Read + Unpin + Send,
{
    let mut tmp = [0u8; MAX_FRAME];
    let len = stream.read(&mut tmp)?;
    buf.put_slice(&tmp[..len]);
    Ok(())
}

/// Handle read and write of server-side socket
pub struct ProstServerStream<S> {
    inner: S,
    manager: Rc<Manager>,
}

/// Handle read and write of client-side socket
pub struct ProstClientStream<S> {
    inner: S,
}

impl<S> ProstServerStream<S>
where
    S: Read + Write + Unpin + Send,
{
    /// new ProstServerStream
    pub(crate) fn new(stream: S, manager: Rc<Manager>) -> Self {
        Self {
            inner: stream,
            manager,
        }
    }

    /// process frame in server-side
    pub fn process(mut self) -> Result<(), Error> {
        if let Ok(cmd) = self.recv() {
            let res = execute::dispatch(cmd, self.manager.clone());
            self.send(res)?;
        };
        Ok(())
    }

    fn send(&mut self, msg: CommandResponse) -> Result<(), Error> {
        let mut buf = BytesMut::new();
        msg.encode_frame(&mut buf)?;
        let encoded = buf.freeze();
        self.inner.write_all(&encoded[..])?;
        self.inner.flush()?;
        Ok(())
    }

    fn recv(&mut self) -> Result<CommandRequest, Error> {
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
    /// new ProstClientStream
    #[allow(dead_code)]
    pub fn new(stream: S) -> Self {
        Self { inner: stream }
    }

    /// process frame in client-side
    pub fn execute(&mut self, cmd: CommandRequest) -> Result<CommandResponse, Error> {
        self.send(cmd)?;
        self.recv()
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
    use super::super::abi::unit_comm::Action as UnitAction;
    use super::*;
    use std::net::{SocketAddr, TcpStream};
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_send_and_recv() {
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(1));
            let addrs = [
                SocketAddr::from(([127, 0, 0, 1], 9528)),
                SocketAddr::from(([127, 0, 0, 1], 9529)),
            ];
            let stream = TcpStream::connect(&addrs[..]).unwrap();
            let mut client = ProstClientStream::new(stream);
            let cmd = CommandRequest::new_unitcomm(UnitAction::Start, "test.service");
            let _ = client.execute(cmd).unwrap();
        });

        let addrs = [
            SocketAddr::from(([127, 0, 0, 1], 9528)),
            SocketAddr::from(([127, 0, 0, 1], 9529)),
        ];
        let fd = std::net::TcpListener::bind(&addrs[..]).unwrap();
        loop {
            for stream in fd.incoming() {
                match stream {
                    Err(e) => panic!("failed: {}", e),
                    Ok(_stream) => {
                        return;
                    }
                }
            }
        }
    }
}
