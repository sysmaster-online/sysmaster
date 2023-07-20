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

//! # An event scheduling framework based on epoll
//!
//! Support events such as io/signal/pidfd/child/timer/defer/post/exit.
//!
//! When multiple events are registered, the events framework will schedule them cyclically.
//!
//! The event source needs to implement the Source trait to be managed by the events framework.
//!
//! # Example:
//! ```rust
//! # use std::{
//! #     net::{TcpListener, TcpStream},
//! #     os::unix::io::{AsRawFd, RawFd},
//! #     rc::Rc};
//! # use event::Error;
//! #
//! # use std::thread;
//! # use std::time::Duration;
//! #
//! # use event::Events;
//! # use event::Source;
//! # use event::{EventState, EventType};
//! #
//! /// Define one struct, implement Source trait
//! #[derive(Debug)]
//! struct Io {
//!     t: TcpStream,
//! }
//!
//! impl Io {
//!     fn new(s: &'static str) -> Io {
//!         Io {
//!             t: TcpStream::connect(s).unwrap(),
//!         }
//!     }
//! }
//!
//! impl Source for Io {
//!     fn fd(&self) -> RawFd {
//!         self.t.as_raw_fd()
//!     }
//!
//!     fn event_type(&self) -> EventType {
//!         EventType::Io
//!     }
//!
//!     fn epoll_event(&self) -> u32 {
//!         (libc::EPOLLIN) as u32
//!     }
//!
//!     /// Set the priority, -127i8 ~ 128i8, the smaller the value, the higher the priority
//!     fn priority(&self) -> i8 {
//!         0i8
//!     }
//!
//!     /// start dispatching after the event arrives
//!     fn dispatch(&self, _: &Events) -> i32 {
//!         println!("Dispatching IO!");
//!         0
//!     }
//!
//!     /// Unless you can guarantee all types of token allocation, it is recommended to use the default implementation here
//!     fn token(&self) -> u64 {
//!         let data: u64 = unsafe { std::mem::transmute(self) };
//!         data
//!     }
//! }
//!
//! fn main() {
//!     /// Simulate the monitoring of a network communication event
//!     thread::spawn(move || {
//!         let listener = TcpListener::bind("0.0.0.0:9098").unwrap();
//!         loop {
//!             let (_stream, addr) = listener.accept().unwrap();
//!             println!("Accepted a new connection: {}", addr);
//!         }
//!     });
//!
//!     thread::sleep(Duration::from_millis(100));
//!
//!     /// Create event scheduling framework
//!     let mut e = Events::new().unwrap();
//!
//!     /// Create event type
//!     let s: Rc<dyn Source> = Rc::new(Io::new("0.0.0.0:9098"));
//!
//!     /// Added to the scheduling framework
//!     e.add_source(s.clone()).unwrap();
//!
//!     /// Scheduling
//!     e.set_enabled(s.clone(), EventState::OneShot).unwrap();
//!
//!     /// One time scheduling, you can also use rloop() to keep cyclic scheduling
//!     e.run(100).unwrap();
//!
//!     /// After the event is deleted, the event will no longer be dispatched
//!     e.del_source(s.clone()).unwrap();
//! }
//! ```
//!
pub mod error;
pub mod events;
pub mod poll;
pub mod source;
mod timer;

pub use crate::events::Events;
pub(crate) use crate::poll::Poll;
pub use crate::source::Source;
pub use error::*;

/// Supports event types added to the frame
/// An event scheduling framework based on epoll
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone)]
pub enum EventType {
    /// Io type
    Io,
    /// realtime timer
    TimerRealtime,
    /// boottime timer
    TimerBoottime,
    /// monotonic timer
    TimerMonotonic,
    /// realtime alarm timer
    TimerRealtimeAlarm,
    /// boottime alarm timer
    TimerBoottimeAlarm,
    /// Signal
    Signal,
    /// child process
    Child,
    /// process
    Pidfd,
    /// Watchdog
    Watchdog,
    /// Inotify monitoring
    Inotify,
    /// Defer event, executed once per LOOP
    Defer,
    /// Post event
    Post,
    /// exit event
    Exit,
}

/// The scheduling status of the event
/// The dispatch status of the event
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum EventState {
    /// Start scheduling
    On,
    /// Close scheduling
    Off,
    /// Stop after dispatching once
    OneShot,
}

#[cfg(test)]
mod tests {
    use libtests::get_target_test_dir;
    use nix::sys::inotify::AddWatchFlags;

    use super::*;
    use std::{
        fs::File,
        net::{TcpListener, TcpStream},
        os::unix::prelude::{AsRawFd, RawFd},
        rc::Rc,
        thread,
        time::Duration,
    };

    #[test]
    fn test_event_type_hash() {
        let mut hash_set = std::collections::HashSet::new();
        hash_set.insert(EventType::Io);
        hash_set.insert(EventType::TimerRealtime);
        hash_set.insert(EventType::TimerBoottime);
        hash_set.insert(EventType::TimerMonotonic);
        hash_set.insert(EventType::TimerRealtimeAlarm);
        hash_set.insert(EventType::TimerBoottimeAlarm);
        hash_set.insert(EventType::Signal);
        hash_set.insert(EventType::Child);
        hash_set.insert(EventType::Pidfd);
        hash_set.insert(EventType::Watchdog);
        hash_set.insert(EventType::Inotify);
        hash_set.insert(EventType::Defer);
        hash_set.insert(EventType::Post);
        hash_set.insert(EventType::Exit);
        assert_eq!(hash_set.len(), 14);
    }

