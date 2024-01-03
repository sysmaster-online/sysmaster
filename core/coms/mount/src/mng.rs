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

//!  The core logic of the mount subclass
use basic::fs::{directory_is_empty, mkdir_p_label};
use basic::mount::filter_options;
use basic::time::USEC_PER_SEC;
use basic::{MOUNT_BIN, UMOUNT_BIN};
use event::{EventState, Events};
use event::{EventType, Source};
use nix::sys::wait::WaitStatus;
use nix::unistd::Pid;

use crate::config::{mount_is_bind, MountConfig};
use crate::rentry::{MountProcFlags, MountResult};
use crate::spawn::MountSpawn;

use super::comm::MountUnitComm;
use super::rentry::MountState;
use core::error::*;
use core::exec::{ExecCommand, ExecContext};
use core::rel::ReStation;
use core::unit::{UnitActiveState, UnitNotifyFlags};
use std::os::unix::prelude::RawFd;
use std::path::Path;
use std::rc::Weak;
use std::{cell::RefCell, rc::Rc};

impl MountState {
    fn mount_state_to_unit_state(&self) -> UnitActiveState {
        match *self {
            MountState::Dead => UnitActiveState::InActive,
            MountState::Mounting => UnitActiveState::Activating,
            MountState::MountingDone => UnitActiveState::Activating,
            MountState::Mounted => UnitActiveState::Active,
            MountState::Remounting
            | MountState::RemountingSigKill
            | MountState::RemountingSigterm => UnitActiveState::Reloading,
            MountState::Unmounting
            | MountState::UnmountingSigkill
            | MountState::UnmountingSigterm => UnitActiveState::DeActivating,
            MountState::Failed => UnitActiveState::Failed,
        }
    }
}

pub(super) struct MountMng {
    comm: Rc<MountUnitComm>,
    state: RefCell<MountState>,

    config: Rc<MountConfig>,
    control_command: RefCell<Option<ExecCommand>>,
    spawn: Rc<MountSpawn>,
    timer: Rc<MountTimer>,

    result: RefCell<MountResult>,
    reload_result: RefCell<MountResult>,
    find_in_mountinfo: RefCell<bool>,
    proc_flags: RefCell<MountProcFlags>,
    pid: RefCell<Option<Pid>>,
    timeout_usec: RefCell<u64>,
}

impl ReStation for MountMng {
    // no input, no compensate

    // data
    fn db_map(&self, _reload: bool) {
        if let Some(state) = self.comm.rentry_mng_get() {
            *self.state.borrow_mut() = state;
        }
    }

    fn db_insert(&self) {
        self.comm.rentry_mng_insert(self.state());
    }

    fn entry_coldplug(&self) {
        // nothing to do.
    }

    fn entry_clear(&self) {
        if let Err(e) = self.disable_timer() {
            log::error!("Failed to disable timer: {}", e);
        }
    }

    // reload: no external connections, no entry
}

impl MountMng {
    pub(super) fn new(
        commr: &Rc<MountUnitComm>,
        configr: &Rc<MountConfig>,
        exec_ctx: &Rc<ExecContext>,
    ) -> Self {
        MountMng {
            comm: Rc::clone(commr),
            state: RefCell::new(MountState::Dead),
            config: Rc::clone(configr),
            control_command: RefCell::new(None),
            spawn: Rc::new(MountSpawn::new(commr, exec_ctx)),
            timer: Rc::new(MountTimer::new(u64::MAX)),
            result: RefCell::new(MountResult::Success),
            reload_result: RefCell::new(MountResult::Success),
            find_in_mountinfo: RefCell::new(false),
            proc_flags: RefCell::new(MountProcFlags::EMPTY),
            pid: RefCell::new(None),
            timeout_usec: RefCell::new(90 * USEC_PER_SEC),
        }
    }

