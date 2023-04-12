// Copyright (c) 2022 Huawei Technologies Co.,Ltd. All rights reserved.
//
// sysMaster is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan
// PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY
// KIND, EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO
// NON-INFRINGEMENT, MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.

//! Encapsulate the command request into a frame
use crate::error::*;
use prost::bytes::{BufMut, BytesMut};
use prost::Message;
use std::{
    io::{Read, Write},
    rc::Rc,
};

use super::execute::ExecuterAction;
use super::{execute, CommandRequest, CommandResponse};

/// Reading buffer size used in `fn read` of `std::io::Read`
const MAX_FRAME: usize = 1024;
/// The length of u8 to represent usize
const USIZE_TO_U8_LENGTH: usize = 8;

/// Frame : encode/decode
pub trait FrameCoder
where
    Self: Message + Sized + Default,
{
    /// Encode message into frame
    fn encode_frame(&self, buf: &mut BytesMut) -> Result<()> {
        self.encode(buf).context(EncodeSnafu)?;
        Ok(())
    }

    /// frame decode frame into Message
    fn decode_frame(buf: &mut BytesMut) -> Result<Self> {
        let msg = Self::decode(&buf[..]).context(DecodeSnafu)?;
        Ok(msg)
    }
}

impl FrameCoder for CommandRequest {}
impl FrameCoder for CommandResponse {}

/// read frame from stream
pub fn read_frame<S>(stream: &mut S, buf: &mut BytesMut) -> Result<()>
where
    S: Read + Unpin + Send,
{
    // 1. Got the message length
    let mut msg_len = [0_u8; USIZE_TO_U8_LENGTH];
    stream.read_exact(&mut msg_len).context(IoSnafu)?;
    let msg_len = get_msg_len(msg_len);

    // 2. Got the message
    let mut tmp = vec![0; MAX_FRAME];
    let mut cur_len: usize = 0;
    loop {
        match stream.read(&mut tmp) {
            Ok(len) => {
                cur_len += len;
                buf.put_slice(&tmp[..len]);
                if len < MAX_FRAME || cur_len >= msg_len {
                    break;
                }
            }
            Err(e) => {
                return Err(Error::ReadStream { msg: e.to_string() });
            }
        }
    }
    Ok(())
}

fn msg_len_vec(len: usize) -> [u8; USIZE_TO_U8_LENGTH] {
    let res = len.to_le_bytes();
    assert_eq!(res.len(), USIZE_TO_U8_LENGTH);
    res
}

fn get_msg_len(message: [u8; USIZE_TO_U8_LENGTH]) -> usize {
    usize::from_le_bytes(message)
}

/// Handle read and write of server-side socket
pub struct ProstServerStream<S, T> {
    inner: S,
    manager: Rc<T>,
}

/// Handle read and write of client-side socket
pub struct ProstClientStream<S> {
    inner: S,
}

impl<S, T> ProstServerStream<S, T>
where
    S: Read + Write + Unpin + Send,
    T: ExecuterAction,
{
    /// new ProstServerStream
    pub fn new(stream: S, manager: Rc<T>) -> Self {
        Self {
            inner: stream,
            manager,
        }
    }

    /// process frame in server-side
    pub fn process(mut self) -> Result<()> {
        if let Ok(cmd) = self.recv() {
            let res = execute::dispatch(cmd, Rc::clone(&self.manager));
            self.send(res)?;
        };
        Ok(())
    }

    fn send(&mut self, msg: CommandResponse) -> Result<()> {
        let mut buf = BytesMut::new();
        msg.encode_frame(&mut buf)?;
        let encoded = buf.freeze();
        let msg_len = msg_len_vec(encoded.len());
        self.inner.write_all(&msg_len).context(IoSnafu)?;
        self.inner.write_all(&encoded).context(IoSnafu)?;
        self.inner.flush().context(IoSnafu)?;
        Ok(())
    }

    fn recv(&mut self) -> Result<CommandRequest> {
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
    pub fn execute(&mut self, cmd: CommandRequest) -> Result<CommandResponse> {
        self.send(cmd)?;
        self.recv()
    }

    fn send(&mut self, msg: CommandRequest) -> Result<()> {
        let mut buf = BytesMut::new();
        msg.encode_frame(&mut buf)?;
        let encoded = buf.freeze();
        let msg_len = msg_len_vec(encoded.len());
        self.inner.write_all(&msg_len).context(IoSnafu)?;
        self.inner.write_all(&encoded).context(IoSnafu)?;
        self.inner.flush().context(IoSnafu)?;
        Ok(())
    }

    fn recv(&mut self) -> Result<CommandResponse> {
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
            let cmd =
                CommandRequest::new_unitcomm(UnitAction::Start, vec!["test.service".to_string()]);
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
                    Err(e) => panic!("failed: {e}"),
                    Ok(_stream) => return,
                }
            }
        }
    }
}
