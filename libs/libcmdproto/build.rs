//! 1. Generate code based on abi.proto
//! 2. Actions required for other tests
// use std::{env, process::Command};

fn main() {
    let mut config = prost_build::Config::new();
    config.bytes(["."]);
    config.type_attribute(".", "#[rustfmt::skip]");
    config
        .out_dir("src/proto")
        .compile_protos(&["abi.proto"], &["./src/proto"])
        .unwrap();
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=abi.proto");
}
