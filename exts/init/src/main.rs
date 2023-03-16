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

//! Daemon sysmaster or Systemd, restart the process when it exits
mod runtime;
use crate::runtime::{param::Param, InitState, RunTime};
use nix::unistd;

fn main() {
    let cmd = get_command();

    if let Ok(res) = RunTime::new(cmd) {
        let mut run_time = res;
        if let Err(err) = run_time.init() {
            println!("Failed to init:{:?} ", err);
            run_time.clear();
            freeze();
        }

        loop {
            match run_time.get_state() {
                InitState::Reexec => {
                    if let Err(err) = run_time.reexec() {
                        println!("Failed to reexec:{:?} ", err);
                        break;
                    }
                }
                InitState::RunRecover => {
                    if let Err(err) = run_time.run() {
                        println!("Failed to run:{:?} ", err);
                        break;
                    }
                }
                InitState::RunUnRecover => {
                    if let Err(err) = run_time.unrecover_run() {
                        println!("Failed to unrecover_run:{:?} ", err);
                        break;
                    }
                }
            }
        }
        run_time.clear();
    }

    freeze();
}

fn get_command() -> Param {
    let mut param = Param::new();
    let agrs: Vec<String> = std::env::args().collect();
    // Parameter parsing starts from the second position.
    param.get_opt(agrs);

    param
}

fn freeze() {
    println!("freeze");
    unistd::sync();
    loop {
        unistd::pause();
    }
}
