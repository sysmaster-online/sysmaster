use utils::Error;
use utils::Result;

use crate::EventType;
use crate::Events;
use std::fmt::Debug;
use std::os::unix::io::RawFd;

pub trait Source {
    fn fd(&self) -> RawFd {
        todo!()
    }

    fn signals(&self) -> Vec<libc::c_int> {
        vec![]
    }

    fn pid(&self) -> libc::pid_t {
        0
    }

    fn event_type(&self) -> EventType {
        EventType::Io
    }

    fn epoll_event(&self) -> u32 {
        (libc::EPOLLIN | libc::EPOLLONESHOT) as u32
    }

    fn token(&self) -> u64;
    // Here is a defalut implementation.
    // fn token(&mut self) -> u64 {
    //     let data: u64 = unsafe { std::mem::transmute(self) };
    //     data
    // }

    fn priority(&self) -> i8 {
        0i8
    }

    fn dispatch(&self, event: &mut Events) -> Result<i32, Error>;
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
