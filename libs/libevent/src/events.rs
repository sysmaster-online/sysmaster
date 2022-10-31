//! An event scheduling framework based on epoll
use crate::timer::Timer;
use crate::{EventState, EventType, Poll, Signals, Source};

use libutils::Error;
use libutils::Result;
use nix::sys::inotify::{AddWatchFlags, InitFlags, Inotify, InotifyEvent, WatchDescriptor};
use nix::NixPath;

use std::cell::RefCell;
use std::collections::{BinaryHeap, HashMap};
use std::convert::TryInto;
use std::mem::MaybeUninit;
use std::os::unix::prelude::{AsRawFd, RawFd};
use std::rc::Rc;

/// An event scheduling framework based on epoll
#[derive(Debug)]
pub struct Events {
    data: RefCell<EventsData>,
}

impl Drop for Events {
    fn drop(&mut self) {
        println!("Events drop, clear.");
        log::debug!("Events drop, clear.");
        // repeating protection
        self.clear();
    }
}

impl Events {
    /// create event
    pub fn new() -> Result<Events> {
        Ok(Events {
            data: RefCell::new(EventsData::new()),
        })
    }

    /// for all: add source which implement Source trait
    pub fn add_source(&self, source: Rc<dyn Source>) -> Result<i32> {
        self.data.borrow_mut().add_source(source)
    }

    /// for all: delete source
    pub fn del_source(&self, source: Rc<dyn Source>) -> Result<i32> {
        self.data.borrow_mut().del_source(source)
    }

    /// for all: set the source enabled state
    pub fn set_enabled(&self, source: Rc<dyn Source>, state: EventState) -> Result<i32> {
        self.data.borrow_mut().set_enabled(source, state)
    }

    /// for all: exit event loop
    pub fn set_exit(&self) {
        self.data.borrow_mut().set_exit()
    }

    /// for all: current time
    pub fn now() {
        todo!();
    }

    /// for all: Scheduling once, processing an event
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

    /// for all: Process the event in a loop until exiting actively
    pub fn rloop(&self) -> Result<i32> {
        loop {
            if let true = self.data.borrow().exit() {
                return Ok(0);
            }
            self.run(-1i32)?;
        }
    }

    /// private: Fetch the highest priority event processing on the pending queue
    fn dispatch(&self) -> Result<i32> {
        if let true = self.data.borrow().exit() {
            return Ok(0);
        }
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

    /// for signal: read the signal content when signal source emit
    pub fn read_signals(&self) -> std::io::Result<Option<libc::siginfo_t>> {
        self.data.borrow_mut().read_signals()
    }

    /// for inotify: add watch point to inotify event
    pub fn add_watch<P: ?Sized + NixPath>(&self, path: &P, mask: AddWatchFlags) -> WatchDescriptor {
        self.data.borrow_mut().add_watch(path, mask)
    }

    /// for inotify: rm watch point to inotify event
    pub fn rm_watch(&self, wd: WatchDescriptor) {
        self.data.borrow_mut().rm_watch(wd);
    }

    /// for inotify: read the inotify event when dispatch
    pub fn read_events(&self) -> Vec<InotifyEvent> {
        self.data.borrow_mut().read_events()
    }

    /// for test: clear all events to release resource
    // repeating protection
    pub fn clear(&self) {
        self.data.borrow_mut().clear();
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
    timerfd: HashMap<EventType, RawFd>,
    signal: Signals,
    timer: Timer,
    inotify: Inotify,
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
            timerfd: HashMap::new(),
            signal: Signals::new(),
            timer: Timer::new(),
            inotify: Inotify::init(InitFlags::IN_CLOEXEC | InitFlags::IN_NONBLOCK).unwrap(),
        }
    }