    #[test]
    fn test_event_state_clone() {
        let state = EventState::On;
        let cloned = state.clone();
        assert_eq!(state, cloned);
    }

    // io test
    struct Io {
        t: TcpStream,
    }

    impl Io {
        fn new(s: &'static str) -> Io {
            Io {
                t: TcpStream::connect(s).unwrap(),
            }
        }
    }

    impl Source for Io {
        fn fd(&self) -> RawFd {
            self.t.as_raw_fd()
        }

        fn event_type(&self) -> EventType {
            EventType::Io
        }

        fn epoll_event(&self) -> u32 {
            (libc::EPOLLIN) as u32
        }

        fn priority(&self) -> i8 {
            0i8
        }

        fn dispatch(&self, _: &Events) -> i32 {
            self.priority();
            0
        }

        fn token(&self) -> u64 {
            let data: u64 = unsafe { std::mem::transmute(self) };
            data
        }
    }

    #[test]
    fn test_io() {
        thread::spawn(move || {
            let listener = TcpListener::bind("0.0.0.0:9097").unwrap();
            loop {
                let (_stream, addr) = listener.accept().unwrap();
                println!("Accepted a new connection: {addr}");
            }
        });

        thread::sleep(Duration::from_millis(100));
        let e = Events::new().unwrap();
        let s: Rc<dyn Source> = Rc::new(Io::new("0.0.0.0:9097"));
        let s2: Rc<dyn Source> = Rc::new(Io::new("127.0.0.1:9097"));
        e.add_source(s.clone()).unwrap();
        e.add_source(s2.clone()).unwrap();

        e.set_enabled(s.clone(), EventState::On).unwrap();
        e.set_enabled(s2.clone(), EventState::On).unwrap();

        e.run(100).unwrap();
        e.run(100).unwrap();
        e.run(100).unwrap();

        e.del_source(s.clone()).unwrap();
        e.del_source(s2.clone()).unwrap();
    }

    #[test]
    fn test_io_onshot() {
        thread::spawn(move || {
            let listener = TcpListener::bind("0.0.0.0:9098").unwrap();
            loop {
                let (_stream, addr) = listener.accept().unwrap();
                println!("Accepted a new connection: {addr}");
            }
        });

        thread::sleep(Duration::from_millis(100));
        let e = Events::new().unwrap();
        let s: Rc<dyn Source> = Rc::new(Io::new("0.0.0.0:9098"));
        e.add_source(s.clone()).unwrap();

        e.set_enabled(s.clone(), EventState::OneShot).unwrap();

        e.run(100).unwrap();
        e.run(100).unwrap();
        e.run(100).unwrap();

        e.del_source(s.clone()).unwrap();
    }

    // timer test
    struct Timer();

    impl Timer {
        fn new() -> Timer {
            Self {}
        }
    }

    impl Source for Timer {
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
            100000
        }

        fn dispatch(&self, e: &Events) -> i32 {
            self.fd();
            self.priority();
            e.set_exit();
            0
        }

        fn token(&self) -> u64 {
            let data: u64 = unsafe { std::mem::transmute(self) };
            data
        }
    }

    #[test]
    fn test_timer() {
        let e = Events::new().unwrap();
        let s: Rc<dyn Source> = Rc::new(Timer::new());
        e.add_source(s.clone()).unwrap();

        e.set_enabled(s.clone(), EventState::On).unwrap();

        e.rloop().unwrap();

        e.del_source(s.clone()).unwrap();
    }

    // test inotify
    #[derive(Debug)]
    struct Inotify();

    impl Inotify {
        fn new() -> Inotify {
            Self {}
        }
    }

    impl Source for Inotify {
        fn fd(&self) -> RawFd {
            0
        }

        fn event_type(&self) -> EventType {
            EventType::Inotify
        }

        fn epoll_event(&self) -> u32 {
            (libc::EPOLLIN) as u32
        }

        fn priority(&self) -> i8 {
            0i8
        }

        fn dispatch(&self, e: &Events) -> i32 {
            e.set_exit();
            println!("test_dir:");
            0
        }

        fn token(&self) -> u64 {
            let data: u64 = unsafe { std::mem::transmute(self) };
            data
        }
    }

    #[test]
    fn test_inotify() {
        thread::spawn(move || loop {
            let mut test_dir = get_target_test_dir().unwrap();
            test_dir.push("libevent-test-xxxxxxfoo.txt");
            let _ = File::create(test_dir.as_os_str()).unwrap();
            let _ = std::fs::remove_file(test_dir);
        });

        let e = Events::new().unwrap();
        let s: Rc<dyn Source> = Rc::new(Inotify::new());
        e.add_source(s.clone()).unwrap();
        e.set_enabled(s.clone(), EventState::On).unwrap();

        let test_dir = get_target_test_dir().unwrap();
        let wd = e.add_watch(&test_dir, AddWatchFlags::IN_ALL_EVENTS);

        e.rloop().unwrap();

        e.rm_watch(wd);
        e.del_source(s.clone()).unwrap();
    }
}
