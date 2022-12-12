//! do prepared actions for build
// if use env out_dir need add build.rs
use std::{env, process::Command};

macro_rules! warn {
    ($message:expr) => {
        println!("cargo:warning={}", $message);
    };
}

fn main() {
    let m_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let s_cmd = format!("{}/build.sh", m_dir);
    let out_dir = env::var("OUT_DIR").unwrap();
    let t: Vec<_> = out_dir.split("build").collect();
    println!("{:?},{:?}", s_cmd, t[0]);

    let result = Command::new(&s_cmd)
        .args(&[t[0].to_string()])
        .status()
        .unwrap();
    warn!(format!("{:?}", result));

    // warn!(format!(
    //     "{:?}",
    //     pkg_config::Config::new().probe("liblmdb").is_ok()
    // ));
    //println!("cargo:rust-flags = -C prefer-dynamic -C target-feature=-crt-static");
    //pkg_config::Config::new().probe("lmdb").unwrap();

    // println!("cargo:rustc-link-search=native=/usr/lib");
    // println!("cargo:rustc-link-lib=dylib=lmdb");
    println!("cargo:rerun-if-changed=build.sh");
    println!("cargo:rerun-if-changed=build.rs");
    // println!("cargo:rerun-if-changed=config.service");
}
