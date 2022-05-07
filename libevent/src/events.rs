//! # 一种基于epoll的事件调度框架
//! An event scheduling framework based on epoll
use crate::{EventState, EventType, Poll, Signals, Source};

use utils::Error;
use utils::Result;

use std::cell::RefCell;
use std::collections::{BinaryHeap, HashMap};
use std::convert::TryInto;
use std::fmt::Debug;
use std::os::unix::io::RawFd;
use std::rc::Rc;

/// 一种基于epoll的事件调度框架
/// An event scheduling framework based on epoll
#[derive(Debug)]
pub struct Events {
    data: RefCell<EventsData>,
}

impl Events {
    /// create event
    pub fn new() -> Result<Events> {
        Ok(Events {
            data: RefCell::new(EventsData::new()),
        })
    }

    /// add source which implement Source trait
    pub fn add_source(&self, source: Rc<dyn Source>) -> Result<i32> {
        self.data.borrow_mut().add_source(source)
    }

    /// delete source
    pub fn del_source(&self, source: Rc<dyn Source>) -> Result<i32> {
        self.data.borrow_mut().del_source(source)
    }

    /// set the dispatch state of the event
    pub fn set_enabled(&self, source: Rc<dyn Source>, state: EventState) -> Result<i32> {
        self.data.borrow_mut().set_enabled(source, state)
    }

    /// read the signal content when signal source emit
    pub fn read_signals(&self) -> std::io::Result<Option<libc::siginfo_t>> {
        self.data.borrow_mut().read_signals()
    }

    /// exit event loop
    pub fn set_exit(&self) {
        self.data.borrow_mut().set_exit()
    }

    /// current time
    pub fn now() {
        todo!();
    }

    /// Scheduling once, processing an event
    pub fn run(&self, timeout: i32) -> Result<i32> {
        if self.data.borrow().exit() {
            return Ok(0);
        }
        if !self.data.borrow_mut().prepare() {
            self.data.borrow_mut().wait(timeout);
        }
        self.dispatch()?;
        Ok(0)
    }

    /// Process the event in a loop until exiting actively
    pub fn rloop(&self) -> Result<i32> {
        loop {
            self.run(-1i32)?;
        }
    }

    /// Fetch the highest priority event processing on the pending queue
    fn dispatch(&self) -> Result<i32> {
        let first = self.data.borrow_mut().pending_pop();
        if first.is_none() {
            return Ok(0);
        }

        let top = first.unwrap();
        let state = self.data.borrow().source_state(&top).unwrap();
        match state {
            EventState::Off => {
                println!("set_enabled Off: {:?}", top);
            }
            EventState::On => {
                top.dispatch(self)?;
                if top.event_type() == EventType::Defer {
                    self.data.borrow_mut().pending_push(top.clone());
                }
            }
            EventState::OneShot => {
                top.dispatch(self)?;
                self.data
                    .borrow_mut()
                    .set_enabled(top.clone(), EventState::Off)?;
            }
        }

        Ok(0)
    }
}

#[derive(Debug)]
pub(crate) struct EventsData {
    poller: Poll,
    exit: bool,
    sources: HashMap<u64, Rc<dyn Source>>,
    defer_sources: HashMap<u64, Rc<dyn Source>>,
    post_sources: HashMap<u64, Rc<dyn Source>>,
    exit_sources: HashMap<u64, Rc<dyn Source>>,
    pending: BinaryHeap<Rc<dyn Source>>,
    state: HashMap<u64, EventState>,
    children: HashMap<i64, i64>,
    pidfd: RawFd,
    signal: Signals,
}

// the declaration "pub(self)" is for identification only.
impl EventsData {
    pub(self) fn new() -> EventsData {
        Self {
            poller: Poll::new().unwrap(),
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
        }
    }

    pub(self) fn add_source(&mut self, source: Rc<dyn Source>) -> Result<i32> {
        let t = source.event_type();
        let token = source.token();
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

    pub(self) fn del_source(&mut self, source: Rc<dyn Source>) -> Result<i32> {
        self.source_offline(&source)?;

        let t = source.event_type();
        let s = source;
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

    pub(self) fn set_enabled(&mut self, source: Rc<dyn Source>, state: EventState) -> Result<i32> {
        let token = source.token();
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

    /// when set to on, register events to the listening queue
    fn source_online(&mut self, source: &Rc<dyn Source>) -> Result<i32> {
        let t = source.event_type();
        let s = source;
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

    /// move the event out of the listening queue
    fn source_offline(&mut self, source: &Rc<dyn Source>) -> Result<i32> {
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

    /// read the signal content when signal source emit
    pub(self) fn read_signals(&mut self) -> std::io::Result<Option<libc::siginfo_t>> {
        self.signal.read_signals()
    }

    /// Wait for the event event through poller
    /// And add the corresponding events to the pending queue
    pub(self) fn wait(&mut self, timeout: i32) -> bool {
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

        if !self.pending_is_empty() || !events.is_empty() {
            true
        } else {
            false
        }
    }

    pub(self) fn prepare(&mut self) -> bool {
        if !self.pending_is_empty() {
            return self.wait(0);
        }

        false
    }

    pub(self) fn pending_pop(&mut self) -> Option<Rc<dyn Source>> {
        self.pending.pop()
    }

    pub(self) fn pending_push(&mut self, source: Rc<dyn Source>) {
        self.pending.push(source)
    }

    pub(self) fn source_state(&self, source: &Rc<dyn Source>) -> Option<EventState> {
        self.state.get(&source.token()).cloned()
    }

    pub(self) fn set_exit(&mut self) {
        self.exit = true;
    }

    pub(self) fn exit(&self) -> bool {
        self.exit
    }

    fn pending_is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}
