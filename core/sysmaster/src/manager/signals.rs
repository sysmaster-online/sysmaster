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

use core::error::*;
use core::rel::{ReliLastFrame, Reliability};
use event::{EventType, Events, Source};
use nix::sys::signal::Signal;
use nix::sys::signalfd::siginfo;
use std::rc::Rc;

pub(crate) const EVENT_SIGNALS: [Signal; 4] = [
    Signal::SIGCHLD,
    Signal::SIGTERM,
    Signal::SIGINT,
    Signal::SIGHUP,
];

pub(super) struct Signals<T> {
    // associated objects
    reli: Rc<Reliability>,
    signal_handler: T,
}

pub(super) trait SignalDispatcher {
    fn dispatch_signal(&self, signal: &siginfo) -> Result<i32>;
}

impl<T> Signals<T> {
    pub(super) fn new(relir: &Rc<Reliability>, data_handler: T) -> Self {
        Signals {
            reli: Rc::clone(relir),
            signal_handler: data_handler,
        }
    }
}

impl<T: SignalDispatcher> Source for Signals<T> {
    fn event_type(&self) -> EventType {
        EventType::Signal
    }

    fn signals(&self) -> Vec<Signal> {
        Vec::from(EVENT_SIGNALS)
    }

    fn epoll_event(&self) -> u32 {
        (libc::EPOLLIN) as u32
    }

    fn dispatch(&self, e: &Events) -> i32 {
        log::debug!("Dispatching signals!");

        self.reli.set_last_frame1(ReliLastFrame::ManagerOp as u32);
        if let Some(info) = e.read_signals() {
            log::debug!("read signal from event: {:?}", info);
            if let Err(e) = self.signal_handler.dispatch_signal(&info) {
                log::error!("dispatch signal failed : {}", e);
            }
        }
        self.reli.clear_last_frame();

        0
    }

    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }

    fn priority(&self) -> i8 {
        -6i8
    }
}
