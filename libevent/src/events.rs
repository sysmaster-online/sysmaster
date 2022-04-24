use crate::{EventState, EventType, Poll, Signals, Source};

use utils::Error;
use utils::Result;

use std::cell::RefCell;
use std::collections::{BinaryHeap, HashMap};
use std::convert::TryInto;
use std::fmt::Debug;
use std::os::unix::io::RawFd;
use std::rc::Rc;

#[derive(Debug)]
pub struct Events {
    poller: Poll,
    exit: bool,
    sources: HashMap<u64, Rc<RefCell<dyn Source>>>,
    defer_sources: HashMap<u64, Rc<RefCell<dyn Source>>>,
    post_sources: HashMap<u64, Rc<RefCell<dyn Source>>>,
    exit_sources: HashMap<u64, Rc<RefCell<dyn Source>>>,
    pending: BinaryHeap<Rc<RefCell<dyn Source>>>,
    state: HashMap<u64, EventState>,
    children: HashMap<i64, i64>,
    pidfd: RawFd,
    signal: Signals,
}

impl Events {
    pub fn new() -> Result<Events> {
        Ok(Self {
            poller: Poll::new()?,
            exit: false,
            sources: HashMap::new(),
            defer_sources: HashMap::new(),
            post_sources: HashMap::new(),
            exit_sources: HashMap::new(),
            pending: BinaryHeap::new(),
            state: HashMap::new(),
            children: HashMap::new(),
            pidfd: 0,
            signal: Signals::new(),
        })
    }

    pub fn add_source(&mut self, source: Rc<RefCell<dyn Source>>) -> Result<i32> {
        if source.try_borrow().is_err() {
            return Ok(0);
        }

        let t = source.try_borrow().unwrap().event_type();
        let s = source.try_borrow().unwrap();
        let token = s.token();
        match t {
            EventType::Io | EventType::Pidfd | EventType::Signal | EventType::Child => {
                self.sources.insert(token, source.clone());
            }
            EventType::Defer => {
                self.defer_sources.insert(token, source.clone());
            }
            EventType::Post => {
                self.post_sources.insert(token, source.clone());
            }
            EventType::Exit => {
                self.exit_sources.insert(token, source.clone());
            }
            // todo: implement
            EventType::Timer | EventType::TimerRelative | EventType::Inotify => todo!(),
        }

        // default state
        self.state.insert(token, EventState::Off);

        Ok(0)
    }

    pub fn del_source(&mut self, source: Rc<RefCell<dyn Source>>) -> Result<i32> {
        if source.try_borrow().is_err() {
            return Ok(0);
        }

        self.source_offline(&source)?;

        let t = source.try_borrow().unwrap().event_type();
        let s = source.try_borrow().unwrap();
        let token = s.token();
        match t {
            EventType::Io | EventType::Pidfd | EventType::Signal | EventType::Child => {
                self.sources.remove(&token);
            }
            EventType::Defer => {
                self.defer_sources
                    .remove(&token)
                    .ok_or(Error::Other { msg: "not found" })?;
            }
            EventType::Post => {
                self.post_sources
                    .remove(&token)
                    .ok_or(Error::Other { msg: "not found" })?;
            }
            EventType::Exit => {
                self.exit_sources
                    .remove(&token)
                    .ok_or(Error::Other { msg: "not found" })?;
            }
            // todo: implement
            EventType::Timer | EventType::TimerRelative | EventType::Inotify => todo!(),
        }

        // remove state
        self.state.remove(&token);

        Ok(0)
    }

    pub fn set_enabled(
        &mut self,
        source: Rc<RefCell<dyn Source>>,
        state: EventState,
    ) -> Result<i32> {
        if source.try_borrow().is_err() {
            return Ok(0);
        }

        let token = source.try_borrow().unwrap().token();
        match state {
            EventState::On | EventState::OneShot => {
                self.source_online(&source)?;
            }
            EventState::Off => {
                self.source_offline(&source)?;
            }
        }

        // renew state
        self.state.insert(token, state);

        Ok(0)
    }