    pub(self) fn add_source(&mut self, source: Rc<dyn Source>) -> Result<i32> {
        let et = source.event_type();
        let token = source.token();
        match et {
            EventType::Io
            | EventType::Pidfd
            | EventType::Signal
            | EventType::Child
            | EventType::Inotify => {
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
            EventType::TimerRealtime
            | EventType::TimerBoottime
            | EventType::TimerMonotonic
            | EventType::TimerRealtimeAlarm
            | EventType::TimerBoottimeAlarm => (),
            // todo: implement
            EventType::Watchdog => todo!(),
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
            EventType::Io
            | EventType::Pidfd
            | EventType::Signal
            | EventType::Child
            | EventType::Inotify => {
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
            EventType::TimerRealtime
            | EventType::TimerBoottime
            | EventType::TimerMonotonic
            | EventType::TimerRealtimeAlarm
            | EventType::TimerBoottimeAlarm => (),
            // todo: implement
            EventType::Watchdog => todo!(),
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
    pub(self) fn source_online(&mut self, source: &Rc<dyn Source>) -> Result<i32> {
        let et = source.event_type();
        let token = source.token();
        let mut event = libc::epoll_event {
            events: source.epoll_event(),
            u64: token,
        };

        match et {
            EventType::Io | EventType::Pidfd => {
                self.poller.register(source.fd(), &mut event)?;
            }
            EventType::Signal => {
                self.signal.reset_sigset(source.signals());
                self.poller.register(self.signal.fd(), &mut event)?;
            }
            EventType::Child => {
                self.add_child(&mut event, source.pid());
            }
            EventType::TimerRealtime
            | EventType::TimerBoottime
            | EventType::TimerMonotonic
            | EventType::TimerRealtimeAlarm
            | EventType::TimerBoottimeAlarm => match self.timerfd.get(&et) {
                None => {
                    let fd = unsafe {
                        libc::timerfd_create(
                            self.timer.clockid(&et),
                            libc::TFD_NONBLOCK | libc::TFD_CLOEXEC,
                        )
                    };
                    self.timerfd.insert(et, fd);
                    self.poller.register(fd, &mut event)?;
                    self.timer.push(source.clone());
                }
                Some(_) => self.timer.push(source.clone()),
            },
            EventType::Defer => {
                self.pending.push(source.clone());
            }
            EventType::Inotify => {
                self.poller.register(self.inotify.as_raw_fd(), &mut event)?;
            }
            EventType::Post => todo!(),
            EventType::Exit => todo!(),
            EventType::Watchdog => todo!(),
        }

        Ok(0)
    }

    /// move the event out of the listening queue
    pub(self) fn source_offline(&mut self, source: &Rc<dyn Source>) -> Result<i32> {
        // unneed unregister when source is already Offline
        if let Some(event_state) = self.state.get(&source.token()) {
            if *event_state == EventState::Off {
                return Ok(0);
            }
        } else {
            return Ok(0);
        }

        let et = source.event_type();
        match et {
            EventType::Io | EventType::Pidfd => {
                self.poller.unregister(source.fd())?;
            }
            EventType::Signal => {
                self.poller.unregister(self.signal.fd())?;
            }
            EventType::Child => {
                self.poller.unregister(self.pidfd)?;
            }
            EventType::TimerRealtime
            | EventType::TimerBoottime
            | EventType::TimerMonotonic
            | EventType::TimerRealtimeAlarm
            | EventType::TimerBoottimeAlarm => {
                if self.timer.is_empty(&et) {
                    let fd = self.timerfd.get(&et);
                    if let Some(fd) = fd {
                        self.poller.unregister(fd.as_raw_fd())?
                    }
                }
            }
            EventType::Inotify => {
                self.poller.unregister(self.inotify.as_raw_fd())?;
            }
            EventType::Defer => (),
            EventType::Post => todo!(),
            EventType::Exit => todo!(),
            EventType::Watchdog => todo!(),
        }

        Ok(0)
    }

    pub(self) fn add_child(&mut self, event: &mut libc::epoll_event, pid: libc::pid_t) {
        let pidfd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) };
        self.pidfd = pidfd.try_into().unwrap();
        let _ = self.poller.register(self.pidfd, event);
        self.children.insert(pid.into(), pidfd);
    }

    /// read the signal content when signal source emit
    pub(self) fn read_signals(&mut self) -> std::io::Result<Option<libc::siginfo_t>> {
        self.signal.read_signals()
    }

    /// add watch point to inotify event
    pub(self) fn add_watch<P: ?Sized + NixPath>(
        &self,
        path: &P,
        mask: AddWatchFlags,
    ) -> WatchDescriptor {
        self.inotify.add_watch(path, mask).unwrap()
    }

    pub(self) fn rm_watch(&self, wd: WatchDescriptor) {
        self.inotify.rm_watch(wd).unwrap();
    }

    pub(self) fn read_events(&self) -> Vec<InotifyEvent> {
        self.inotify.read_events().unwrap()
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
            let token = event.u64;
            if let Some(s) = self.sources.get(&token) {
                self.pending.push(s.clone());
            }
        }

        for et in [
            EventType::TimerRealtime,
            EventType::TimerBoottime,
            EventType::TimerMonotonic,
            EventType::TimerRealtimeAlarm,
            EventType::TimerBoottimeAlarm,
        ] {
            if let Some(next) = self.timer.next(&et) {
                if self.timer.timerid(&et) >= next {
                    while let Some(source) = self.timer.pop(&et) {
                        self.pending_push(source);
                    }
                }
            }
        }

        !self.pending_is_empty() || !events.is_empty()
    }

    pub(self) fn prepare(&mut self) -> bool {
        let mut ret = false;

        for et in [
            EventType::TimerRealtime,
            EventType::TimerBoottime,
            EventType::TimerMonotonic,
            EventType::TimerRealtimeAlarm,
            EventType::TimerBoottimeAlarm,
        ] {
            self.timer.now();
            if let Some(next) = self.timer.next(&et) {
                if self.timer.timerid(&et) >= next {
                    while let Some(source) = self.timer.pop(&et) {
                        self.pending_push(source);
                    }
                    ret = true;
                } else {
                    let new_value = self.timer.timer_stored(next);
                    let mut old_value = MaybeUninit::<libc::itimerspec>::zeroed();
                    unsafe {
                        libc::timerfd_settime(
                            self.timerfd.get(&et).unwrap().as_raw_fd(),
                            libc::TFD_TIMER_ABSTIME,
                            &new_value,
                            old_value.as_mut_ptr(),
                        );
                    }
                }
            }
        }

        if !self.pending_is_empty() {
            return self.wait(0);
        }

        ret
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

    pub(self) fn pending_is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    fn clear(&mut self) {
        self.sources.clear();
        self.defer_sources.clear();
        self.post_sources.clear();
        self.exit_sources.clear();
        self.pending.clear();
        self.state.clear();
        self.children.clear();
        self.timerfd.clear();
    }
}
