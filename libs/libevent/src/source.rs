//! # Events must implement the Source trait
use libutils::Result;

use crate::EventType;
use crate::Events;
use std::fmt::Debug;
use std::os::unix::io::RawFd;

/// Events must implement the Source trait
pub trait Source {
    /// Can be converted into a handle for Io events, you need to specify the fd to listen to
    fn fd(&self) -> RawFd {
        todo!()
    }

    /// The signal type needs to specify the signal to listen to
    fn signals(&self) -> Vec<libc::c_int> {
        vec![]
    }

    /// The pidfd type needs to specify the listening pid
    fn pid(&self) -> libc::pid_t {
        0
    }

    /// timer on useconds
    fn time(&self) -> u64 {
        u64::MAX
    }

    /// timer on useconds, USEC_PER_SEC * SEC you want to
    /// When both are implemented, time_relative() shall prevail
    fn time_relative(&self) -> u64 {
        // USEC_PER_SEC * SEC you want to;
        u64::MAX
    }

    /// Specify the type of source
    fn event_type(&self) -> EventType {
        EventType::Io
    }

    /// Specifies the epoll event type to listen for
    fn epoll_event(&self) -> u32 {
        (libc::EPOLLIN | libc::EPOLLONESHOT) as u32
    }

    /// token用来区分事件来源, 除非你能统一指定, 否则建议使用推荐实现
    ///
    /// The token is used to distinguish the source of the event, unless you can specify it uniformly,
    /// it is recommended to use the recommended implementation
    /// ```bash
    /// fn token(&self) -> u64 {
    ///     let data: u64 = unsafe { std::mem::transmute(self) };
    ///     data
    /// }
    /// ```
    fn token(&self) -> u64;

    /// Set the priority, -127i8 ~ 128i8, the smaller the value, the higher the priority
    fn priority(&self) -> i8 {
        0i8
    }

    /// The code of callback
    fn dispatch(&self, event: &Events) -> Result<i32>;
}

// for HashSet
impl std::hash::Hash for dyn Source {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.token().hash(state);
    }
}

impl PartialEq for dyn Source {
    fn eq(&self, other: &dyn Source) -> bool {
        self.token() == other.token()
    }
}

impl Eq for dyn Source {}

// for BinaryHeap
impl Ord for dyn Source {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority().cmp(&other.priority()).reverse()
    }
}

impl PartialOrd for dyn Source {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.priority().cmp(&other.priority()).reverse())
    }
}

impl Debug for dyn Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Source { ... }")
    }
}
