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
//

//! utils for build.rs
const BLKID: &str = "blkid";
const BLKID_MIN_VER: &str = "2.35.2";

/// Generate cfg blkid version information
pub fn build_libblkid() {
    let libblkid = match pkg_config::Config::new()
        .atleast_version(BLKID_MIN_VER)
        .probe(BLKID)
    {
        Ok(lib) => lib,
        Err(_) => {
            println!("cargo:rustc-cfg={}=\"libblkid_2_37\"", BLKID);
            return;
        }
    };

    // Get MIN info
    let min_version = libblkid
        .version
        .split('.')
        .nth(1)
        .expect("Failed to get MIN info of version");

    if min_version
        .parse::<u32>()
        .expect("Failed to parse MIN info of version")
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
}
