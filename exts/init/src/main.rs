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
use std::path::Path;

const SYSMASTER_PATH: &str = "/usr/lib/sysmaster/sysmaster";
const SYSTEMD_PATH: &str = "/usr/lib/systemd/systemd";

fn main() {
    let mut cmd = get_command();
    if cmd.manager_args.is_empty() {
        let argument = detect_init();
        if argument.is_empty() {
            println!("argument is invalid!");
            freeze();
        }
        cmd.manager_args.push(argument);
    }

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

    println!("freeze");
    freeze();
}

fn get_command() -> Param {
    let mut param = Param::new();
    let mut is_manager = false;

    for arg in std::env::args() {
        if arg.contains("sysmaster") {
            is_manager = true;
        }
        if is_manager {
            param.manager_args.push(arg);
        } else {
            param.init_args.push(arg);
        }
    }
    param
}

fn detect_init() -> String {
    if Path::new(SYSMASTER_PATH).exists() {
        return String::from(SYSMASTER_PATH);
    } else if Path::new(SYSTEMD_PATH).exists() {
        return String::from(SYSTEMD_PATH);
    }

    String::new()
}

fn freeze() {
    unistd::sync();
    loop {
        unistd::pause();
    }
}