    pub(super) fn enter_mounting(&self) {
        let mount_config = self.config.config_data();
        let mount_config = mount_config.borrow();

        let mount_where = Path::new(&mount_config.Mount.Where);
        let mount_what = Path::new(&mount_config.Mount.What);
        let directory_mode = self.config.directory_mode();

        let _ = mkdir_p_label(mount_where, directory_mode);
        if !mount_where.exists() || !mount_where.is_dir() {
            log::error!(
                "Failed to create the mount directory: {}",
                mount_config.Mount.Where
            );
            return;
        }
        if !directory_is_empty(mount_where) {
            log::warn!(
                "The mount directory {} is not empty.",
                mount_config.Mount.Where
            );
        }

        let mount_parameters = self.config.mount_parameters();
        if mount_is_bind(&mount_parameters) {
            if let Err(e) = mkdir_p_label(mount_what, directory_mode) {
                log::error!(
                    "Failed to create mount source {}: {}",
                    mount_config.Mount.What,
                    e
                );
            }
        }
        let filtered_options =
            filter_options(&mount_parameters.options, vec!["nofail", "noauto", "auto"]);

        let mut mount_command = ExecCommand::empty();
        if let Err(e) = mount_command.set_path(MOUNT_BIN) {
            log::error!("Failed to set mount command: {}", e);
            return;
        }
        mount_command.append_many_argv(vec![&mount_parameters.what, &mount_config.Mount.Where]);
        if !mount_parameters.fstype.is_empty() {
            mount_command.append_many_argv(vec!["-t", &mount_parameters.fstype]);
        }
        if !filtered_options.is_empty() {
            mount_command.append_many_argv(vec!["-o", &filtered_options]);
        }
        *self.control_command.borrow_mut() = Some(mount_command.clone());
        self.unwatch_control_pid();
        let control_pid = match self.spawn.spawn_cmd(&mount_command) {
            Err(e) => {
                log::error!(
                    "Failed to mount {} to {}: {}",
                    &mount_config.Mount.What,
                    &mount_config.Mount.Where,
                    e
                );
                return;
            }
            Ok(v) => v,
        };
        self.set_control_pid(Some(control_pid));
        self.watch_control_pid();
        self.set_state(MountState::Mounting, true);
    }

    pub(super) fn enter_signal(&self, state: MountState, res: MountResult) {
        if self.result() == MountResult::Success {
            self.set_result(res);
        }
        let unit = match self.comm.owner() {
            None => {
                log::error!(
                    "Failed to determine the owner unit if {}, ignoring",
                    self.comm.get_owner_id()
                );
                return;
            }
            Some(v) => v,
        };
        let kill_option = state.to_kill_option();
        let ret = unit.kill_context(
            self.config.kill_context(),
            None,
            self.control_pid(),
            kill_option,
            false,
        );
        if let Err(e) = ret {
            log::warn!(
                "Failed to kill process of {}: {}",
                self.comm.get_owner_id(),
                e
            );
            self.enter_dead_or_mounted(MountResult::FailureResources);
            return;
        }

        if ret.unwrap() {
            // enable timer
            self.set_state(state, true);
        } else if self.state() == MountState::RemountingSigterm {
            self.enter_signal(MountState::RemountingSigKill, MountResult::Success);
        } else if self.state() == MountState::RemountingSigKill {
            self.enter_mounted(MountResult::Success, true);
        } else if self.state() == MountState::UnmountingSigterm {
            self.enter_signal(MountState::UnmountingSigkill, MountResult::Success);
        } else {
            self.enter_dead_or_mounted(MountResult::Success);
        }
    }

    pub(super) fn enter_dead_or_mounted(&self, res: MountResult) {
        if self.find_in_mountinfo() {
            self.enter_mounted(res, true);
        } else {
            self.enter_dead(res, true);
        }
    }

    pub(super) fn enter_dead(&self, res: MountResult, notify: bool) {
        if self.result() == MountResult::Success {
            self.set_result(res);
        }
        log::info!("Mountpoint {} enter dead", self.comm.get_owner_id());
        if self.result() == MountResult::Success {
            self.set_state(MountState::Dead, notify);
        } else {
            self.set_state(MountState::Failed, notify);
        }
    }

