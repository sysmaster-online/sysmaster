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
use nix::sys::epoll::EpollEvent;
use nix::sys::time::{TimeSpec, TimeValLike};
use nix::sys::timer::Expiration;
use nix::sys::timerfd::{ClockId, TimerFd, TimerFlags, TimerSetTimeFlags};
use nix::unistd;
use std::os::unix::prelude::{AsRawFd, RawFd};
use std::rc::Rc;

pub struct Timer {
    epoll: Rc<Epoll>,
    timer: TimerFd,
    current_cnt: i64,
    time_cnt: i64,
}

impl Timer {
    pub fn new(epoll: &Rc<Epoll>, time_wait: i64, time_cnt: i64) -> Result<Timer, Errno> {
        let timer = TimerFd::new(
            ClockId::CLOCK_REALTIME,
            TimerFlags::TFD_NONBLOCK | TimerFlags::TFD_CLOEXEC,
        )?;
        timer.set(
            Expiration::Interval(TimeSpec::seconds(time_wait)),
            TimerSetTimeFlags::empty(),
        )?;

        epoll.register(timer.as_raw_fd())?;
        Ok(Timer {
            epoll: epoll.clone(),
            timer,
            current_cnt: 0,
            time_cnt,
        })
    }

    fn flush(&self) {
        // The writing method of [0u8; 8] refers to TimerFd::wait()
        if let Err(err) = unistd::read(self.timer.as_raw_fd(), &mut [0u8; 8]) {
            eprintln!("Failed to flush_timer! err:{:?}", err);
            unistd::sleep(1);
        }
    }

    pub fn is_time_out(&mut self, event: EpollEvent) -> Result<bool, Errno> {
        if self.epoll.is_err(event) {
            return Err(Errno::EIO);
        }
        self.flush();
        self.current_cnt += 1;
        if self.time_cnt <= self.current_cnt {
            eprintln!("time out!");
            self.reset();
            return Ok(true);
        }
        Ok(false)
    }

    // reset timer.
    pub fn reset(&mut self) {
        self.current_cnt = 0;
    }

    pub fn fd(&self) -> RawFd {
        self.timer.as_raw_fd()
    }

    pub fn clear(&mut self) {
        self.epoll.safe_close(self.timer.as_raw_fd());
    }
}
