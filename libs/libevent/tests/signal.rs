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

use libevent::EventState;
// These tests cannot run as a regular test because cargo would spawn a thread to run it,
// failing the signal masking. So we make our own, non-threaded harnessing
use libevent::EventType;
use libevent::Events;
use libevent::Source;
use nix::unistd::fork;
use nix::unistd::ForkResult;
use std::rc::Rc;

#[derive(Debug)]
struct Signals {}

impl Signals {
    fn new() -> Signals {
        Signals {}
    }
}

impl Source for Signals {
    fn event_type(&self) -> EventType {
        EventType::Signal
    }

    fn signals(&self) -> Vec<libc::c_int> {
        vec![libc::SIGCHLD, libc::SIGTERM]
    }

    fn epoll_event(&self) -> u32 {
        (libc::EPOLLIN | libc::EPOLLONESHOT) as u32
    }

    fn priority(&self) -> i8 {
        0i8
    }

    fn dispatch(&self, e: &Events) -> i32 {
        println!("Dispatching signal!");
        match e.read_signals() {
            Ok(Some(info)) => {
                println!("read signo: {:?}", info.si_signo);
            }
            Ok(None) => (),
            Err(e) => {
                println!("{e:?}");
            }
        }
        0
    }

    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }
}

fn main() {
    let e = Events::new().unwrap();
    let s: Rc<dyn Source> = Rc::new(Signals::new());
    e.add_source(s.clone()).unwrap();
    e.set_enabled(s.clone(), EventState::OneShot).unwrap();

    let pid = unsafe { fork() };
    match pid {
        Ok(ForkResult::Parent { child, .. }) => {
            println!("Continuing execution in parent process, new child has pid: {child}");
            e.run(-1).unwrap();
        }
        Ok(ForkResult::Child) => println!("I'm a new child process"),
        Err(_) => println!("Fork failed"),
    }
}