    pub(super) fn enter_mounted(&self, res: MountResult, notify: bool) {
        if self.result() == MountResult::Success {
            self.set_result(res);
        }
        self.set_state(MountState::Mounted, notify);
    }

    pub(super) fn enter_unmounting(&self) {
        // Todo: retry_umount
        let mut umount_command = ExecCommand::empty();
        if let Err(e) = umount_command.set_path(UMOUNT_BIN) {
            log::error!("Failed to set umount command: {}", e);
        }
        let mount_where = self.config.mount_where();
        umount_command.append_many_argv(vec![&mount_where, "-c"]);
        *self.control_command.borrow_mut() = Some(umount_command.clone());
        self.unwatch_control_pid();
        let control_pid = match self.spawn.spawn_cmd(&umount_command) {
            Err(e) => {
                log::error!("Failed to umount {}: {}", mount_where, e);
                return;
            }
            Ok(v) => v,
        };
        self.set_control_pid(Some(control_pid));
        self.watch_control_pid();
        self.set_state(MountState::Unmounting, true);
    }

    pub(super) fn enter_remounting(&self) {
        self.set_result(MountResult::Success);
        let mount_parameters = self.config.mount_parameters();
        let options = if mount_parameters.options.is_empty() {
            "remount".to_string()
        } else {
            "remount".to_string() + &mount_parameters.options
        };
        let mut remount_command = ExecCommand::empty();
        if let Err(e) = remount_command.set_path(MOUNT_BIN) {
            log::error!("Failed to set remount command: {}", e);
        }
        remount_command.append_many_argv(vec![
            &self.config.mount_what(),
            &self.config.mount_where(),
            "-o",
            &options,
        ]);
        *self.control_command.borrow_mut() = Some(remount_command.clone());
        self.unwatch_control_pid();
        let control_pid = match self.spawn.spawn_cmd(&remount_command) {
            Err(e) => {
                log::error!(
                    "Failed to remount {} to {}: {}",
                    self.config.mount_what(),
                    self.config.mount_where(),
                    e
                );
                self.set_reload_result(MountResult::FailureResources);
                self.enter_dead_or_mounted(MountResult::Success);
                return;
            }
            Ok(v) => v,
        };
        self.set_control_pid(Some(control_pid));
        self.watch_control_pid();
        self.set_state(MountState::Remounting, true);
    }

    pub(super) fn cycle_clear(&self) {
        self.set_result(MountResult::Success);
        self.set_reload_result(MountResult::Success);
        *self.control_command.borrow_mut() = None;
    }

    pub(super) fn timeout_usec(&self) -> u64 {
        *self.timeout_usec.borrow()
    }

    pub(super) fn attach_spawn(&self, mng: &Rc<MountMng>) {
        self.spawn.attach_mng(mng)
    }

    pub(super) fn timer(&self) -> Rc<MountTimer> {
        self.timer.clone()
    }

    pub(super) fn attach_timer(&self, mng: &Rc<MountMng>) {
        self.timer.attach_mng(mng)
    }

    pub(super) fn enable_timer(&self, usec: u64) -> Result<i32> {
        let events = self.comm.um().events();
        if usec == u64::MAX {
            events.del_source(self.timer())?;
            return Ok(0);
        }
        log::debug!("Enable a timer: {}us", usec);

        let timer = self.timer();
        events.del_source(timer.clone())?;

        timer.set_time(usec);
        events.add_source(timer.clone())?;
        events.set_enabled(timer, EventState::OneShot)?;

        Ok(0)
    }

    pub(super) fn disable_timer(&self) -> Result<i32> {
        let events = self.comm.um().events();
        events.del_source(self.timer())?;
        Ok(0)
    }

