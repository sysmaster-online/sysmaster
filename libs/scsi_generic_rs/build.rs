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

//! generate scsi_generic.rs
//!

use std::{env, path::PathBuf};

fn main() {
    let scsi_generic_gen = bindgen::Builder::default()
        .header("header.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate header.h");

    let path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("scsi_generic.rs");
    scsi_generic_gen
        .write_to_file(path)
        .expect("Couldn't write scsi_generic.rs!");
}
