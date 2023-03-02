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
use nix::errno::Errno;
use nix::sys::socket::{self, sockopt, AddressFamily, SockFlag, SockType, UnixAddr};
use nix::sys::time::{TimeSpec, TimeVal, TimeValLike};
use nix::sys::timer::Expiration;
use nix::sys::timerfd::{ClockId, TimerFd, TimerFlags, TimerSetTimeFlags};
use std::os::unix::prelude::{AsRawFd, RawFd};
use std::rc::Rc;
use std::{fs, path::PathBuf};

const LISTEN_BACKLOG: usize = 10;
const SOCKET_TIMEOUT: i64 = 10;
const INVALID_FD: i32 = -1;
const ACCEPT_COUNT: i32 = 3;
const INIT_SOCKET: &str = "/run/sysmaster/init";

pub struct Alive {
    pub epoll: Rc<Epoll>,
    pub alive_fd: RawFd,
    pub connect_fd: RawFd,
    pub time_fd: RawFd,
    pub manager_time_count: i64,
    pub alive_time_count: i64,
    pub time_out: i64,
    pub time_wait: i64,
    timer: TimerFd,
}

impl Alive {
    pub fn new(epoll: &Rc<Epoll>, time_wait: i64, time_out: i64) -> Result<Alive, Errno> {
        let timer = TimerFd::new(ClockId::CLOCK_REALTIME, TimerFlags::empty())?;
        Ok(Alive {
            epoll: epoll.clone(),
            alive_fd: INVALID_FD,
            connect_fd: INVALID_FD,
            time_fd: INVALID_FD,
            manager_time_count: 0,
            alive_time_count: 0,
            time_out,
            time_wait,
            timer,
        })
    }

    pub fn init(&mut self) -> Result<(), Errno> {
        self.timer.set(
            Expiration::Interval(TimeSpec::seconds(self.time_wait)),
            TimerSetTimeFlags::empty(),
        )?;
        self.time_fd = self.timer.as_raw_fd();
        self.epoll.register(self.time_fd)?;

        let sock_path = PathBuf::from(INIT_SOCKET);
        self.alive_fd = socket::socket(
            AddressFamily::Unix,
            SockType::Stream,
            SockFlag::empty(),
            None,
        )?;

        let parent_path = sock_path.as_path().parent();
        match parent_path {
            Some(path) => fs::create_dir_all(path)
                .map_err(|e| Errno::from_i32(e.raw_os_error().unwrap_or(Errno::EINVAL as i32)))?,
            None => return Err(Errno::EINVAL),
        }

        if let Err(e) = nix::unistd::unlink(&sock_path) {
            println!("Failed to unlink path:{:?}, error:{}", sock_path, e);
        }

        let addr = UnixAddr::new(&sock_path)?;
        socket::bind(self.alive_fd, &addr)?;
        socket::listen(self.alive_fd, LISTEN_BACKLOG)
    }

    pub fn wait_connect(&mut self) -> Result<(), Errno> {
        if self.connect_fd >= 0 {
            self.del_connect_epoll()?;
        }

        if let Err(err) = self.wait_alive() {
            println!("Failed to wait_alive:{:?}", err);
            return Err(err);
        }

        self.epoll.register(self.connect_fd)
    }

    pub fn del_connect_epoll(&mut self) -> Result<(), Errno> {
        if let Err(err) = self.epoll.unregister(self.connect_fd) {
            println!("Failed to del_connect_epoll:{:?}", err);
            return Err(err);
        }
        self.connect_fd = INVALID_FD;
        Ok(())
    }

    fn wait_alive(&mut self) -> Result<(), Errno> {
        let timeval = TimeVal::seconds(SOCKET_TIMEOUT);
        socket::setsockopt(self.alive_fd, sockopt::ReceiveTimeout, &timeval)?;

        let mut count = 0;
        loop {
            match socket::accept(self.alive_fd) {
                Ok(fd) => {
                    self.connect_fd = fd;
                    return Ok(());
                }
                Err(err) => {
                    if Errno::EINTR == err {
                        continue;
                    }
                    if count < ACCEPT_COUNT && (Errno::EWOULDBLOCK == err || Errno::EAGAIN == err) {
                        println!("Failed to wait_alive!: {:?}", err);
                        count += 1;
                    } else {
                        return Err(err);
                    }
                }
            }
        }
    }

    pub fn recv_buf(&self) -> Result<String, Errno> {
        self.epoll.recv_nowait(self.connect_fd)
    }

    pub fn is_manageable(&self, buf: &String) -> bool {
        "MANAGEABLE" == buf
    }

    pub fn is_alive(&self, buf: &String) -> bool {
        "ALIVE" == buf
    }

    pub fn is_unmanageable(&self, buf: &String) -> bool {
        "UNMANAGEABLE" == buf
    }
}
