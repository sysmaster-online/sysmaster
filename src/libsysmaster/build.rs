//! 1. Generate code based on abi.proto
//! 2. Actions required for other tests
use std::{env, process::Command};

fn main() {
    let mut config = prost_build::Config::new();
    config.bytes(&["."]);
    config.type_attribute(".", "#[rustfmt::skip]");
    config
        .out_dir("src/proto")
        .compile_protos(&["abi.proto"], &["./src/proto"])
        .unwrap();
    let m_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let s_cmd = format!("{}/build_conf.sh", m_dir);

    let out_dir = env::var("OUT_DIR").unwrap();
    let t: Vec<_> = out_dir.split("build").collect();
    println!("{:?},{:?}", s_cmd, t[0]);
    let result = Command::new(&s_cmd)
        .args(&[&format!(" {}", t[0])])
        .status()
        .unwrap();
    println!("{:?}", result);
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=abi.proto");
}
