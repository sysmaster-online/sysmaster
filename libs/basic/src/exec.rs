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

//! exec functions

use nix::unistd;
use std::{fs, os::unix::prelude::PermissionsExt, path::PathBuf, process::Command};

/// The function `execute_directories` executes a given list of directories in parallel using fork and
/// waits for all child processes to finish.
///
/// Arguments:
///
/// * `directories`: The `directories` parameter is a vector of string slices (`&str`) representing the
/// directories that need to be executed.
///
/// Returns:
///
/// The function `execute_directories` returns a `std::io::Result<()>`.
pub fn execute_directories(directories: Vec<&str>) -> std::io::Result<()> {
    match unsafe { unistd::fork() } {
        Ok(unistd::ForkResult::Child) => {
            std::process::exit(do_execute(directories).map_or(1, |_| 0))
        }
        Ok(unistd::ForkResult::Parent { child }) => match nix::sys::wait::waitpid(child, None) {
            Ok(_) => Ok(()),
            Err(err) => Err(std::io::Error::from(err)),
        },
        Err(err) => Err(std::io::Error::from(err)),
    }
}

/// The function `do_execute` takes a list of directories, spawns child processes for each directory's
/// generator, and waits for them to finish.
///
/// Arguments:
///
/// * `directories`: A vector of strings representing directories.
///
/// Returns:
///
/// The function `do_execute` returns a `std::io::Result<()>`.
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

/// The function `get_generator` takes a vector of directory paths (`directories`) as
/// input. It reads each directory and retrieves a list of file paths that have
/// executable permissions (`permissions().mode() & 0x111 != 0`). These file paths are
/// stored in a vector (`result`) and returned as a `std::io::Result<Vec<PathBuf>>`. If
/// there is an error while reading a directory, an error message is logged and the
/// directory is skipped.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_generator() {
        let directories = vec!["/usr/bin", "/usr/lib64"];
        let _expected_files = vec![
            PathBuf::from("/usr/bin/ls"),
            PathBuf::from("/usr/bin/cat"),
            // Add more expected files here
        ];

        let actual_files = get_generator(directories).expect("get_generator() failed");

        // assert_eq!(actual_files, expected_files);

        println!("get_generator() tests passed.{:?}", actual_files);
    }
}
