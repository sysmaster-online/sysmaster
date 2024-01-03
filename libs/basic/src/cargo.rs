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

//! Cargo functions
use crate::error::*;
use std::env;

/// This function returns the path to the directory where the compiled binary is located.
/// It first tries to get the path from the OUT_DIR environment variable, and if that is not set,
/// it tries to get the path from the LD_LIBRARY_PATH environment variable.
/// If neither variable is set, it returns an error indicating that the code is not running with cargo.
pub fn env_path() -> Result<String> {
    let path = env::var("OUT_DIR")
        .or_else(|_| env::var("LD_LIBRARY_PATH"))
        .context(VarSnafu)?;

    let out_dir = path.split(':').collect::<Vec<_>>()[0]
        .split("target")
        .collect::<Vec<_>>()[0]
        .to_string();
    let tmp_str: Vec<_> = out_dir.split("build").collect();
    if tmp_str.is_empty() {
        return Err(Error::Other {
            msg: "not running with cargo".to_string(),
        });
    }

    Ok(tmp_str[0].to_string())
}

const BLKID: &str = "blkid";
const BLKID_MIN_VER: &str = "2.35.2";

/// Generate information about the version of the cfg blkid library
pub fn build_libblkid() {
    // Attempt to find the blkid library using pkg-config
    let libblkid = match pkg_config::Config::new()
        .atleast_version(BLKID_MIN_VER)
        .probe(BLKID)
    {
        Ok(lib) => lib,
        Err(_) => {
            // If the library is not found, set a flag indicating that the version is 2.37 or higher
            println!("cargo:rustc-cfg={}=\"libblkid_2_37\"", BLKID);
            return;
        }
    };

    // Extract the MIN version number from the library version
    let min_version = libblkid
        .version
        .split('.')
        .nth(1)
        .expect("Failed to extract MIN version number");

    // If the MIN version number is 37 or higher, set a flag indicating that the version is 2.37 or higher
    if min_version
        .parse::<u32>()
        .expect("Failed to parse MIN version number")
        >= 37
    {
        println!("cargo:rustc-cfg={}=\"libblkid_2_37\"", BLKID);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_libblkid() {
        build_libblkid();
    }

    #[test]
    fn env_path_test() {
        let result = env_path().unwrap();

        println!("{:?}", result);
    }
}
