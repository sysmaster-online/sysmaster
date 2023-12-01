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

//! mount unit is entry of mount type of unit，need impl
//! UnitObj,UnitMngUtil, UnitSubClass trait

use crate::config::MountConfig;

use super::comm::MountUnitComm;
use super::mng::MountMng;
use core::error::*;
use core::exec::ExecContext;
use core::rel::{ReStation, Reliability};
use core::unit::{SubUnit, UmIf, UnitActiveState, UnitBase, UnitMngUtil};
use nix::sys::wait::WaitStatus;
use std::path::PathBuf;
use std::rc::Rc;

struct MountUnit {
    comm: Rc<MountUnitComm>,
    mng: Rc<MountMng>,
    config: Rc<MountConfig>,
    exec_ctx: Rc<ExecContext>,
}

impl ReStation for MountUnit {
    // no input, no compensate

    // data
    fn db_map(&self, reload: bool) {
        self.mng.db_map(reload);
    }

    fn db_insert(&self) {
        self.mng.db_insert();
    }

    // reload: no external connections, entry-only
    fn entry_coldplug(&self) {
        // rebuild external connections, like: timer, ...
        // do nothing now
    }

    fn entry_clear(&self) {
        // release external connection, like: timer, ...
        // do nothing now
    }
}

impl MountUnit {
    fn new(_um: Rc<dyn UmIf>) -> MountUnit {
        let comm = Rc::new(MountUnitComm::new());
        let config = Rc::new(MountConfig::new(&comm));
        let exec_ctx = Rc::new(ExecContext::new());
        let mng = Rc::new(MountMng::new(&comm, &config, &exec_ctx));
        MountUnit {
            comm: comm.clone(),
            mng: mng.clone(),
            config: config.clone(),
            exec_ctx: exec_ctx.clone(),
        }
    }

    fn parse(&self) -> Result<()> {
        let cfg_data = self.config.config_data();
        self.exec_ctx
            .insert_envs_files(cfg_data.borrow().Mount.EnvironmentFile.clone());

        if let Some(rlimit) = cfg_data.borrow().Mount.LimitCORE {
            self.exec_ctx.insert_rlimit(libc::RLIMIT_CORE as u8, rlimit);
        }

        if let Some(rlimit) = cfg_data.borrow().Mount.LimitNOFILE {
            self.exec_ctx
                .insert_rlimit(libc::RLIMIT_NOFILE as u8, rlimit);
        }

        if let Some(rlimit) = cfg_data.borrow().Mount.LimitNPROC {
            self.exec_ctx
                .insert_rlimit(libc::RLIMIT_NPROC as u8, rlimit);
        }

        self.exec_ctx
            .set_root_directory(cfg_data.borrow().Mount.RootDirectory.clone());
        self.exec_ctx
            .set_working_directory(cfg_data.borrow().Mount.WorkingDirectory.clone());
        self.exec_ctx
            .set_runtime_directory(cfg_data.borrow().Mount.RuntimeDirectory.clone());
        self.exec_ctx
            .set_state_directory(cfg_data.borrow().Mount.StateDirectory.clone());

        self.exec_ctx
            .set_selinux_context(cfg_data.borrow().Mount.SELinuxContext.clone());

        #[cfg(feature = "linux")]
        if let Err(e) = self.exec_ctx.set_user(&cfg_data.borrow().Mount.User) {
            log::error!("Failed to set user: {}", e);
            return Err(e);
        }

        #[cfg(feature = "linux")]
        if let Err(e) = self.exec_ctx.set_group(&cfg_data.borrow().Mount.Group) {
            log::error!("Failed to set group: {}", e);
            return Err(e);
        }

        if let Err(e) = self.exec_ctx.set_umask(&cfg_data.borrow().Mount.UMask) {
            log::error!("Failed to set umask: {}", e);
            return Err(e);
        }

        Ok(())
    }
}

impl SubUnit for MountUnit {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn load(&self, paths: Vec<PathBuf>) -> Result<()> {
        let unit_name = self.comm.get_owner_id();
        self.config.load(paths, &unit_name, true);
        self.parse()?;
        Ok(())
    }

    fn current_active_state(&self) -> UnitActiveState {
        self.mng.mount_state_to_unit_state()
    }

    fn get_subunit_state(&self) -> String {
        self.mng.get_state()
    }

    fn attach_unit(&self, unit: Rc<dyn UnitBase>) {
        self.comm.attach_unit(unit);
        self.db_insert();
    }

    fn init(&self) {}

    fn done(&self) {}

    fn dump(&self) {}

    fn start(&self) -> Result<()> {
        log::info!("Mounting {}", self.comm.get_owner_id());
        let started = self.mng.start_check()?;
        if started {
            log::debug!("{} is being mounted, skipping.", self.comm.get_owner_id());
            return Ok(());
        }

        self.mng.start_action();
        self.mng.enter_mounted(true);

        Ok(())
    }

    fn stop(&self, _force: bool) -> Result<()> {
        self.mng.enter_dead(true);
        Ok(())
    }

    fn kill(&self) {}

    fn release_resources(&self) {}

    fn sigchld_events(&self, _wait_status: WaitStatus) {}

    fn reset_failed(&self) {}
}

impl UnitMngUtil for MountUnit {
    fn attach_um(&self, um: Rc<dyn UmIf>) {
        self.comm.attach_um(um);
    }

    fn attach_reli(&self, reli: Rc<Reliability>) {
        self.comm.attach_reli(reli);
    }
}

/*impl Default for MountUnit {
    fn default() -> Self {
        MountUnit::new()
    }
}*/

use core::declare_unitobj_plugin_with_param;
declare_unitobj_plugin_with_param!(MountUnit, MountUnit::new);
