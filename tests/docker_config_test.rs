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

mod common;

#[test]
#[ignore]
fn docker_config_test_dependency_001() {
    common::run_script(
        "docker_config_test",
        "docker_config_test_dependency_001",
        "1",
    );
}

#[test]
#[ignore]
fn docker_config_test_dependency_002() {
    common::run_script(
        "docker_config_test",
        "docker_config_test_dependency_002",
        "1",
    );
}

#[test]
#[ignore]
fn docker_config_test_dependency_003() {
    common::run_script(
        "docker_config_test",
        "docker_config_test_dependency_003",
        "1",
    );
}

#[test]
#[ignore]
fn docker_config_test_dependency_004() {
    common::run_script(
        "docker_config_test",
        "docker_config_test_dependency_004",
        "1",
    );
}

#[test]
#[ignore]
fn docker_config_test_seq_001() {
    common::run_script("docker_config_test", "docker_config_test_seq_001", "1");
}

#[test]
#[ignore]
fn docker_config_test_condition_001() {
    common::run_script(
        "docker_config_test",
        "docker_config_test_condition_001",
        "1",
    );
}

#[test]
#[ignore]
fn docker_config_test_condition_002() {
    common::run_script(
        "docker_config_test",
        "docker_config_test_condition_002",
        "1",
    );
}

#[test]
#[ignore]
fn docker_config_test_assert_001() {
    common::run_script("docker_config_test", "docker_config_test_assert_001", "1");
}

#[test]
#[ignore]
fn docker_config_test_startlimit_001() {
    common::run_script(
        "docker_config_test",
        "docker_config_test_startlimit_001",
        "1",
    );
}

#[test]
#[ignore]
fn docker_config_test_exec_001() {
    common::run_script("docker_config_test", "docker_config_test_exec_001", "1");
}

#[test]
#[ignore]
fn docker_config_test_action_001() {
    common::run_script("docker_config_test", "docker_config_test_action_001", "1");
}

#[test]
#[ignore]
fn docker_config_test_timeout_001() {
    common::run_script("docker_config_test", "docker_config_test_timeout_001", "1");
}