    pub(super) fn dispatch_timer(&self) -> i32 {
        log::info!("Dispatch timeout event for {}", self.comm.get_owner_id());
        match self.state() {
            MountState::Mounting | MountState::MountingDone => {
                self.enter_signal(MountState::UnmountingSigterm, MountResult::FailureTimeout);
            }
            MountState::Remounting => {
                self.set_reload_result(MountResult::FailureTimeout);
                self.enter_signal(MountState::RemountingSigterm, MountResult::Success);
            }
            MountState::RemountingSigterm => {
                self.set_reload_result(MountResult::FailureTimeout);
                // Todo: check if SendKill is configured to yes
                self.enter_signal(MountState::RemountingSigKill, MountResult::Success);
            }
            MountState::RemountingSigKill => {
                self.set_reload_result(MountResult::FailureTimeout);
                self.enter_dead_or_mounted(MountResult::Success);
            }
            MountState::Unmounting => {
                self.enter_signal(MountState::UnmountingSigterm, MountResult::FailureTimeout);
            }
            MountState::UnmountingSigterm => {
                // Todo: check SendKill
                self.enter_signal(MountState::UnmountingSigkill, MountResult::FailureTimeout);
            }
            MountState::UnmountingSigkill => {
                self.enter_dead_or_mounted(MountResult::FailureTimeout);
            }
            // Dead, Mounted, Failed should be impossible here.
            _ => return -1,
        }
        0
    }

    pub(super) fn start_check(&self) -> Result<bool> {
        let ret = self.comm.owner().map_or(false, |u| u.test_start_limit());
        if !ret {
            self.enter_dead(MountResult::Success, true);
            return Err(Error::UnitActionECanceled);
        }

        Ok(false)
    }

    pub(super) fn start_action(&self) -> Result<()> {
        if [
            MountState::Unmounting,
            MountState::UnmountingSigterm,
            MountState::UnmountingSigkill,
        ]
        .contains(&self.state())
        {
            return Err(Error::UnitActionEAgain);
        }

        if [MountState::Mounting, MountState::MountingDone].contains(&self.state()) {
            return Ok(());
        }
        self.cycle_clear();
        self.enter_mounting();
        Ok(())
    }

    pub(super) fn stop_action(&self) -> Result<i32> {
        let state = self.state();
        if [
            MountState::Unmounting,
            MountState::UnmountingSigkill,
            MountState::UnmountingSigterm,
        ]
        .contains(&state)
        {
            return Ok(0);
        }
        if [
            MountState::Mounting,
            MountState::MountingDone,
            MountState::Remounting,
        ]
        .contains(&state)
        {
            self.enter_signal(MountState::UnmountingSigterm, MountResult::Success);
            return Ok(0);
        }
        if state == MountState::RemountingSigterm {
            self.set_state(MountState::UnmountingSigterm, true);
            return Ok(0);
        }
        if state == MountState::RemountingSigKill {
            self.set_state(MountState::UnmountingSigkill, true);
            return Ok(0);
        }
        if state == MountState::Mounted {
            self.enter_unmounting();
            return Ok(1);
        }
        Ok(0)
    }

    pub fn update_mount_state_by_mountinfo(&self) {
        // Todo: device state update
        if !self.is_mounted() {
            self.set_find_in_mountinfo(false);
            self.config
                .updated_mount_parameters_from_mountinfo("", "", "");
            match self.state() {
                MountState::Mounted => self.enter_dead(MountResult::Success, true),
                MountState::MountingDone => self.set_state(MountState::Mounting, true),
                _ => {}
            }
        } else if self
            .proc_flags
            .borrow()
            .intersects(MountProcFlags::JUST_MOUNTED | MountProcFlags::JUST_CHANGED)
        {
            match self.state() {
                MountState::Dead | MountState::Failed => {
                    self.cycle_clear();
                    self.enter_mounted(MountResult::Success, true)
                }
                MountState::Mounting => self.set_state(MountState::MountingDone, true),
                // Trigger a notify
                _ => self.set_state(self.state(), true),
            }
        }
        self.set_proc_flags(MountProcFlags::EMPTY);
    }

    pub fn reset_failed(&self) {
        if self.state() == MountState::Failed {
            self.set_state(MountState::Dead, true);
        }
        self.set_result(MountResult::Success);
        self.set_reload_result(MountResult::Success);
    }

    pub fn get_state(&self) -> String {
        let state = *self.state.borrow();
        state.to_string()
    }

