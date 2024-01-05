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

//! path_mng is the core of the path unit，implement the state transition, event management and sub child management.
//!

use super::comm::PathUnitComm;
use super::config::PathConfig;
use super::rentry::{PathResult, PathState};
use basic::fd::close;
use basic::fs::{directory_is_not_empty, mkdir_p_label};
use basic::glob::glob_first;
use basic::IN_SET;
use constants::INVALID_FD;
use core::error::*;
use core::rel::ReStation;
use core::unit::{PathSpec, PathType};
use core::unit::{UnitActiveState, UnitNotifyFlags};
use event::EventState;
use event::{EventType, Events, Source};
use nix::unistd::{access, AccessFlags};
use std::cell::RefCell;
use std::fmt;
use std::os::unix::prelude::RawFd;
use std::path::Path;
use std::rc::{Rc, Weak};

impl PathState {
    fn to_unit_active_state(self) -> UnitActiveState {
        match self {
            PathState::Dead => UnitActiveState::InActive,
            PathState::Waiting => UnitActiveState::Active,
            PathState::Running => UnitActiveState::Active,
            PathState::Failed => UnitActiveState::Failed,
        }
    }
}

pub(crate) struct PathMng {
    // associated objects
    comm: Rc<PathUnitComm>,
    config: Rc<PathConfig>,

    // owned objects
    all_inotify: RefCell<Vec<Rc<PathInotify>>>,
    state: RefCell<PathState>,
    result: RefCell<PathResult>,
}

impl ReStation for PathMng {
    // input: do nothing

    // compensate: do nothing

    // data
    fn db_map(&self, _reload: bool) {
        if let Some((state, result)) = self.comm.rentry_mng_get() {
            *self.state.borrow_mut() = state;
            *self.result.borrow_mut() = result;
        }
    }

    fn db_insert(&self) {
        self.comm.rentry_mng_insert(
            self.state(),
            self.result(),
            self.specs()
                .iter()
                .map(|p| {
                    (
                        p.path_type(),
                        p.previous_exists(),
                        p.path().to_str().unwrap().to_string(),
                    )
                })
                .collect::<_>(),
        );
    }

    // reload: entry-only
    fn entry_coldplug(&self) {
        if self.state() == PathState::Dead {
            return;
        }

        if IN_SET!(self.state(), PathState::Waiting, PathState::Running) {
            self.enter_waiting(true, false)
        }
    }

    fn entry_clear(&self) {
        self.unwatch();
    }
}

// the declaration "pub(self)" is for identification only.
impl PathMng {
    pub(crate) fn new(commr: &Rc<PathUnitComm>, configr: &Rc<PathConfig>) -> PathMng {
        PathMng {
            comm: Rc::clone(commr),
            config: Rc::clone(configr),
            all_inotify: RefCell::new(Vec::new()),
            state: RefCell::new(PathState::Dead),
            result: RefCell::new(PathResult::Success),
        }
    }

    pub(crate) fn start_action(&self) {
        if !IN_SET!(self.state(), PathState::Dead, PathState::Failed) {
            return;
        }

        let u = match self.comm.owner() {
            None => return,
            Some(u) => u,
        };
        if !self.comm.um().test_trigger_loaded(&u.id()) {
            return;
        }

        // TODO: unit_acquire_invocation_id;

        self.path_mkdir();

        *self.result.borrow_mut() = PathResult::Success;

        self.enter_waiting(true, false);

        self.db_update();
    }

    pub(crate) fn stop_action(&self) {
        if !IN_SET!(self.state(), PathState::Waiting, PathState::Running) {
            return;
        }

        self.enter_dead(PathResult::Success);
        self.db_update();
    }

    pub(crate) fn push_inotify(&self, inotify: Rc<PathInotify>) {
        self.all_inotify.borrow_mut().push(inotify);
        self.db_update();
    }

    pub fn all_inotify(&self) -> Vec<Rc<PathInotify>> {
        self.all_inotify.borrow().iter().cloned().collect::<_>()
    }

    fn specs(&self) -> Vec<Rc<PathSpec>> {
        self.all_inotify
            .borrow()
            .iter()
            .map(|p| Rc::clone(&p.spec()))
            .collect::<_>()
    }

