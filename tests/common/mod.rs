use std::{env, process::Command};

pub fn run_script(suit: &str, name: &str) {
    let m_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let testpath = format!("{}/tests/{}/{}/{}.sh", m_dir, suit, name, name);
    let logpath = format!("{}/tests/{}/{}/{}.log", m_dir, suit, name, name);
    let cmd = format!("sh -x {} &> {}", testpath, logpath);
    println!("[ {} ]: {}", name, cmd);

    let status = Command::new("/bin/bash")
        .arg("-c")
        .arg(cmd)
        .status()
        .expect("failed to execute process!");

    if status.success() {
        println!("[ {} ]: {}", name, status);
    } else {
        println!("[ {} ]: {}   Detail Log:", name, status);
        let cmd = format!("cat {}", logpath);
        Command::new("/bin/bash")
            .arg("-c")
            .arg(cmd)
            .status()
            .expect("failed to cat log!");
    }

    assert!(status.success());
}
