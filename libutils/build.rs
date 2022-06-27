// if use env out_dir need add build.rs
use std::{env, process::Command};

fn main() {
    let m_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let s_cmd = format!("{}/build.sh",m_dir);
    let out_dir = env::var("OUT_DIR").unwrap();
    let t: Vec<_> = out_dir.split("build").collect();
    println!("{:?},{:?}",s_cmd,t[0]);

    let result = Command::new(&s_cmd).args(&[&format!(" {}",t[0])]).status().unwrap();
    println!("{:?}",result);
    println!("cargo:rerun-if-changed=build.rs");
}