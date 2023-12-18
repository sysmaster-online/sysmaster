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

//! An event scheduling framework based on epoll
use crate::error::*;
use crate::timer::Timer;
use crate::{EventState, EventType, Poll, Source};
use nix::sys::inotify::{AddWatchFlags, InitFlags, Inotify, InotifyEvent, WatchDescriptor};
use nix::sys::signalfd::siginfo;
use nix::sys::signalfd::SfdFlags;
use nix::sys::signalfd::SigSet;
use nix::sys::signalfd::SignalFd;
use nix::unistd;
use nix::NixPath;
use snafu::ResultExt;
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
        // repeating protection
        self.clear();
    }
}

impl Events {
    /// create event
    pub fn new() -> Result<Events> {
        Ok(Events {
            data: RefCell::new(EventsData::new()?),
        })
    }

    /// for all: add source which implement Source trait
    pub fn add_source(&self, source: Rc<dyn Source>) -> Result<i32> {
        self.data.borrow_mut().add_source(source)
    }

    /// for all: check if the source exists
    pub fn has_source(&self, source: Rc<dyn Source>) -> bool {
        self.data.borrow().has_source(source)
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
        let state = match self.data.borrow_mut().source_state(top.token()) {
            None => return Ok(0),
            Some(v) => v.state,
        };

        /* If a non-post event source raised, mark all post event sources as pending. */
        if state != EventState::Off && top.event_type() != EventType::Post {
            self.data.borrow_mut().pending_posts();
        }

        match state {
            EventState::Off => {}
            EventState::On => {
                top.dispatch(self);
                if top.event_type() == EventType::Defer {
                    self.data.borrow_mut().pending_push(top.clone(), 0);
                }
            }
            EventState::OneShot => {
                self.data
                    .borrow_mut()
                    .set_enabled(top.clone(), EventState::Off)?;

                top.dispatch(self);
            }
        }
        Ok(0)
    }

    /// for signal: read the signal content when signal source emit
    pub fn read_signals(&self) -> Option<siginfo> {
        self.data.borrow_mut().read_signals()
    }

    /// The "events" represents the "event_event" returned by epoll_wait.
    pub fn epoll_event(&self, token: u64) -> u32 {
        self.data.borrow().epoll_event(token)
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
    /// repeating protection
    pub fn clear(&self) {
        self.data.borrow_mut().clear();
    }
}

#[derive(Debug, Clone)]
pub(crate) struct State {
    state: EventState,
    epoll_event: u32,
    in_pending: bool,
}

impl Default for State {
    fn default() -> State {
        State {
            state: EventState::Off,
            epoll_event: 0,
            in_pending: false,
        }
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
    state: HashMap<u64, State>,
    children: HashMap<i64, i64>,
    pidfd: RawFd,
    timerfd: HashMap<EventType, RawFd>,
    signalfd: SignalFd,
    timer: Timer,
    inotifyfd: Inotify,
}

// the declaration "pub(self)" is for identification only.
impl EventsData {
    pub(self) fn new() -> Result<EventsData> {
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
            timerfd: HashMap::new(),
            signalfd: SignalFd::with_flags(
                &SigSet::empty(),
                SfdFlags::SFD_NONBLOCK | SfdFlags::SFD_CLOEXEC,
            )
            .context(NixSnafu)?,
            timer: Timer::new(),
            inotifyfd: Inotify::init(InitFlags::IN_CLOEXEC | InitFlags::IN_NONBLOCK)
                .context(NixSnafu)?,
        })
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
        self.state.insert(token, State::default());

        Ok(0)
    }

