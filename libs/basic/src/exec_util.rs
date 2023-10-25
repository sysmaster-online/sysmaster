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

use nix::unistd;
use std::{fs, os::unix::prelude::PermissionsExt, path::PathBuf, process::Command};

///
pub fn execute_directories(directories: Vec<&str>) -> std::io::Result<()> {
    match unsafe { unistd::fork() } {
        Ok(unistd::ForkResult::Child) => do_execute(directories),
        Ok(unistd::ForkResult::Parent { child }) => match nix::sys::wait::waitpid(child, None) {
            Ok(_) => Ok(()),
            Err(err) => Err(std::io::Error::from(err)),
        },
        Err(err) => Err(std::io::Error::from(err)),
    }
}

///
fn do_execute(directories: Vec<&str>) -> std::io::Result<()> {
    let mut child = Vec::new();
    for generator in get_generator(directories)? {
        child.push(
            match Command::new(&generator)
                .arg("/etc/sysmaster/system")
                .spawn()
            {
                Ok(pid) => pid,
                Err(err) => {
                    log::error!("{:?} spawn err: {}", &generator, err);
                    continue;
                }
            },
        );
    }

    for mut child in child {
        match child.wait() {
            Ok(_) => continue,
            Err(err) => log::error!("wait pid err:{}", err),
        }
    }
    Ok(())
}

///
fn get_generator(directories: Vec<&str>) -> std::io::Result<Vec<PathBuf>> {
    let mut result: Vec<PathBuf> = Vec::new();

    for directory in directories {
        let dir = match fs::read_dir(directory) {
            Ok(dir) => dir,
            Err(err) => {
                log::error!("read dir {} err: {}", directory, err);
                continue;
            }
        };

        for entry in dir {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };
            if entry.file_type().unwrap().is_file()
                && entry.metadata().unwrap().permissions().mode() & 0x111 != 0
            {
                result.push(entry.path());
            }
        }
    }
    Ok(result)
}
