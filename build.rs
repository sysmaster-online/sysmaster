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
use std::{env, fs, path::Path};

const RELEASE: &str = "release";

macro_rules! warn {
    ($message:expr) => {
        println!("cargo:warning={}", $message);
    };
}

fn copy_directory(src: &Path, dst: &Path) -> std::io::Result<()> {
    if src.is_dir() {
        if !dst.exists() {
            fs::create_dir(dst)?;
        }

        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let entry_path = entry.path();
            if let Some(entry_name) = entry_path.file_name() {
                let dst_path = dst.join(entry_name);

                if entry_path.is_dir() {
                    copy_directory(&entry_path, &dst_path)?;
                } else {
                    fs::copy(&entry_path, &dst_path)?;
                }
            }
        }
    } else {
        fs::copy(src, dst)?;
    }

    Ok(())
}

fn main() {
    // pre install git hooks
    let hooks = vec!["pre-commit", "commit-msg"];
    for hook in hooks {
        let source_file = format!("ci/{}", hook);
        let target_path = Path::new(".git/hooks/");
        if !target_path.exists() {
            let _ = std::fs::create_dir_all(target_path);
        }
        let target_file = target_path.join(hook);
        if !target_file.exists() {
            let _ = std::fs::copy(&source_file, target_path.join(hook));
        }
    }

    // copy test config
    let args: Vec<&str> = vec!["tests/presets", "tests/test_units"];
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    let out_dir = env::var("OUT_DIR").unwrap();
    let t: Vec<_> = out_dir.split("build").collect();
    let dst = t[0];

    for arg in args {
        let src = format!("{}/{}", manifest_dir, arg);
        if let Err(e) = copy_directory(Path::new(&src), Path::new(dst)) {
            warn!(format!("{:?}", e));
        }
    }

    // set rerun
    println!("cargo:rerun-if-changed=build.sh");
    println!("cargo:rerun-if-changed=build.rs");

    // turn on "debug" for non-release build
    let profile = env::var("PROFILE").unwrap();
    if profile != RELEASE {
        println!("cargo:rustc-cfg=debug");
    }
}
