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

use crate::comm::SocketUnitComm;
use core::error::*;
use core::exec::{ExecCommand, ExecContext, ExecParameters};
use nix::unistd::Pid;
use std::rc::Rc;

pub(super) struct SocketSpawn {
    comm: Rc<SocketUnitComm>,
    exec_ctx: Rc<ExecContext>,
}

impl SocketSpawn {
    pub(super) fn new(comm: &Rc<SocketUnitComm>, exec_ctx: &Rc<ExecContext>) -> SocketSpawn {
        SocketSpawn {
            comm: comm.clone(),
            exec_ctx: exec_ctx.clone(),
        }
    }

    pub(super) fn start_socket(&self, cmdline: &ExecCommand) -> Result<Pid> {
        let mut params = ExecParameters::new();

        if let Some(unit) = self.comm.owner() {
            let um = self.comm.um();
            unit.prepare_exec()?;
            match um.exec_spawn(&unit.id(), cmdline, &mut params, self.exec_ctx.clone()) {
                Ok(pid) => {
                    um.child_watch_pid(&unit.id(), pid);
                    Ok(pid)
                }
                Err(_e) => {
                    log::error!("failed to start socket: {}", unit.id());
                    Err("spawn exec return error".to_string().into())
                }
            }
        } else {
            Err("spawn exec return error".to_string().into())
        }
    }
}
