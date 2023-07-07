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

//! do prepared actions for build
// if use env out_dir need add build.rs
use std::env;

const RELEASE: &str = "release";

fn main() {
    println!("cargo:rerun-if-changed=build.sh");
    println!("cargo:rerun-if-changed=build.rs");

    // turn on "debug" for non-release build
    let profile = env::var("PROFILE").unwrap();
    if profile != RELEASE {
        println!("cargo:rustc-cfg=debug");
    }
}
