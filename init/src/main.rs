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
    let cmd = match get_command() {
        Some(c) => c,
        None => return,
    };

    // The main role of init is to manage sysmaster and recycle zombie processes.
    // Reexec: Reexecute or connect the sysmaster.
    // Run: Monitor the sysmaster's liveliness and acceptance of message.
    // Unrecover: On-site problem collection or recreate new sysmaster.
    match RunTime::new(cmd) {
        Ok(mut run_time) => loop {
            let state = run_time.state();
            match state {
                InitState::Reexec => run_time.reexec(),
                InitState::Run => run_time.run(),
                InitState::Unrecover => run_time.unrecover(),
            }
        },
        Err(err) => eprintln!(
            "Failed to new init, it may be necessary to run it as root :{:?}",
            err
        ),
    }

    // freeze, after RunTime::new fails or signal_fd or timer_fd generates epoll error.
    freeze();
}

fn get_command() -> Option<Param> {
    let args: Vec<String> = std::env::args().collect();
    if args.contains(&String::from("--version")) || args.contains(&String::from("-V")) {
        let version = env!("CARGO_PKG_VERSION");
        println!("sysMaster init version: {}", version);
        None
    } else {
        let mut param = Param::new();
        param.get_opt(args);
        Some(param)
    }
}

fn freeze() {
    println!("freeze");
    unistd::sync();
    loop {
        unistd::pause();
    }
}
