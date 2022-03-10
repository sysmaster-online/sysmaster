use crate::{signal::Signals, EventType, Poll, Source};

use std::{
    cell::RefCell,
    collections::{BinaryHeap, HashMap},
    convert::TryInto,
    fmt::Debug,
    rc::Rc,
};

#[derive(Debug)]
pub struct Events {
    poller: Poll,
    exit: bool,
    sources: HashMap<u64, Rc<RefCell<dyn Source>>>,
    defer_sources: HashMap<u64, Rc<RefCell<dyn Source>>>,
    post_sources: HashMap<u64, Rc<RefCell<dyn Source>>>,
    exit_sources: HashMap<u64, Rc<RefCell<dyn Source>>>,
    pending: BinaryHeap<Rc<RefCell<dyn Source>>>,
    children: HashMap<i64, i64>,
    signal: Signals,
}

impl Events {
    pub fn new() -> std::io::Result<Events> {
        Ok(Self {
            poller: Poll::new()?,
            exit: false,
            sources: HashMap::new(),
            defer_sources: HashMap::new(),
            post_sources: HashMap::new(),
            exit_sources: HashMap::new(),
            pending: BinaryHeap::new(),
            signal: Signals::new(),
            children: HashMap::new(),
        })
    }

    pub fn add_source(&mut self, source: Rc<RefCell<dyn Source>>) {
        if source.try_borrow().is_err() {
            return;
        }

        let t = source.try_borrow().unwrap().event_type();
        let s = source.try_borrow().unwrap();
        let token = s.token();
        let mut event = libc::epoll_event {
            events: s.epoll_event(),
            u64: token,
        };
        match t {
            EventType::Io => {
                let _ = self.poller.register(s.fd(), &mut event);
            }
            EventType::Timer => todo!(),
            EventType::TimerRelative => todo!(),
            EventType::Signal => {
                self.signal.reset_sigset(s.signals());
                let _ = self.poller.register(self.signal.get_fd(), &mut event);
            }
            EventType::Child => {
                let pidfd = unsafe { libc::syscall(libc::SYS_pidfd_open, s.pid(), 0) };
                let _ = self.poller.register(pidfd.try_into().unwrap(), &mut event);
                self.children.insert(s.pid().into(), pidfd);
            }
            EventType::Pidfd => {
                let _ = self.poller.register(s.fd(), &mut event);
            }
            EventType::Inotify => todo!(),
            EventType::Defer => {
                self.defer_sources.insert(token, source.clone());
                self.pending.push(source.clone());
                return;
            }
            EventType::Post => {
                self.post_sources.insert(token, source.clone());
                return;
            }
            EventType::Exit => {
                self.exit_sources.insert(token, source.clone());
                return;
            }
        }
        self.sources.insert(token, source.clone());
    }

    pub fn del_source(&mut self, source: Rc<RefCell<dyn Source>>) {
        if source.try_borrow().is_err() {
            return;
        }
        let s = source.try_borrow().unwrap();
        match s.event_type() {
            EventType::Io => {
                let _ = self.poller.unregister(s.fd());
            }
            EventType::Timer => todo!(),
            EventType::TimerRelative => todo!(),
            EventType::Signal => {
                let _ = self.poller.unregister(self.signal.get_fd());
                self.signal.restore_sigset();
            }
            EventType::Child => {
                // let pidfd = self.children.remove(s.pid().try_into().unwrap()).unwrap();
                // let _ = self.poller.unregister(pidfd.try_into().unwrap());
            }
            EventType::Pidfd => {
                let _ = self.poller.unregister(s.fd());
            }
            EventType::Inotify => todo!(),
            EventType::Defer => todo!(),
            EventType::Post => todo!(),
            EventType::Exit => todo!(),
        }

        self.sources.remove(&s.token());
    }

    pub fn read_signals(&mut self) -> std::io::Result<Option<libc::siginfo_t>> {
        self.signal.read_signals()
    }

    /// Wait for the event event through poller
    /// And add the corresponding events to the pengding queue
    fn wait(&mut self, timeout: i32) -> bool {
        let events = {
            loop {
                let result = self.poller.poll(timeout);

                match result {
                    Ok(events) => break events,
                    Err(_err) => return false,
                };
            }
        };

        for event in events.iter() {
            #[allow(unaligned_references)]
            let token = &event.u64;
            let s = self.sources.get(&token).unwrap();
            self.pending.push(s.clone());
        }

        true
    }

    /// Wait for the event event through poller
    /// And add the corresponding events to the pengding queue
    fn prepare(&mut self) -> bool {
        true
    }

    fn pending_top(&self) -> Rc<RefCell<dyn Source>> {
        self.pending.peek().unwrap().clone()
    }

    /// Fetch the highest priority event processing on the pending queue
    fn dispatch(&mut self) {
        if self.pending.peek().is_none() {
            return;
        }
        // self.pending.peek().unwrap().try_borrow().unwrap().dispatch(&mut self);
        println!("Event in pending queue :{:?}", self.pending);
        {
            let first = self.pending_top();
            let top = first.try_borrow().unwrap();
            match top.event_type() {
                EventType::Io => {
                    top.dispatch(self);
                }
                EventType::Timer => todo!(),
                EventType::TimerRelative => todo!(),
                EventType::Signal => {
                    top.dispatch(self);
                }
                EventType::Child => {
                    top.dispatch(self);
                }
                EventType::Pidfd => {
                    top.dispatch(self);
                }
                EventType::Inotify => todo!(),
                EventType::Defer => {
                    top.dispatch(self);
                }
                EventType::Post => todo!(),
                EventType::Exit => {
                    top.dispatch(self);
                }
            }
        }

        let event_type = self
            .pending
            .peek()
            .unwrap()
            .try_borrow()
            .unwrap()
            .event_type();

        if event_type != EventType::Defer {
            self.pending.pop();
        }
    }

    /// Scheduling once, processing an event
    pub fn run(&mut self, timeout: i32) {
        if self.exit == true {
            return;
        }
        if self.prepare() {
            self.wait(timeout);
        }
        self.dispatch();
    }

    /// Process the event in a loop until exiting actively
    pub fn rloop(&mut self) {
        loop {
            if self.exit == true {
                return;
            }
            self.run(-1i32);
        }
    }

    pub fn exit(&mut self) {
        self.exit = true;
    }

    pub fn now() {
        todo!();
    }
}
