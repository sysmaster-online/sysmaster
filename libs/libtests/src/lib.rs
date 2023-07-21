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

//! This crate provides common, functions for unit tests
use std::{
    env,
    io::{self, ErrorKind},
    path::PathBuf,
};

/// get the source project root path
pub fn get_project_root() -> io::Result<PathBuf> {
    let path = env::current_dir()?;
    let mut current_path = Some(path.as_path());

    while let Some(p) = current_path {
        let has_cargo = p.read_dir()?.any(|p| {
            if let Ok(entry) = p {
                entry.file_name().eq("Cargo.lock")
            } else {
                false
            }
        });

        if has_cargo {
            return Ok(p.into());
        }

        current_path = p.parent();
    }

    Err(io::Error::new(ErrorKind::NotFound, "NotFound"))
}

/// get the crate root path
pub fn get_crate_root() -> io::Result<PathBuf> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    Ok(PathBuf::from(manifest_dir))
}

/// get the target test dir
pub fn get_target_test_dir() -> io::Result<PathBuf> {
    let test_dir = get_project_root()?.join("target").join("tests");

    if !test_dir.exists() {
        std::fs::create_dir_all(&test_dir)?;
    }

    Ok(test_dir)
}

#[cfg(test)]
mod tests {
    use crate::{get_crate_root, get_project_root, get_target_test_dir};

    #[test]
    fn test_get_project_root() {
        let mut file_path = get_project_root().unwrap();
        file_path.push("target");

        assert!(file_path.exists());
    }

    #[test]
    fn test_get_crate_root() {
        let mut file_path = get_crate_root().unwrap();
        file_path.push("Cargo.toml");

        println!("{:?}", file_path);

        assert!(file_path.is_file());
        assert!(file_path.exists());
    }

    #[test]
    fn test_get_target_test_dir() {
        let test_dir = get_target_test_dir().unwrap();
        assert!(test_dir.exists());
    }
}
