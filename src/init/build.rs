// build.rs

use std::process::Command;

fn main() {
    //默认安装 pre-commit 命令
    let _ = Command::new("pre-commit")
        .arg("--version")
        .output()
        .unwrap_or_else(|_e| {
            Command::new("pip")
                .args(["install", "--force-reinstall", "pre-commit"])
                .output()
                .unwrap();
            Command::new("pre-commit").arg("install").output().unwrap()
        });
}
