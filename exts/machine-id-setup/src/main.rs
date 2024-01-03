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

//!

use basic::{
    id128,
    machine::{self, machine_id_setup},
};
use nix::{Error, Result};
use std::path::Path;

fn check_args(args: &[String]) -> bool {
    if args.len() == 1 {
        return true;
    }
    let mut i = 1;
    while i < args.len() {
        i += 1;
        match &args[i] as &str {
            "--commit" => continue,
            "--print" => continue,
            _ => return false,
        }
    }
    true
}

fn main() -> Result<()> {
    log::init_log_to_console("machine-id-setup", log::Level::Info);
    let args: Vec<String> = std::env::args().collect();
    let id: String;

    if !check_args(&args) {
        log::error!("Invalid args. Support [--commit][--print]");
        return Err(nix::Error::EINVAL);
    }

    if args.contains(&String::from("--commit")) {
        let etc_machine_id = "/etc/machine-id".to_string();
        machine::machine_id_commit()?;
        id = match id128::id128_read_by_path(
            Path::new(&etc_machine_id),
            id128::Id128FormatFlag::ID128_FORMAT_PLAIN,
        ) {
            Ok(id) => id,
            Err(e) => {
                log::error!("Failed to read machine Id back: {}", e);
                return Err(Error::EIO);
            }
        }
    } else {
        id = match machine_id_setup(false, "") {
            Ok(id) => id,
            Err(e) => {
                log::error!("Failed to setup machine-id:{}", e);
                return Err(e);
            }
        }
    }

    if args.contains(&String::from("--print")) {
        println!("{}", id);
    }
    Ok(())
}
