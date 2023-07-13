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

use std::{fs, io, os::linux::fs::MetadataExt};

const SYSTEM_DATA_UNIT_DIR: &str = "/usr/lib/sysmaster/";
const B_EXEC: u32 = 0o100;

fn mkdir_parents_lable(path: &str) -> io::Result<()> {
    if path.is_empty() {
        return Result::Err(io::ErrorKind::NotFound.into());
    }

    let size = path.rfind('/').unwrap_or(0);

    if 0 == size {
        return Ok(());
    }
    fs::create_dir_all(&path[..size])?;

    Ok(())
}

pub(crate) fn add_symlink(from_service: &str, to_where: &str) -> io::Result<()> {
    if from_service.is_empty() || to_where.is_empty() {
        return Err(io::ErrorKind::NotFound.into());
    }

    let from = SYSTEM_DATA_UNIT_DIR.to_string() + "/" + from_service;
    let to = to_where.to_string() + ".wants/" + from_service;

    let _ = mkdir_parents_lable(&to);

    let e = std::os::unix::fs::symlink(from, &to);
    match e {
        Err(a) => {
            if a.kind() == io::ErrorKind::AlreadyExists {
                log::debug!("symlink already exists");
                return Ok(());
            }

            log::debug!("Failed to create symlink {}", to);
            Err(a)
        }
        Ok(_) => Ok(()),
    }
}

pub(crate) fn check_executable(file: &str) -> io::Result<()> {
    match fs::metadata(file) {
        Err(err) => {
            if err.kind() == io::ErrorKind::NotFound {
                log::debug!("{} does not exist, skipping", file);
                return Err(err);
            }

            log::debug!(
                "Couldn't determine if {} exists and is executable, skipping",
                file
            );
            return Err(err);
        }
        Ok(a) => {
            let mode = a.st_mode();
            if 0 == B_EXEC & mode {
                log::debug!("{} is not marked executable, skipping", file);
                return Err(io::ErrorKind::PermissionDenied.into());
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    #[test]
    fn mkdir_parents_lable_test() {
        let path = "/tmp/a/";
        mkdir_parents_lable(path).unwrap();

        let path = "/tmp/a/";
        mkdir_parents_lable(path).unwrap();

        let path = "/tmp/a/b/b/";
        mkdir_parents_lable(path).unwrap();

        let path = "/tmp/a/c";
        mkdir_parents_lable(path).unwrap();

        let path = "/tmp/a////d/e//";
        mkdir_parents_lable(path).unwrap();
    }

    #[test]
    #[should_panic]
    fn mkdir_empty_parents_lable_test() {
        let path = "";
        mkdir_parents_lable(path).unwrap();
    }

    #[test]
    fn add_symlink_test() {
        let str_from = "/tmp".to_string() + "/" + "multi-user.target";
        add_symlink("rc-local.service", &str_from).unwrap();
    }

    #[test]
    #[should_panic]
    fn add_empty_symlink_test() {
        add_symlink("", "").unwrap();
        /*
        let from_service = "/tmp";
        add_symlink(from_service, "").unwrap();

        let to_where = "/tmp";
        add_symlink("", to_where).unwrap();

        let from_service = "no_exit_file";
        let to_where = "/tmp/tolink";
        add_symlink(from_service, to_where).unwrap();
        */
    }

    #[test]
    fn check_executable_test() {
        let _file = fs::File::create("test.exec").unwrap();
        unsafe {
            let exec_name = CString::new("test.exec").unwrap();
            libc::chmod(exec_name.as_ptr(), 0o777);
        }
        assert!(check_executable("test.exec").map_or(false, |_| { true }));
        unsafe {
            let exec_name = CString::new("test.exec").unwrap();
            libc::chmod(exec_name.as_ptr(), 0o644);
        }
        assert!(!check_executable("test.exec").map_or(false, |_| { true }));
        fs::remove_file("test.exec").unwrap();
    }
}