    fn set_state(&self, new_state: MountState, notify: bool) {
        let old_state = self.state();
        self.change_state(new_state);

        if notify {
            self.state_notify(new_state, old_state);
        }
    }

    fn state_notify(&self, new_state: MountState, old_state: MountState) {
        if new_state != old_state {
            log::debug!(
                "{} original state[{:?}] -> new state[{:?}]",
                self.comm.get_owner_id(),
                old_state,
                new_state,
            );
        }

        let old_unit_state = old_state.mount_state_to_unit_state();
        let new_unit_state = new_state.mount_state_to_unit_state();
        if let Some(u) = self.comm.owner() {
            u.notify(
                old_unit_state,
                new_unit_state,
                UnitNotifyFlags::RELOAD_FAILURE,
            )
        }

        self.db_update();
    }

    fn change_state(&self, new_state: MountState) {
        self.state.replace(new_state);
    }

    pub fn state(&self) -> MountState {
        *self.state.borrow()
    }

    pub(super) fn mount_state_to_unit_state(&self) -> UnitActiveState {
        self.state().mount_state_to_unit_state()
    }

    fn result(&self) -> MountResult {
        *self.result.borrow()
    }

    fn set_result(&self, r: MountResult) {
        *self.result.borrow_mut() = r
    }

    fn set_reload_result(&self, r: MountResult) {
        if *self.reload_result.borrow() != MountResult::Success {
            return;
        }
        *self.reload_result.borrow_mut() = r
    }

    pub fn find_in_mountinfo(&self) -> bool {
        *self.find_in_mountinfo.borrow()
    }

    pub fn set_find_in_mountinfo(&self, find: bool) {
        *self.find_in_mountinfo.borrow_mut() = find
    }

    pub fn set_proc_flags(&self, proc_flags: MountProcFlags) {
        *self.proc_flags.borrow_mut() = proc_flags
    }

    pub fn append_proc_flags(&self, new_proc_flags: MountProcFlags) {
        let cur = *self.proc_flags.borrow();
        *self.proc_flags.borrow_mut() = cur | new_proc_flags
    }

    pub fn is_mounted(&self) -> bool {
        self.proc_flags
            .borrow()
            .contains(MountProcFlags::IS_MOUNTED)
    }

    pub fn set_control_pid(&self, control_pid: Option<Pid>) {
        *self.pid.borrow_mut() = control_pid;
    }

    pub fn control_pid(&self) -> Option<Pid> {
        *self.pid.borrow()
    }

    pub fn watch_control_pid(&self) {
        if let Some(pid) = self.control_pid() {
            if let Some(u) = self.comm.owner() {
                self.comm.um().child_watch_pid(&u.id(), pid);
            }
        }
    }

    pub fn unwatch_control_pid(&self) {
        if let Some(pid) = self.control_pid() {
            if let Some(u) = self.comm.owner() {
                self.comm.um().child_unwatch_pid(&u.id(), pid)
            }
            self.set_control_pid(None);
        }
    }
}

impl MountMng {
    pub(super) fn sigchld_event(&self, wait_status: WaitStatus) {
        if self.control_pid() != wait_status.pid() {
            return;
        }
        self.do_sigchld_event(wait_status);
        self.db_update();
    }

