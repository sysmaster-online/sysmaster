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

use super::epoll::Epoll;
use super::timer::Timer;
use nix::errno::Errno;
use nix::sys::epoll::EpollEvent;
use nix::sys::inotify::{AddWatchFlags, InitFlags, Inotify, WatchDescriptor};
use nix::sys::socket::{self, AddressFamily, SockFlag, SockType, UnixAddr};
use nix::sys::stat::{self, Mode};
use nix::unistd;
use std::os::unix::io::AsRawFd;
use std::os::unix::prelude::RawFd;
use std::rc::Rc;
use std::{fs, path::PathBuf, str};

const LISTEN_BACKLOG: usize = 10;
const ACCEPT_COUNT: i32 = 3;
const BUF_SIZE: usize = 16; //The communication string length is fixed to 16 characters.
use constants::{ALIVE, INIT_SOCKET, INVALID_FD};

pub struct Comm {
    epoll: Rc<Epoll>,
    listen_fd: RawFd,
    connect_fd: RawFd,
    online_fd: RawFd, // Specsify either listen_fd or connect_fd.
    timer: Timer,
    inotify: Inotify,
    wd: WatchDescriptor,
}

#[derive(PartialEq, Eq)]
pub enum CommType {
    Succeed,
    PipON,
    PipOFF,
    PipTMOUT,
}

impl Comm {
    pub fn new(epoll: &Rc<Epoll>, time_wait: i64, time_cnt: i64) -> Result<Comm, Errno> {
        let timer = Timer::new(epoll, time_wait, time_cnt)?;

        let (listen_fd, inotify, wd) = create_listen_fd(epoll)?;

        let mut comm = Comm {
            epoll: epoll.clone(),
            listen_fd,
            connect_fd: INVALID_FD,
            online_fd: INVALID_FD,
            timer,
            inotify,
            wd,
        };

        comm.set_online_fd(comm.listen_fd)?;
        Ok(comm)
    }

    pub fn is_fd(&self, fd: RawFd) -> bool {
        fd == self.online_fd || fd == self.timer.fd() || fd == self.inotify.as_raw_fd()
    }

    pub fn proc(&mut self, event: EpollEvent) -> CommType {
        if self.timer.fd() as u64 == event.data() {
            if self.timer.is_time_out(event) {
                return CommType::PipTMOUT;
            }
            return CommType::Succeed;
        }

        if self.inotify.as_raw_fd() as u64 == event.data() {
            // Dont self.inotify.read_events(), because if recover fails, event can be retrieved to recover again.
            return self.recover();
        }

        // When the program runs normally, listen_fd will not be closed,
        // but connect_fd will be closed during listening.
        if self.listen_fd as u64 == event.data() && self.epoll.is_err(event) {
            return self.recover();
        }

        if self.online_fd as u64 == event.data() {
            match self.online_fd {
                x if x == self.listen_fd => return self.listen_proc(),
                x if x == self.connect_fd => return self.connect_proc(event),
                _ => {}
            }
        }
        CommType::Succeed
    }

    pub fn finish(&mut self) {
        _ = self.set_online_fd(self.listen_fd);
        self.timer.reset();
    }

    fn connect(&mut self) -> Result<(), Errno> {
        let mut count = 0;
        loop {
            match socket::accept4(
                self.listen_fd,
                SockFlag::SOCK_CLOEXEC | SockFlag::SOCK_NONBLOCK,
            ) {
                Ok(fd) => {
                    self.connect_fd = fd;
                    return self.set_online_fd(self.connect_fd);
                }
                Err(err) => {
                    if Errno::EINTR == err {
                        continue;
                    }
                    if count < ACCEPT_COUNT && (Errno::EWOULDBLOCK == err || Errno::EAGAIN == err) {
                        count += 1;
                    } else {
                        eprintln!("Failed to connect!: {:?}", err);
                        return Err(err);
                    }
                }
            }
        }
    }

    fn recv_msg(&self) -> Result<String, Errno> {
        let mut buffer = [0u8; BUF_SIZE];
        let mut count = 0;
        loop {
            let buflen =
                match socket::recv(self.connect_fd, &mut buffer, socket::MsgFlags::MSG_DONTWAIT) {
                    Ok(len) => len,
                    Err(err) => {
                        if Errno::EINTR == err {
                            continue;
                        }
                        if (Errno::EAGAIN == err || Errno::EWOULDBLOCK == err) && count < 3 {
                            count += 1;
                            continue;
                        }
                        return Err(err);
                    }
                };

            match str::from_utf8(&buffer[..buflen]) {
                Ok(v) => return Ok(v.to_string()),
                _ => return Err(Errno::EINVAL),
            }
        }
    }

    fn is_alive(&self, buf: &String) -> bool {
        ALIVE == buf
    }

    fn set_online_fd(&mut self, fd: RawFd) -> Result<(), Errno> {
        if self.online_fd == fd {
            return Ok(());
        }

        match fd {
            x if x == self.listen_fd => self.do_listen_set(),
            x if x == self.connect_fd => self.do_connect_set(),
            _ => Ok(()),
        }
    }

