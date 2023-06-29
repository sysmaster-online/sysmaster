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
    time_wait: i64,
    time_cnt: i64,
}

impl Timer {
    pub fn new(epoll: &Rc<Epoll>, time_wait: i64, time_cnt: i64) -> Result<Timer, Errno> {
        let timer = create_timer(epoll, time_wait)?;
        Ok(Timer {
            epoll: epoll.clone(),
            timer,
            current_cnt: 0,
            time_wait,
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

    #[allow(clippy::wrong_self_convention)]
    pub fn is_time_out(&mut self, event: EpollEvent) -> bool {
        if self.epoll.is_err(event) {
            return self.recover();
        }
        self.flush();
        self.current_cnt += 1;
        if self.time_cnt <= self.current_cnt {
            eprintln!("time out!");
            self.reset();
            return true;
        }
        false
    }

    // reset timer.
    pub fn reset(&mut self) {
        self.current_cnt = 0;
    }

    pub fn fd(&self) -> RawFd {
        self.timer.as_raw_fd()
    }

    fn recover(&mut self) -> bool {
        match create_timer(&self.epoll, self.time_wait) {
            Ok(timer) => {
                // After successfully creating a new timer, recycle the old timer so that
                // if create_timer fails, event can be retrieved to create_timer again.
                // timer have drop, no need to manually close timer.
                self.timer = timer;
                eprintln!("timer recover");
            }
            Err(e) => {
                eprintln!("Failed to create_timer:{:?}", e);
            }
        }
        // Here we believe that the system has encountered an exception, set it to timeout
        true
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        // self.timer does not need to drop, because TimerFd have drop.
    }
}

fn create_timer(epoll: &Rc<Epoll>, time_wait: i64) -> Result<TimerFd, Errno> {
    let timer = TimerFd::new(
        ClockId::CLOCK_REALTIME,
        TimerFlags::TFD_NONBLOCK | TimerFlags::TFD_CLOEXEC,
    )?;
    timer.set(
        Expiration::Interval(TimeSpec::seconds(time_wait)),
        TimerSetTimeFlags::empty(),
    )?;

    epoll.register(timer.as_raw_fd())?;
    Ok(timer)
}
