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

use std::{env, process::Command};

pub fn run_script(suit: &str, name: &str, docker_flg: &str) {
    let m_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let unit_config_test = match docker_flg {
        "1" => "",
        _ => "unit_config_test",
    };
    let base_path = format!(
        "{}/tests/{}/{}/{}/{}",
        m_dir, suit, unit_config_test, name, name
    );
    let cmd = format!(
        "BUILD_PATH={} DOCKER_TEST={} sh -x {}.sh &> {}.log",
        m_dir, docker_flg, base_path, base_path
    );
    println!("[{}]: {}", name, cmd);

    let status = Command::new("/bin/bash")
        .arg("-c")
        .arg(cmd)
        .status()
        .expect("failed to execute process!");

    if status.success() {
        println!("[{}]: {}", name, status);
    } else {
        println!("[{}]: {}   Detail Log:", name, status);
        let cmd = format!("cat {}", base_path + ".log");
        Command::new("/bin/bash")
            .arg("-c")
            .arg(cmd)
            .status()
            .expect("failed to cat log!");
    }

    assert!(status.success());
}
