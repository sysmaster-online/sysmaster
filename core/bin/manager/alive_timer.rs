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

use event::{EventState, EventType, Events, Source};
use nix::errno::Errno;
use nix::sys::socket::{self, MsgFlags};
use std::cell::RefCell;
use std::os::unix::prelude::RawFd;
use std::rc::{Rc, Weak};
const ALIVE: &str = "ALIVE01234567890";

pub(super) struct AliveTimer {
    sub: Rc<AliveTimerSub>,
}

impl AliveTimer {
    pub(super) fn new(eventr: &Rc<Events>, fd: RawFd) -> AliveTimer {
        AliveTimer {
            sub: AliveTimerSub::new(eventr, fd),
        }
    }

    pub(super) fn enable(&self, enable: bool) -> i32 {
        self.sub.enable(enable)
    }
}

struct AliveTimerSub {
    // associated objects
    event: Rc<Events>,

    // owned objects
    data: Rc<AliveTimerData>,
}

impl AliveTimerSub {
    pub(super) fn new(eventr: &Rc<Events>, fd: RawFd) -> Rc<AliveTimerSub> {
        let sub = Rc::new(AliveTimerSub {
            event: Rc::clone(eventr),
            data: Rc::new(AliveTimerData::new(fd)),
        });
        sub.data.set_sub(&sub);
        sub.register();
        sub
    }

    pub(self) fn enable(&self, enable: bool) -> i32 {
        let source = Rc::clone(&self.data);
        let state = match enable {
            true => EventState::OneShot,
            false => EventState::Off,
        };
        self.event.set_enabled(source, state).unwrap_or(-1)
    }

    fn register(&self) {
        // event
        let source = Rc::clone(&self.data);
        self.event.add_source(source).unwrap();
    }
}

struct AliveTimerData {
    // associated objects
    sub: RefCell<Weak<AliveTimerSub>>,
    alive_fd: RawFd,
}

impl AliveTimerData {
    pub(self) fn new(fd: RawFd) -> AliveTimerData {
        AliveTimerData {
            alive_fd: fd,
            sub: RefCell::new(Weak::new()),
        }
    }

    pub(self) fn set_sub(&self, sub: &Rc<AliveTimerSub>) {
        self.sub.replace(Rc::downgrade(sub));
    }

    pub(self) fn sub(&self) -> Rc<AliveTimerSub> {
        self.sub.clone().into_inner().upgrade().unwrap()
    }

    fn keep_alive(&self) {
        let mut count = 0;
        loop {
            if let Err(err) = socket::send(self.alive_fd, ALIVE.as_bytes(), MsgFlags::MSG_DONTWAIT)
            {
                if Errno::EINTR == err {
                    continue;
                }
                if (Errno::EAGAIN == err || Errno::EWOULDBLOCK == err) && count < 3 {
                    count += 1;
                    continue;
                }
                log::error!("Failed to write ALIVE:{:?}", err);
                return;
            }
            return;
        }
    }
}

impl Source for AliveTimerData {
    fn fd(&self) -> RawFd {
        0
    }

    fn event_type(&self) -> EventType {
        EventType::TimerRealtime
    }

    fn epoll_event(&self) -> u32 {
        (libc::EPOLLIN) as u32
    }

    fn priority(&self) -> i8 {
        0i8
    }

    fn time_relative(&self) -> u64 {
        5000000
    }

    fn dispatch(&self, _: &Events) -> i32 {
        self.keep_alive();
        self.sub().enable(true)
    }

    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }
}
