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

//! subcommand for devctl settle
//!

use crate::subcmds::utils::devmaster_queue_is_empty;
use crate::Result;
use basic::time::{parse_sec, USEC_INFINITY, USEC_PER_SEC};
use event::{EventState, EventType, Events, Source};
use libdevmaster::framework::control_manager::CONTROL_MANAGER_LISTEN_ADDR;
use nix::sys::inotify::AddWatchFlags;
use nix::unistd::{access, getuid, AccessFlags};
use std::io::Write;
use std::os::unix::io::RawFd;
use std::os::unix::net::UnixStream;
use std::rc::Rc;

#[derive(Debug)]
pub struct SettleArgs {
    timeout: Option<String>,
    exit_if_exists: Option<String>,
}

impl SettleArgs {
    pub fn new(timeout: Option<String>, exit_if_exists: Option<String>) -> Self {
        SettleArgs {
            timeout,
            exit_if_exists,
        }
    }

    /// subcommand for Wait for pending uevents
    pub fn subcommand(&self) -> Result<()> {
        let timeout = match &self.timeout {
            Some(timeout) => match parse_sec(timeout) {
                Ok(d) => d,
                Err(err) => {
                    log::error!("Failed to parse timeout value:{:?} err:{:?}", timeout, err);
                    return Err(err);
                }
            },
            None => 120 * USEC_PER_SEC,
        };

        // TODO: emit_deprecation_warning(); dbus is not currently implemented

        if 0 == getuid().as_raw() {
            /* guarantee that the devmaster daemon isn't pre-processing */
            let mut stream = match UnixStream::connect(CONTROL_MANAGER_LISTEN_ADDR) {
                Ok(stream) => stream,
                Err(err) => {
                    log::error!(
                        "Failed to connect to devmaster daemon, ignoring err:{:?}",
                        err
                    );
                    return Ok(());
                }
            };
            if let Err(err) = stream.write_all(b"ping ") {
                log::error!("Failed to ping to devmaster daemon, err:{:?}", err);
                return Ok(());
            }
        } else {
            /* For non-privileged users, at least check if devmaster is running. */
            if let Err(err) = access(CONTROL_MANAGER_LISTEN_ADDR, AccessFlags::F_OK) {
                if err == nix::Error::ENOENT {
                    log::error!("devmaster is not running err:{:?}", err);
                } else {
                    log::error!(
                        "Failed to check if {} exists err:{:?}",
                        CONTROL_MANAGER_LISTEN_ADDR,
                        err
                    );
                }
                return Err(err);
            }
        }

        let events = Events::new().unwrap();
        if timeout != USEC_INFINITY {
            let time: Rc<dyn Source> = Rc::new(Timer::new(timeout));
            events.add_source(time.clone()).unwrap();
            events
                .set_enabled(time.clone(), EventState::OneShot)
                .unwrap();
        }

        let _wd = events.add_watch("/run/devmaster", AddWatchFlags::IN_DELETE);
        let s: Rc<dyn Source> = Rc::new(Inotify::new(self.exit_if_exists.clone()));

        events.add_source(s.clone()).unwrap();
        events.set_enabled(s.clone(), EventState::On).unwrap();

        /* Check before entering the event loop, as the devmaster queue may be already empty. */
        if check(&self.exit_if_exists) {
            return Ok(());
        }

        events.rloop().unwrap();
        Ok(())
    }
}

/// trigger monitor
#[derive(Debug)]
struct Inotify {
    exit_if_exists: Option<String>,
}

/// public methods
impl Inotify {
    /// create a monitor instance for monitoring trigger
    pub fn new(exit_if_exists: Option<String>) -> Self {
        Inotify { exit_if_exists }
    }
}

impl Source for Inotify {
    /// socket fd
    fn fd(&self) -> RawFd {
        0
    }

    /// event type
    fn event_type(&self) -> EventType {
        EventType::Inotify
    }

    /// epoll type
    fn epoll_event(&self) -> u32 {
        (libc::EPOLLIN) as u32
    }

    /// priority of event source
    fn priority(&self) -> i8 {
        0i8
    }

    fn time_relative(&self) -> u64 {
        0
    }

    /// receive device from socket and remove path or uuid from settle_path_or_ids
    fn dispatch(&self, event: &Events) -> i32 {
        let _events = event.read_events();

        if check(&self.exit_if_exists) {
            event.set_exit();
        }

        0
    }

    /// token of event source
    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }
}

/// trigger monitor
#[derive(Debug)]
struct Timer {
    time_usec: u64,
}

/// public methods
impl Timer {
    /// create a monitor instance for monitoring trigger
    pub fn new(time_usec: u64) -> Self {
        Timer { time_usec }
    }
}

impl Source for Timer {
    /// socket fd
    fn fd(&self) -> RawFd {
        0
    }

    /// event type
    fn event_type(&self) -> EventType {
        EventType::TimerBoottime
    }

    /// epoll type
    fn epoll_event(&self) -> u32 {
        (libc::EPOLLIN) as u32
    }

    /// priority of event source
    fn priority(&self) -> i8 {
        0i8
    }

    fn time_relative(&self) -> u64 {
        self.time_usec
    }

    /// receive device from socket and remove path or uuid from settle_path_or_ids
    fn dispatch(&self, event: &Events) -> i32 {
        event.set_exit();
        0
    }

    /// token of event source
    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }
}

/// check the existence of exit_if_exists
fn check(exit_if_exists: &Option<String>) -> bool {
    if let Some(path) = exit_if_exists {
        match access(path.as_str(), AccessFlags::F_OK) {
            Ok(()) => return true,
            Err(err) => {
                if err != nix::Error::ENOENT {
                    log::error!(
                        "Failed to check the existence of {:?}, ignoring: {:?}",
                        path,
                        err
                    );
                }
            }
        }
    }

    /* exit if queue is empty */
    match devmaster_queue_is_empty() {
        Ok(flag) => flag,
        Err(err) => {
            if err != nix::Error::ENOENT {
                log::error!(
                    "Failed to check if devmaster queue is empty, ignoring: {:?}",
                    err
                );
            }

            false
        }
    }
}
