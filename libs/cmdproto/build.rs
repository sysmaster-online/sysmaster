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

//! 1. Generate code based on abi.proto
//! 2. Actions required for other tests
// use std::{env, process::Command};

fn main() {
    // when update proto/abi.proto, can use the fowllow code to gengerate abi.rs
    // let mut config = prost_build::Config::new();
    // config.bytes(["."]);
    // config.type_attribute(".", "#[rustfmt::skip]");
    // config
    //     .out_dir("src/proto")
    //     .compile_protos(&["abi.proto"], &["./src/proto"])
    //     .unwrap();
    // println!("cargo:rerun-if-changed=build.rs");
    // println!("cargo:rerun-if-changed=./src/proto/abi.proto");
}
