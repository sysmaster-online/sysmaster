use std::env;
use std::process::Command;

fn main() {
    let mut config = prost_build::Config::new();
    config.bytes(&["."]);
    // config.type_attribute(".", "#[derive(PartialOrd)]");
    config.type_attribute(".", "#[rustfmt::skip]");
    config
        .out_dir("src/proto")
        .compile_protos(&["abi.proto"], &["./src/proto"])
        .unwrap();
    let m_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let s_cmd = format!("{}/build.sh",m_dir);
    let out_dir = env::var("OUT_DIR").unwrap();
    let result = Command::new(&s_cmd).args(&[&format!(" {}",out_dir)]).status().unwrap();
    println!("{:?}",result);
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=abi.proto");
}
