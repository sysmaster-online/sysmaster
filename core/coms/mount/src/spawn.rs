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
//

use core::error::*;
use core::exec::{ExecCommand, ExecContext, ExecParameters};
use std::rc::Rc;

use nix::unistd::Pid;

use crate::comm::MountUnitComm;

pub(crate) struct MountSpawn {
    comm: Rc<MountUnitComm>,
    exec_ctx: Rc<ExecContext>,
}

impl MountSpawn {
    pub(super) fn new(comm: &Rc<MountUnitComm>, exec_ctx: &Rc<ExecContext>) -> MountSpawn {
        MountSpawn {
            comm: comm.clone(),
            exec_ctx: exec_ctx.clone(),
        }
    }

    pub(super) fn spawn_cmd(&self, cmdline: &ExecCommand) -> Result<Pid> {
        let mut params = ExecParameters::new();

        if let Some(unit) = self.comm.owner() {
            let um = self.comm.um();
            unit.prepare_exec()?;
            match um.exec_spawn(&unit.id(), cmdline, &mut params, self.exec_ctx.clone()) {
                Ok(pid) => {
                    um.child_watch_pid(&unit.id(), pid);
                    Ok(pid)
                }
                Err(e) => {
                    log::error!("Failed to spawn the mount command of {}: {}", unit.id(), e);
                    Err("spawn mount command error".to_string().into())
                }
            }
        } else {
            Err("spawn mount command error".to_string().into())
        }
    }
}
