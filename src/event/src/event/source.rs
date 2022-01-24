use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    os::unix::io::RawFd, cell::RefCell, rc::Rc,
};

use crate::poll::Poll;

pub trait Source {
    fn fd(&self) -> RawFd;

    fn epoll_event(&self) -> libc::epoll_event {
        libc::epoll_event {
            events: (libc::EPOLLIN | libc::EPOLLONESHOT) as u32,
            u64: self.token(),
        }
    }

    fn description(&self) -> &'static str;

    fn token(&self) -> u64 {
        let mut s = DefaultHasher::new();
        self.description().hash(&mut s);
        s.finish()
    }

    fn priority(&self) -> i8 {
        0i8
    }

    fn dispatch(&self) {
        println!("Dispatching!");
    }

    fn register(&self, poller: Rc<RefCell<Poll>>) {
        let _ = poller.borrow_mut().register(self.fd(), &mut self.epoll_event());
    }

    fn reregister(&self, poller: Rc<RefCell<Poll>>) {
        let _ = poller.borrow_mut().reregister(self.fd(), &mut self.epoll_event());
    }

    fn deregister(&self, poller: Rc<RefCell<Poll>>) {
        let _ = poller.borrow_mut().unregister(self.fd());
    }
}

// for HashSet
impl Hash for dyn Source {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
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
