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

//! # random-seed
//!
//! random-seed.service, random_seed Load and save the system random seed at boot and shutdown

use std::{env, process};
mod random_seed;
use crate::random_seed::run;

fn main() {
    log::init_log_to_console_syslog("random-seed", log::Level::Debug);
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        log::error!("{}", "This program requires one argument.");
        process::exit(1);
    }

    unsafe {
        libc::umask(0o022);
    }

    if let Err(str) = run(&args[1]) {
        log::error!("{}", str);
        process::exit(1);
    }

    process::exit(0);
}
