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
use std::{
    fs::OpenOptions,
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
};

fn main() {
    println!("cargo:rustc-link-lib=blkid");

    let libblkid_gen = Builder::default()
        .header("header.h")
        .size_t_is_usize(true)
        .generate()
        .expect("Unable to generate libblkid");

    let out_path = PathBuf::from("./src/libblkid.rs");
    libblkid_gen
        .write_to_file(out_path.clone())
        .expect("Couldn't write libblkid_gen");

    let mut file = match OpenOptions::new().read(true).write(true).open(out_path) {
        Ok(f) => f,
        Err(e) => {
            println!("open libblkid.rs err: {}", e);
            return;
        }
    };
    let mut buf: Vec<u8> = Vec::new();
    file.read_to_end(&mut buf)
        .expect("read whole file of libblkid.rs failed");
    file.seek(SeekFrom::Start(0)).expect("seek start failed");
    file.write_all(
        b"#![allow\
(non_camel_case_types)]
#![allow\
(non_upper_case_globals)]
#![allow\
(non_snake_case)]
#![allow\
(deref_nullptr)]
#![allow\
(unused)]\n",
    )
    .expect("weite all failed");

    file.write_all(&buf).expect("write all failed");
}