    pub(self) fn has_source(&self, source: Rc<dyn Source>) -> bool {
        let token = source.token();
        self.sources.contains_key(&token)
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
                self.defer_sources.remove(&token).ok_or(Error::Other {
                    word: "item not found",
                })?;
            }
            EventType::Post => {
                self.post_sources.remove(&token).ok_or(Error::Other {
                    word: "item not found",
                })?;
            }
            EventType::Exit => {
                self.exit_sources.remove(&token).ok_or(Error::Other {
                    word: "item not found",
                })?;
            }
            EventType::TimerRealtime
            | EventType::TimerBoottime
            | EventType::TimerMonotonic
            | EventType::TimerRealtimeAlarm
            | EventType::TimerBoottimeAlarm => {
                if self.timer.is_empty(&t) {
                    let fd = self.timerfd.remove(&t);
                    if let Some(fd) = fd {
                        self.poller.unregister(fd.as_raw_fd())?;
                        let _ = nix::unistd::close(fd);
                    }
                }
            }
            // todo: implement
            EventType::Watchdog => todo!(),
        }

        // remove state
        self.state.remove(&token);

        Ok(0)
    }

    pub(self) fn set_enabled(&mut self, source: Rc<dyn Source>, state: EventState) -> Result<i32> {
        let token = source.token();
        if let Some(current) = self.state.get(&token) {
            if current.state == state {
                return Ok(0);
            }
        }
        match state {
            EventState::On | EventState::OneShot => {
                self.source_online(&source)?;
            }
            EventState::Off => {
                self.source_offline(&source)?;
            }
        }

        if let Some(current) = self.state.get_mut(&token) {
            current.state = state;
        }

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
                let mut mask = SigSet::empty();
                for sig in source.signals() {
                    mask.add(sig);
                }
                mask.thread_set_mask().context(NixSnafu)?;
                self.signalfd.set_mask(&mask).context(NixSnafu)?;
                self.poller
                    .register(self.signalfd.as_raw_fd(), &mut event)?;
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
                self.pending_push(source.clone(), 0);
            }
            EventType::Inotify => {
                self.poller
                    .register(self.inotifyfd.as_raw_fd(), &mut event)?;
            }
            EventType::Post => {}
            EventType::Exit => todo!(),
            EventType::Watchdog => todo!(),
        }

        Ok(0)
    }

    /// move the event out of the listening queue
    pub(self) fn source_offline(&mut self, source: &Rc<dyn Source>) -> Result<i32> {
        // unneed unregister when source is already Offline
        if let Some(current) = self.state.get(&source.token()) {
            if current.state == EventState::Off {
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
                self.poller.unregister(self.signalfd.as_raw_fd())?;
            }
            EventType::Child => {
                self.poller.unregister(self.pidfd)?;
            }
            EventType::TimerRealtime
            | EventType::TimerBoottime
            | EventType::TimerMonotonic
            | EventType::TimerRealtimeAlarm
            | EventType::TimerBoottimeAlarm => {
                self.timer.remove(&et, source.clone());
            }
            EventType::Inotify => {
                self.poller.unregister(self.inotifyfd.as_raw_fd())?;
            }
            EventType::Defer => (),
            EventType::Post => {}
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
    pub(self) fn read_signals(&mut self) -> Option<siginfo> {
        self.signalfd.read_signal().unwrap_or(None)
    }

    pub(crate) fn epoll_event(&self, token: u64) -> u32 {
        match self.state.get(&token) {
            Some(t) => t.epoll_event,
            None => 0u32,
        }
    }

    /// add watch point to inotify event
    pub(self) fn add_watch<P: ?Sized + NixPath>(
        &self,
        path: &P,
        mask: AddWatchFlags,
    ) -> WatchDescriptor {
        self.inotifyfd.add_watch(path, mask).unwrap()
    }

    pub(self) fn rm_watch(&self, wd: WatchDescriptor) {
        self.inotifyfd.rm_watch(wd).unwrap();
    }

    pub(self) fn read_events(&self) -> Vec<InotifyEvent> {
        self.inotifyfd.read_events().unwrap()
    }

    /// Wait for the event event through poller
    /// And add the corresponding events to the pending queue
    pub(self) fn wait(&mut self, timeout: i32) -> bool {
        let events = if let Ok(s) = self.poller.poll(timeout) {
            s
        } else {
            return false;
        };

        for event in events.iter() {
            let token = event.u64;
            if let Some(source) = self.sources.get(&token) {
                #[allow(renamed_and_removed_lints)]
                #[allow(mutable_borrow_reservation_conflict)]
                self.pending_push(source.clone(), event.events);
            }
        }

        for et in [
            EventType::TimerRealtime,
            EventType::TimerBoottime,
            EventType::TimerMonotonic,
            EventType::TimerRealtimeAlarm,
            EventType::TimerBoottimeAlarm,
        ] {
            let next = match self.timer.next(&et) {
                None => continue,
                Some(v) => v,
            };
            if self.timer.timerid(&et) < next {
                continue;
            }
            if !self.flush_timer(&et) {
                return false;
            }

            while let Some(source) = self.timer.pop(&et) {
                self.pending_push(source, 0);
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
            let next = match self.timer.next(&et) {
                None => continue,
                Some(v) => v,
            };

            if self.timer.timerid(&et) >= next {
                while let Some(source) = self.timer.pop(&et) {
                    self.pending_push(source, 0);
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

        if !self.pending_is_empty() {
            return self.wait(0);
        }

        ret
    }

    pub(self) fn pending_pop(&mut self) -> Option<Rc<dyn Source>> {
        if let Some(top) = self.pending.pop() {
            if let Some(state) = self.state.get_mut(&top.token()) {
                state.in_pending = false;
            }
            return Some(top);
        };

        None
    }

    pub(self) fn pending_push(&mut self, source: Rc<dyn Source>, event: u32) {
        if let Some(current) = self.state.get_mut(&source.token()) {
            if current.in_pending {
                current.epoll_event |= event;
            } else {
                self.pending.push(source);
                current.in_pending = true;
            }
        }
    }

    pub(self) fn pending_posts(&mut self) {
        for (token, post_source) in self.post_sources.iter() {
            if let Some(current) = self.state.get_mut(token) {
                if current.state == EventState::Off {
                    continue;
                }

                if !current.in_pending {
                    self.pending.push(post_source.clone());
                    current.in_pending = true;
                }
            }
        }
    }

    pub(self) fn source_state(&self, token: u64) -> Option<State> {
        self.state.get(&token).cloned()
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

    fn flush_timer(&self, et: &EventType) -> bool {
        let timer_fd = self.timerfd.get(et).unwrap().as_raw_fd();
        if let Err(err) = unistd::read(timer_fd, &mut [0u8; 8]) {
            if err == nix::errno::Errno::EAGAIN || err == nix::errno::Errno::EINTR {
                return true;
            }
            return false;
        }
        true
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
        if nix::unistd::close(self.inotifyfd.as_raw_fd()).is_err() {
            println!("Failed to close inotify fd.");
        }
    }
}
