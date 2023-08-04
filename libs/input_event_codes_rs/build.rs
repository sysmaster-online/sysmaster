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

//! generate input_event_codes.rs and get_input_event_key.rs
//!

use std::{env, fs::write, process::Command};

fn main() {
    let input_event_codes_gen = bindgen::Builder::default()
        .header("header.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate header.h");

    let path = env::current_dir().unwrap();
    input_event_codes_gen
        .write_to_file(path.join("src/input_event_codes.rs"))
        .expect("Couldn't write input_event_codes.rs!");

    let out_path = path.to_str().unwrap();
    let s_cmd = format!("{}/src/build.sh", out_path);

    let output = Command::new(s_cmd)
        .arg(format!("{}/src", out_path))
        .output()
        .expect("Couldn't generate content of get_input_event_key.rs!");

    let contents =
        String::from_utf8(output.stdout).expect("Invalid generated get_input_event_key.rs!");
    write("src/get_input_event_key.rs", contents)
        .expect("Couldn't write to get_input_event_key.rs!");
}