    fn do_sigchld_event(&self, wait_status: WaitStatus) {
        self.set_control_pid(None);
        log::debug!("Got a mount process sigchld, status: {:?}", wait_status);
        let mut f = self.sigchld_result(wait_status);
        let state = self.state();

        if [
            MountState::Remounting,
            MountState::RemountingSigKill,
            MountState::RemountingSigterm,
        ]
        .contains(&state)
        {
            self.set_reload_result(f);
        } else if self.result() == MountResult::Success {
            self.set_result(f);
        }
        if self.control_command.borrow().is_some() {
            *self.control_command.borrow_mut() = None;
        }

        // mount process has exited, but the mount point is not mounted
        if state == MountState::Mounting {
            if f == MountResult::Success {
                log::warn!(
                    "Mount process of {} has exited, but there is no mount.",
                    self.comm.get_owner_id()
                );
                f = MountResult::FailureProtocol;
            }
            self.enter_dead(f, true);
            return;
        }

        // we have seen this mountpoint in /p/s/mountinfo
        if state == MountState::MountingDone {
            self.enter_mounted(MountResult::Success, true);
            return;
        }

        if [
            MountState::Remounting,
            MountState::RemountingSigKill,
            MountState::RemountingSigterm,
        ]
        .contains(&state)
        {
            self.enter_dead_or_mounted(MountResult::Success);
            return;
        }

        if state == MountState::Unmounting {
            // umount process has exited, but we can still see the mountpoint in /p/s/mountinfo
            if f == MountResult::Success && self.find_in_mountinfo() {
                self.enter_mounted(MountResult::Success, true);
            } else {
                self.enter_dead_or_mounted(f);
            }
            return;
        }

        if [MountState::UnmountingSigkill, MountState::UnmountingSigterm].contains(&state) {
            self.enter_dead_or_mounted(f);
        }
    }

    fn sigchld_result(&self, wait_status: WaitStatus) -> MountResult {
        match wait_status {
            WaitStatus::Exited(_, status) => {
                if status == 0 {
                    MountResult::Success
                } else {
                    MountResult::FailureExitCode
                }
            }
            WaitStatus::Signaled(_pid, _sig, core_dump) => {
                if core_dump {
                    MountResult::FailureCoreDump
                } else {
                    MountResult::FailureSignal
                }
            }
            _ => unreachable!(),
        }
    }
}

pub(super) struct MountTimer {
    time: RefCell<u64>,
    mng: RefCell<Weak<MountMng>>,
}

impl MountTimer {
    pub fn new(usec: u64) -> Self {
        MountTimer {
            time: RefCell::new(usec),
            mng: RefCell::new(Weak::new()),
        }
    }

    pub(super) fn attach_mng(&self, mng: &Rc<MountMng>) {
        *self.mng.borrow_mut() = Rc::downgrade(mng)
    }

    pub(super) fn set_time(&self, usec: u64) {
        *self.time.borrow_mut() = usec
    }

    pub(self) fn mng(&self) -> Rc<MountMng> {
        self.mng.borrow().clone().upgrade().unwrap()
    }

    fn do_dispatch(&self) -> i32 {
        self.mng().dispatch_timer()
    }
}

impl Source for MountTimer {
    fn fd(&self) -> RawFd {
        0
    }

    fn event_type(&self) -> EventType {
        EventType::TimerMonotonic
    }

    fn epoll_event(&self) -> u32 {
        (libc::EPOLLIN) as u32
    }

    fn time_relative(&self) -> u64 {
        *self.time.borrow()
    }

    fn dispatch(&self, _: &Events) -> i32 {
        self.do_dispatch()
    }

    fn priority(&self) -> i8 {
        0
    }

    fn token(&self) -> u64 {
        let data: u64 = unsafe { std::mem::transmute(self) };
        data
    }

    fn description(&self) -> String {
        String::from("MountTimer")
    }
}

// #[cfg(test)]
// mod tests {
//     use super::MountMng;
//     use super::MountState;
//     use super::MountUnitComm;
//     use std::rc::Rc;

//     #[test]
//     fn test_mount_set_state() {
//         let _comm = Rc::new(MountUnitComm::new());
//         let tm = MountMng::new(&_comm);
//         tm.set_state(MountState::Mounted, false);
//         assert_eq!(tm.state(), MountState::Mounted)
//     }

//     #[test]
//     fn test_mount_enter_dead() {
//         let _comm = Rc::new(MountUnitComm::new());
//         let tm = MountMng::new(&_comm);
//         tm.enter_dead(false);
//         assert_eq!(tm.state(), MountState::Dead)
//     }

//     #[test]
//     fn test_mount_enter_mounted() {
//         let _comm = Rc::new(MountUnitComm::new());
//         let tm = MountMng::new(&_comm);
//         tm.enter_mounted(false);
//         assert_eq!(tm.state(), MountState::Mounted)
//     }
// }
