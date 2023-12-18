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
use nix::unistd::sleep;
use std::cell::RefCell;
use std::os::unix::net::UnixStream;
use std::os::unix::prelude::RawFd;
use std::rc::{Rc, Weak};

pub(super) struct AliveTimer {
    sub: Rc<AliveTimerSub>,
}

impl AliveTimer {
    pub(super) fn new(eventr: &Rc<Events>) -> AliveTimer {
        AliveTimer {
            sub: AliveTimerSub::new(eventr),
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
    pub(super) fn new(eventr: &Rc<Events>) -> Rc<AliveTimerSub> {
        let sub = Rc::new(AliveTimerSub {
            event: Rc::clone(eventr),
            data: Rc::new(AliveTimerData::new()),
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
}

impl AliveTimerData {
    pub(self) fn new() -> AliveTimerData {
        AliveTimerData {
            sub: RefCell::new(Weak::new()),
        }
    }

    pub(self) fn set_sub(&self, sub: &Rc<AliveTimerSub>) {
        self.sub.replace(Rc::downgrade(sub));
    }

    pub(self) fn sub(&self) -> Rc<AliveTimerSub> {
        self.sub.clone().into_inner().upgrade().unwrap()
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
        60000000
    }

    fn dispatch(&self, _: &Events) -> i32 {
        let socket_path = "/run/sysmaster/init.sock";
        for _ in 0..3 {
            match UnixStream::connect(socket_path) {
                Ok(_) => break,
                Err(e) => {
                    log::error!("Couldn't connect {:?}: {:?}, retry...", socket_path, e);
                    sleep(1);
                    continue;
                }
            };
        }
        self.sub().enable(true)
    }

    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }

    fn description(&self) -> String {
        String::from("AliveTimer")
    }
}
