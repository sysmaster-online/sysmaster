use std::{env, process::Command};

pub fn run_script(name: &str) {
    let m_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let testpath = format!("{}/tests/{}/{}.sh", m_dir, name, name);
    let logpath = format!("{}/tests/{}/{}.log", m_dir, name, name);
    let cmd = format!("sh -x {} &> {}", testpath, logpath);
    println!("[ {} ]: {}", name, cmd);

    let status = Command::new("/bin/bash")
                         .arg("-c")
                         .arg(cmd)
                         .status()
                         .expect("failed to execute process!");
    println!("[ {} ]: {}", name, status);
    assert!(status.success());
}