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
mod signal;
pub mod source;
mod timer;

pub use crate::events::Events;
pub(crate) use crate::poll::Poll;
pub(crate) use crate::signal::Signals;
pub use crate::source::Source;
pub use error::*;

/// Supports event types added to the frame
/// An event scheduling framework based on epoll
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
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
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum EventState {
    /// Start scheduling
    On,
    /// Close scheduling
    Off,
    /// Stop after dispatching once
    OneShot,
}