    pub(crate) fn reset_failed(&self) {
        if self.state() == PathState::Failed {
            self.set_state(PathState::Dead);
        }
        self.set_result(PathResult::Success);
    }

    pub(crate) fn state(&self) -> PathState {
        *self.state.borrow()
    }

    pub(crate) fn get_state(&self) -> String {
        let state = *self.state.borrow();
        state.to_string()
    }

    fn set_state(&self, state: PathState) {
        let old_state = self.state();
        self.state.replace(state);

        if !IN_SET!(state, PathState::Waiting, PathState::Running) {
            self.unwatch();
        }

        if state != old_state {
            log::debug!("Changed {} -> {}", old_state.to_string(), state.to_string());
        }

        if let Some(u) = self.comm.owner() {
            u.notify(
                old_state.to_unit_active_state(),
                state.to_unit_active_state(),
                UnitNotifyFlags::EMPTY,
            )
        }
    }

    fn result(&self) -> PathResult {
        *self.result.borrow()
    }

    fn set_result(&self, res: PathResult) {
        *self.result.borrow_mut() = res;
    }

    fn db_update(&self) {
        self.db_insert();
    }

    pub(super) fn current_active_state(&self) -> UnitActiveState {
        self.state().to_unit_active_state()
    }

    fn watch(&self) {
        let events = self.comm.um().events();
        for inotify in self.all_inotify() {
            if inotify.watch().is_err() {
                self.unwatch();
                return;
            }
            if events.add_source(inotify.clone()).is_err() {
                self.unwatch();
                return;
            }
            if events.set_enabled(inotify.clone(), EventState::On).is_err() {
                self.unwatch();
                return;
            }
        }
    }

    fn unwatch(&self) {
        let events = self.comm.um().events();
        for inotify in self.all_inotify().iter() {
            let source = Rc::clone(inotify);
            events.set_enabled(source, EventState::Off).unwrap();

            close(inotify.fd());
            inotify.spec().set_inotify_fd(INVALID_FD);
        }
    }

    fn path_mkdir(&self) {
        if !self.config.config_data().borrow().Path.MakeDirectory {
            return;
        }

        for inotify in self.all_inotify() {
            if IN_SET!(
                inotify.spec().path_type(),
                PathType::Exists,
                PathType::ExistsGlob
            ) {
                continue;
            }

            if let Err(e) = mkdir_p_label(
                Path::new(&inotify.spec().path()),
                self.config.config_data().borrow().Path.DirectoryMode,
            ) {
                log::error!("mkdir({:?}) failed: {}", inotify.spec().path(), e);
            }
        }
    }

    pub fn enter_waiting(&self, initial: bool, from_trigger_notify: bool) {
        let u = match self.comm.owner() {
            None => return,
            Some(u) => u,
        };
        let um = self.comm.um();
        let trigger = um.unit_get_trigger(&u.id());

        /* If the triggered unit is already running, so are we */
        if !trigger.is_empty() && !um.current_active_state(&trigger).is_inactive_or_failed() {
            self.set_state(PathState::Running);
            self.unwatch();
            return;
        }

        if let Some(trigger_path) = self.check_good(initial, from_trigger_notify) {
            log::debug!("{} Got triggered.", self.comm.get_owner_id());
            self.enter_running(&trigger_path);
            return;
        }

        self.watch();

        /* The file might have appeared/been removed during the preparation for watch, so we must recheck. */
        if let Some(trigger_path) = self.check_good(false, from_trigger_notify) {
            log::debug!("{} Got triggered.", self.comm.get_owner_id());
            self.enter_running(&trigger_path);
            return;
        }

        self.set_state(PathState::Waiting);
    }

