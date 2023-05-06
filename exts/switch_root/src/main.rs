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

//! switch root
mod switch_root;

use nix::{errno::Errno, unistd};
use std::{env, ffi::CString, path::Path};
use switch_root::switch_root;

const SIG_SWITCH_ROOT_OFFSET: i32 = 10;

fn main() {
    let mut args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("not enough arguments");
        return;
    }

    args.remove(0);
    let newroot = args.remove(0);

    if !switch_root(&newroot) {
        eprintln!("switch_root move failed");
        return;
    }

    if args.len() <= 1 {
        call_sysmaster(args);
    } else {
        call_init(args);
    }
}

fn call_sysmaster(mut args: Vec<String>) {
    let mut pid = 1;
    if !args.is_empty() {
        pid = args.remove(0).parse().map_or(1, |pid| pid);
    }
    let res = unsafe { libc::kill(pid, libc::SIGRTMIN() + SIG_SWITCH_ROOT_OFFSET) };
    if let Err(err) = Errno::result(res).map(drop) {
        eprintln!("Failed to kill sysmaster:{err}");
    }
}

fn call_init(args: Vec<String>) {
    let init_string = args[0].clone();
    if !Path::new(&init_string).exists() {
        eprintln!("{} not exists", &init_string);
        return;
    }
    let init = CString::new(init_string).unwrap();

    let mut args_cstr = Vec::new();
    for str in args.iter() {
        args_cstr.push(std::ffi::CString::new(str.to_string()).unwrap());
    }

    let cstr_args = args_cstr
        .iter()
        .map(|cstring| cstring.as_c_str())
        .collect::<Vec<_>>();

    if let Err(e) = unistd::execv(&init, &cstr_args) {
        eprintln!("execv failed: {}", e);
    }
}
