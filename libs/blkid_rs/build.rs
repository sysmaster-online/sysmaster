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

//! generate libblkid.rs
//!

use bindgen::Builder;
use std::{env, path::PathBuf};

fn main() {
    println!("cargo:rustc-link-lib=blkid");

    let libblkid_gen = Builder::default()
        .header("header.h")
        .size_t_is_usize(true)
        .generate()
        .expect("Unable to generate libblkid");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("libblkid.rs");
    libblkid_gen
        .write_to_file(out_path)
        .expect("Couldn't write libblkid.rs");

    basic::cargo::build_libblkid();
}