    fn enter_running(&self, _trigger_path: &str) {
        let u = match self.comm.owner() {
            None => return,
            Some(u) => u,
        };

        let um = self.comm.um();
        if um.has_stop_job(&self.comm.owner().unwrap().id()) {
            return;
        }

        let trigger = um.unit_get_trigger(&u.id());
        if trigger.is_empty() {
            log::error!("{} Unit to trigger vanished.", u.id());
            self.enter_dead(PathResult::FailureResources);
            return;
        }

        if let Err(err) = um.unit_start_by_job(&trigger) {
            log::error!("Failed to queue unit startup job: {:?}", err);
            self.enter_dead(PathResult::FailureResources);
        }

        self.set_state(PathState::Running);
        self.unwatch();
    }

    fn enter_dead(&self, result: PathResult) {
        if self.result() == PathResult::Success {
            self.set_result(result);
        }

        if self.result() != PathResult::Success {
            self.set_state(PathState::Failed)
        } else {
            self.set_state(PathState::Dead)
        }
    }

    fn check_good(&self, initial: bool, from_trigger_notify: bool) -> Option<String> {
        for inotify in self.all_inotify() {
            if let Some(trigger_path) = spec_check_good(&inotify.spec, initial, from_trigger_notify)
            {
                return Some(trigger_path);
            }
        }

        None
    }
}

fn spec_check_good(
    spec: &Rc<PathSpec>,
    initial: bool,
    from_trigger_notify: bool,
) -> Option<String> {
    let mut trigger = String::new();
    let mut good = false;

    match spec.path_type() {
        PathType::Exists => {
            if let Ok(()) = access(spec.path().as_path(), AccessFlags::F_OK) {
                good = true;
            }
        }

        PathType::ExistsGlob => {
            if let Some(s) = spec.path().as_path().to_str() {
                match glob_first(s) {
                    Ok(first) => {
                        good = true;
                        trigger = first;
                    }
                    Err(_) => good = false,
                }
            }
        }

        PathType::DirectoryNotEmpty => {
            good = match directory_is_not_empty(Path::new(spec.path().as_path())) {
                Ok(flag) => flag,
                Err(err) => !IN_SET!(err.get_errno(), libc::ENOENT, libc::ENOTDIR),
            };
        }

        PathType::Changed | PathType::Modified => {
            let b = match access(spec.path().as_path(), AccessFlags::F_OK) {
                Ok(()) => true,
                Err(_) => false,
            };

            good = !initial && !from_trigger_notify && b != spec.previous_exists();
            spec.set_previous_exists(b);
        }
        _ => {}
    }

    if good {
        if trigger.is_empty() {
            match spec.path().to_str() {
                Some(path) => return Some(path.to_string()),
                None => return None,
            }
        }
        return Some(trigger);
    }

    None
}

pub struct PathInotify {
    // associated objects
    mng: Weak<PathMng>,

    // owned objects
    spec: Rc<PathSpec>,
}

impl PathInotify {
    pub(crate) fn new(mng: &Rc<PathMng>, spec: Rc<PathSpec>) -> PathInotify {
        PathInotify {
            mng: Rc::downgrade(mng),
            spec,
        }
    }

    pub(crate) fn spec(&self) -> Rc<PathSpec> {
        self.spec.clone()
    }

    pub(crate) fn watch(&self) -> Result<()> {
        self.spec.watch()
    }

    fn mng(&self) -> Rc<PathMng> {
        self.mng.clone().upgrade().unwrap()
    }
}

impl fmt::Display for PathInotify {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.spec)
    }
}

impl Source for PathInotify {
    fn fd(&self) -> RawFd {
        self.spec.inotify_fd()
    }

    fn event_type(&self) -> EventType {
        EventType::Io
    }

    fn epoll_event(&self) -> u32 {
        (libc::EPOLLIN) as u32
    }

    fn priority(&self) -> i8 {
        0i8
    }

    fn dispatch(&self, _event: &Events) -> i32 {
        if !IN_SET!(self.mng().state(), PathState::Waiting, PathState::Running) {
            return 0;
        }

        match self.spec.read_fd_event() {
            Ok(changed) => {
                if changed {
                    self.mng().enter_running(self.spec.path().to_str().unwrap())
                } else {
                    self.mng().enter_waiting(false, false)
                }
            }
            Err(_) => {
                self.mng().enter_dead(PathResult::FailureResources);
                return 0;
            }
        }

        0
    }

    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }
}
