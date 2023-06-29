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

//! rc-local-generator

mod rc_local_generator;
use rc_local_generator::*;

const RC_LOCAL_PATH: &str = "/etc/rc.local";

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let args_size = args.len();
    let mut str_to = String::new();

    if 1 == args_size {
        str_to.push_str("/tmp");
    } else if 1 < args_size && 4 == args_size {
        str_to.push_str(&args[1]);
    } else {
        log::debug!("This program takes zero or three arguments.");
        return;
    }

    if check_executable(RC_LOCAL_PATH).is_ok() {
        str_to = str_to + "/" + "multi-user.target";

        let f = add_symlink("rc-local.service", &str_to);
        match f {
            Ok(()) => {}
            Err(_) => log::debug!("failed to create symlink!"),
        }
    }
}