    fn source_online(&mut self, source: &Rc<RefCell<dyn Source>>) -> Result<i32> {
        let t = source.try_borrow().unwrap().event_type();
        let s = source.try_borrow().unwrap();
        let token = s.token();
        let mut event = libc::epoll_event {
            events: s.epoll_event(),
            u64: token,
        };

        match t {
            EventType::Io | EventType::Pidfd => {
                self.poller.register(s.fd(), &mut event)?;
            }
            EventType::Timer => todo!(),
            EventType::TimerRelative => todo!(),
            EventType::Signal => {
                self.signal.reset_sigset(s.signals());
                self.poller.register(self.signal.get_fd(), &mut event)?;
            }
            EventType::Child => {
                self.add_child(&mut event, s.pid());
            }
            EventType::Inotify => todo!(),
            EventType::Defer => {
                self.pending.push(source.clone());
            }
            EventType::Post => todo!(),
            EventType::Exit => todo!(),
        }

        Ok(0)
    }

    fn source_offline(&mut self, source: &Rc<RefCell<dyn Source>>) -> Result<i32> {
        let source = source.try_borrow().unwrap();
        // unneed unregister when source is allready Offline
        if *self.state.get(&source.token()).unwrap() == EventState::Off {
            return Ok(0);
        }

        let t = source.event_type();
        match t {
            EventType::Io | EventType::Pidfd => {
                self.poller.unregister(source.fd())?;
            }
            EventType::Timer => todo!(),
            EventType::TimerRelative => todo!(),
            EventType::Signal => {
                self.poller.unregister(self.signal.get_fd())?;
            }
            EventType::Child => {
                self.poller.unregister(self.pidfd)?;
            }
            EventType::Inotify => todo!(),
            EventType::Defer => (),
            EventType::Post => todo!(),
            EventType::Exit => todo!(),
        }

        Ok(0)
    }

    fn add_child(&mut self, event: &mut libc::epoll_event, pid: libc::pid_t) {
        let pidfd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) };
        self.pidfd = pidfd.try_into().unwrap();
        let _ = self.poller.register(self.pidfd, event);
        self.children.insert(pid.into(), pidfd);
    }

    pub fn read_signals(&mut self) -> std::io::Result<Option<libc::siginfo_t>> {
        self.signal.read_signals()
    }

    /// Wait for the event event through poller
    /// And add the corresponding events to the pengding queue
    fn wait(&mut self, timeout: i32) -> bool {
        let events = {
            #[allow(clippy::never_loop)]
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
            let s = self.sources.get(token).unwrap();
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
    fn dispatch(&mut self) -> Result<i32> {
        if self.pending.peek().is_none() {
            return Ok(0);
        }

        let first = self.pending_top();
        let top = first.try_borrow().unwrap();
        let state = self.state.get(&top.token()).unwrap();
        match state {
            EventState::Off => {
                println!("set_enabled Off: {:?}", top);
            }
            EventState::On => {
                top.dispatch(self)?;
                if top.event_type() == EventType::Defer {
                    self.pending.push(first.clone());
                }
            }
            EventState::OneShot => {
                top.dispatch(self)?;
                self.set_enabled(first.clone(), EventState::Off)?;
            }
        }

        self.pending.pop();

        Ok(0)
    }

    /// Scheduling once, processing an event
    pub fn run(&mut self, timeout: i32) -> Result<i32> {
        if self.exit {
            return Ok(0);
        }
        if self.prepare() {
            self.wait(timeout);
        }
        self.dispatch()?;
        Ok(0)
    }

    /// Process the event in a loop until exiting actively
    pub fn rloop(&mut self) -> Result<i32> {
        loop {
            if self.exit {
                return Ok(0);
            }
            self.run(-1i32)?;
        }
    }

    pub fn exit(&mut self) {
        self.exit = true;
    }

    pub fn now() {
        todo!();
    }
}