    fn do_listen_set(&mut self) -> Result<(), Errno> {
        if self.connect_fd != INVALID_FD {
            if let Err(err) = self.epoll.unregister(self.connect_fd) {
                eprintln!("Failed to unregister connect_fd:{:?}", err);
            }
            self.epoll.safe_close(self.connect_fd);
            self.connect_fd = INVALID_FD;
        }
        if let Err(err) = self.epoll.register(self.listen_fd) {
            eprintln!("Failed to register listen_fd:{:?}", err);
            return Err(err);
        }
        self.online_fd = self.listen_fd;
        Ok(())
    }

    fn do_connect_set(&mut self) -> Result<(), Errno> {
        if self.listen_fd != INVALID_FD {
            if let Err(err) = self.epoll.unregister(self.listen_fd) {
                eprintln!("Failed to unregister listen_fd:{:?}", err);
                return Err(err);
            }
        }

        if let Err(err) = self.epoll.register(self.connect_fd) {
            eprintln!("Failed to register connect_fd:{:?}", err);
            return Err(err);
        }
        self.online_fd = self.connect_fd;
        Ok(())
    }

    fn listen_proc(&mut self) -> CommType {
        if self.connect().is_err() {
            return CommType::PipOFF;
        }
        self.timer.reset();
        CommType::PipON
    }

    fn connect_proc(&mut self, event: EpollEvent) -> CommType {
        if self.epoll.is_err(event) {
            _ = self.set_online_fd(self.listen_fd);
            return CommType::PipOFF;
        }
        match self.recv_msg() {
            Ok(buf) => {
                if self.is_alive(&buf) {
                    self.timer.reset();
                } else {
                    eprintln!("msg is invalid! {:?}", buf);
                }
            }
            Err(err) => {
                eprintln!("Failed to recv_msg {:?}", err);
                unistd::sleep(1);
            }
        }
        CommType::Succeed
    }

    fn recover(&mut self) -> CommType {
        match create_listen_fd(&self.epoll) {
            Ok((listen_fd, inotify, wd)) => {
                self.epoll.safe_close(self.listen_fd);
                self.epoll.safe_close(self.inotify.as_raw_fd());
                self.listen_fd = listen_fd;
                self.inotify = inotify;
                self.wd = wd;
                eprintln!("comm recover");
                if self.online_fd == self.connect_fd {
                    // The socket file(INIT_SOCKET) cannot be used when connecting,
                    // so recreate the socket file(INIT_SOCKET) and return success.
                    return CommType::Succeed;
                } else {
                    // If init is in the listening state,
                    // the sysmaster cannot be connected through the old socket file(INIT_SOCKET) at this time,
                    // we must recreate the socket file and then reexec the sysmaster.
                    return CommType::PipTMOUT;
                }
            }
            Err(e) => {
                eprintln!("Failed to create_listen_fd:{:?}", e);
            }
        }
        CommType::Succeed
    }
}

impl Drop for Comm {
    fn drop(&mut self) {
        self.epoll.safe_close(self.listen_fd);
        self.listen_fd = INVALID_FD;

        self.epoll.safe_close(self.connect_fd);
        self.connect_fd = INVALID_FD;

        let _ = self.inotify.rm_watch(self.wd);
        self.epoll.safe_close(self.inotify.as_raw_fd());
    }
}

fn create_listen_fd(epoll: &Rc<Epoll>) -> Result<(i32, Inotify, WatchDescriptor), Errno> {
    let listen_fd = socket::socket(
        AddressFamily::Unix,
        SockType::Stream,
        SockFlag::SOCK_CLOEXEC,
        None,
    )?;

    // create '/run/sysmaster' with mode 755
    let sock_path = PathBuf::from(INIT_SOCKET);
    let path = match sock_path.as_path().parent() {
        None => return Err(Errno::EINVAL),
        Some(v) => v,
    };
    let old_mask = stat::umask(Mode::from_bits_truncate(!0o755));
    let ret = fs::create_dir_all(path);
    let _ = stat::umask(old_mask);
    if let Err(e) = ret {
        eprintln!("Failed to create directory {path:?}: {e}");
        return Err(Errno::from_i32(
            e.raw_os_error().unwrap_or(Errno::EINVAL as i32),
        ));
    }

    if let Err(e) = unistd::unlink(&sock_path) {
        eprintln!("Failed to unlink path:{:?}, error:{}", sock_path, e);
    }

    // create '/run/sysmaster/init' with mode 600
    let addr = UnixAddr::new(&sock_path)?;
    let old_mask = stat::umask(Mode::from_bits_truncate(!0o600));
    let ret = socket::bind(listen_fd, &addr);
    let _ = stat::umask(old_mask);
    if let Err(e) = ret {
        eprintln!("Failed to bind socket {sock_path:?}: {e}");
        return Err(e);
    }
    socket::listen(listen_fd, LISTEN_BACKLOG)?;

    let inotify = Inotify::init(InitFlags::all())?;

    let wd = inotify.add_watch(INIT_SOCKET, AddWatchFlags::IN_ALL_EVENTS)?;

    epoll.register(inotify.as_raw_fd())?;
    Ok((listen_fd, inotify, wd))
}
