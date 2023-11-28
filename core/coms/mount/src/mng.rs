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
use basic::{MOUNT_BIN, UMOUNT_BIN};
use basic::fs_util::{directory_is_empty, mkdir_p_label};
use basic::mount_util::filter_options;
use libc::umount;
use nix::sys::wait::WaitStatus;

use crate::config::{MountConfig, mount_is_bind};
use crate::rentry::{MountRe, MountResult};
use crate::spawn::MountSpawn;

use super::comm::MountUnitComm;
use super::rentry::MountState;
use core::error::*;
use core::exec::{ExecCommand, ExecContext};
use core::rel::ReStation;
use core::unit::{UnitActiveState, UnitNotifyFlags};
use std::path::Path;
use std::{cell::RefCell, rc::Rc};

impl MountState {
    fn mount_state_to_unit_state(&self) -> UnitActiveState {
        match *self {
            MountState::Dead => UnitActiveState::InActive,
            MountState::Mounted => UnitActiveState::Active,
            _ => UnitActiveState::InActive,
        }
    }
}

pub(super) struct MountMng {
    comm: Rc<MountUnitComm>,
    state: RefCell<MountState>,

    config: Rc<MountConfig>,
    control_command: RefCell<Option<ExecCommand>>,
    spawn: Rc<MountSpawn>,
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

    // reload: no external connections, no entry
}

impl MountMng {
    pub(super) fn new(commr: &Rc<MountUnitComm>, configr: &Rc<MountConfig>, exec_ctx: &Rc<ExecContext>) -> Self {
        MountMng {
            comm: Rc::clone(commr),
            state: RefCell::new(MountState::Dead),
            config: Rc::clone(configr),
            control_command: RefCell::new(None),
            spawn: Rc::new(MountSpawn::new(commr, exec_ctx)),
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
            log::error!("Failed to create the mount directory: {}", mount_config.Mount.Where);
            return;
        }
        if !directory_is_empty(mount_where) {
            log::warn!("The mount directory {} is not empty.", mount_config.Mount.Where);
        }

        let mount_parameters = self.config.mount_parameters();
        if mount_is_bind(&mount_parameters) {
            if let Err(e) = mkdir_p_label(mount_what, directory_mode) {
                log::error!("Failed to create mount source {}: {}", mount_config.Mount.What, e);
            }
        }
        let filtered_options = filter_options(&mount_parameters.options, vec!["nofail", "noauto", "auto"]);

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

        if let Err(e) = self.spawn.spawn_cmd(&mount_command) {
            log::error!("Failed to mount {} to {}: {}", &mount_config.Mount.What, &mount_config.Mount.Where, e);
            return;
        }

        self.set_state(MountState::Mounting, true);
    }

    pub(super) fn enter_signal(&self, state: MountState, res: MountResult) {}

    pub(super) fn enter_dead_or_mounted(&self) {}

    pub(super) fn enter_dead(&self, notify: bool) {
        self.set_state(MountState::Dead, notify);
    }

    pub(super) fn enter_mounted(&self, notify: bool) {
        self.set_state(MountState::Mounted, notify);
    }

    pub(super) fn enter_unmounting(&self) {
        // retry_umount
        let mut umount_command = ExecCommand::empty();
        if let Err(e) = umount_command.set_path(UMOUNT_BIN) {
            log::error!("Failed to set umount command: {}", e);
        }
        let mount_where = self.config.mount_where();
        umount_command.append_many_argv(vec![&mount_where, "-c"]);
        if let Err(e) = self.spawn.spawn_cmd(&umount_command) {
            log::error!("Failed to umount {}: {}", mount_where, e);
            return;
        }
        self.set_state(MountState::Unmounting, true);
    }

    pub(super) fn enter_remounting(&self) {}

    pub(super) fn dispatch_timer(&self) {}

    pub(super) fn start_check(&self) -> Result<bool> {
        let ret = self.comm.owner().map_or(false, |u| u.test_start_limit());
        if !ret {
            self.enter_dead(true);
            return Err(Error::UnitActionECanceled);
        }

        Ok(false)
    }

    pub(super) fn start_action(&self) -> Result<()> {
        if [
            MountState::Unmounting,
            MountState::UnmountingSigterm,
            MountState::UnmountingSigkill,
            MountState::Cleaning,
        ]
        .contains(&self.state())
        {
            return Err(Error::UnitActionEAgain);
        }

        if [MountState::Mounting, MountState::MountingDone].contains(&self.state()) {
            return Ok(());
        }

        self.enter_mounting();
        Ok(())
    }

    pub(super) fn stop_action(&self) -> Result<i32> {
        let state = self.state();
        if [MountState::Unmounting, MountState::UnmountingSigkill, MountState::UnmountingSigterm].contains(&state) {
            return Ok(0);
        }
        if [MountState::Mounting, MountState::MountingDone, MountState::Remounting].contains(&state) {
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
        if state == MountState::Cleaning {
            self.enter_signal(MountState::UnmountingSigkill, MountResult::Success);
            return Ok(0);
        }
        Ok(0)
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

    fn state(&self) -> MountState {
        *self.state.borrow()
    }

    pub(super) fn mount_state_to_unit_state(&self) -> UnitActiveState {
        self.state().mount_state_to_unit_state()
    }
}

impl MountMng {
    pub(super) fn sigchld_event(&self, wait_status: WaitStatus) {
        self.do_sigchld_event(wait_status);
        self.db_update();
    }

    fn do_sigchld_event(&self, wait_status: WaitStatus) {
        log::info!("Got a mount process sigchld, status: {:?}", wait_status);
        return;
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
